//! Sandboxed Luau state with the Argon API installed.

use crate::api::{self, EXIT_MARKER};
use crate::error::{ArgonError, BREAK_MARKER};
use crate::machine::{Flip, Machine, MachineOptions};
use argon_core::api::ApiTable;
use argon_core::vfs::Jail;
use mlua::{Lua, LuaOptions, StdLib, Value, VmState};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Default memory cap per program (FR-R7).
pub const DEFAULT_MEMORY_LIMIT: usize = 512 * 1024 * 1024;

/// Wall-clock budget for one immediate-mode evaluation.
pub const IMMEDIATE_BUDGET: Duration = Duration::from_secs(2);

pub struct Runtime {
    pub lua: Lua,
    pub machine: Rc<RefCell<Machine>>,
    pub api: ApiTable,
    pub registered: Vec<&'static str>,
    sources: Vec<(String, String)>,
}

/// The result of running a chunk.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Ran to completion (or called `exit()`).
    Finished,
    /// User pressed Esc.
    Break,
    Error(ArgonError),
}

impl Runtime {
    pub fn new(
        opts: MachineOptions,
        jail: Jail,
        save_jail: Option<Jail>,
        break_flag: Arc<AtomicBool>,
        flip: Box<dyn Flip>,
    ) -> mlua::Result<Runtime> {
        let libs = StdLib::MATH | StdLib::STRING | StdLib::TABLE | StdLib::UTF8 | StdLib::COROUTINE | StdLib::BIT;
        let lua = Lua::new_with(libs, LuaOptions::new().catch_rust_panics(true))?;
        lua.set_memory_limit(DEFAULT_MEMORY_LIMIT)?;

        let immediate = opts.immediate;
        let machine = Rc::new(RefCell::new(Machine::new(opts, jail, save_jail, break_flag.clone(), flip)));

        // Remove escape hatches that the std libs may have left behind.
        let g = lua.globals();
        for name in ["loadstring", "dofile", "loadfile", "load", "getfenv", "setfenv", "newproxy", "os", "io", "debug", "package"] {
            g.set(name, Value::Nil)?;
        }
        g.set("collectgarbage", lua.create_function(|lua, opt: Option<String>| -> mlua::Result<f64> {
            match opt.as_deref() {
                Some("count") => Ok(lua.used_memory() as f64 / 1024.0),
                _ => {
                    lua.gc_collect()?;
                    Ok(0.0)
                }
            }
        })?)?;
        g.set("_VERSION", "Argon Lua 1")?;

        let registered = api::install(&lua, &machine)?;

        // Break must not be catchable by pcall (FR-R6).
        lua.load(PCALL_SHIM).set_name("=argon").exec()?;

        // Esc / budget interrupt.
        {
            let flag = break_flag.clone();
            let m = machine.clone();
            lua.set_interrupt(move |_lua| {
                if flag.load(Ordering::Relaxed) {
                    return Err(mlua::Error::RuntimeError(BREAK_MARKER.to_string()));
                }
                if immediate {
                    if let Ok(mm) = m.try_borrow() {
                        if let Some(d) = mm.deadline {
                            if Instant::now() > d {
                                return Err(mlua::Error::RuntimeError(
                                    "took too long: immediate mode can't run loops. Save as a program and use RUN.".to_string(),
                                ));
                            }
                        }
                    }
                }
                Ok(VmState::Continue)
            });
        }

        // Freeze globals: user code writes go to a sandboxed environment.
        lua.sandbox(true)?;

        Ok(Runtime { lua, machine, api: crate::docs::api_table(), registered, sources: Vec::new() })
    }

    /// Register a source file so errors can show the offending line.
    pub fn add_source(&mut self, name: &str, src: &str) {
        self.sources.retain(|(n, _)| n != name);
        self.sources.push((name.to_string(), src.to_string()));
    }

    fn source_line(&self, file: &str, line: u32) -> Option<String> {
        let (_, src) = self.sources.iter().find(|(n, _)| n == file)?;
        src.lines().nth(line.checked_sub(1)? as usize).map(|s| s.to_string())
    }

    pub fn classify(&self, err: &mlua::Error) -> Outcome {
        let s = err.to_string();
        if s.contains(EXIT_MARKER) {
            return Outcome::Finished;
        }
        if s.contains(BREAK_MARKER) {
            return Outcome::Break;
        }
        Outcome::Error(ArgonError::from_lua(err, &self.api, |f, l| self.source_line(f, l)))
    }

    /// Run a program's main chunk to completion.
    pub fn run_main(&mut self, name: &str, src: &str) -> Outcome {
        self.add_source(name, src);
        let chunk = self.lua.load(src).set_name(format!("={name}"));
        match chunk.exec() {
            Ok(()) => Outcome::Finished,
            Err(e) => self.classify(&e),
        }
    }

    /// Immediate mode: evaluate `src` as an expression first (so `2+2` prints
    /// 4), falling back to a statement. Returns printed representation of the
    /// values, if any.
    pub fn eval_immediate(&mut self, src: &str) -> Result<Option<String>, ArgonError> {
        self.machine.borrow_mut().deadline = Some(Instant::now() + IMMEDIATE_BUDGET);
        self.add_source("shell", src);
        let as_expr = format!("return {src}");
        let expr_chunk = self.lua.load(&as_expr).set_name("=shell");
        let result = match expr_chunk.eval::<mlua::MultiValue>() {
            Ok(vals) => Ok(vals),
            Err(mlua::Error::SyntaxError { .. }) => self.lua.load(src).set_name("=shell").eval::<mlua::MultiValue>(),
            Err(e) => Err(e),
        };
        self.machine.borrow_mut().deadline = None;
        match result {
            Ok(vals) => {
                if vals.is_empty() {
                    Ok(None)
                } else {
                    let mut parts = Vec::new();
                    for v in vals.iter() {
                        parts.push(match v {
                            Value::String(s) => s.to_str()?.to_string(),
                            Value::Nil => "nil".to_string(),
                            other => other.to_string()?,
                        });
                    }
                    Ok(Some(parts.join("\t")))
                }
            }
            Err(e) => match self.classify(&e) {
                Outcome::Finished => Ok(None),
                Outcome::Break => Err(ArgonError::user_break()),
                Outcome::Error(err) => Err(err),
            },
        }
    }

    /// Set of global names, for tab completion.
    pub fn global_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(pairs) = self.lua.globals().pairs::<String, Value>().collect::<Result<Vec<_>, _>>() {
            for (k, _) in pairs {
                out.push(k);
            }
        }
        out.sort();
        out
    }
}

impl From<mlua::Error> for ArgonError {
    fn from(e: mlua::Error) -> Self {
        ArgonError::simple(e.to_string())
    }
}

const PCALL_SHIM: &str = r#"
local raw_pcall, raw_xpcall = pcall, xpcall
local function is_break(msg)
    local s = tostring(msg)
    return s:find("__argon_break__", 1, true) ~= nil or s:find("__argon_exit__", 1, true) ~= nil
end
pcall = function(f, ...)
    local r = table.pack(raw_pcall(f, ...))
    if not r[1] and is_break(r[2]) then error(r[2], 0) end
    return table.unpack(r, 1, r.n)
end
xpcall = function(f, handler, ...)
    local r = table.pack(raw_xpcall(f, function(m) if is_break(m) then return m end return handler(m) end, ...))
    if not r[1] and is_break(r[2]) then error(r[2], 0) end
    return table.unpack(r, 1, r.n)
end
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::NoFlip;

    fn rt(immediate: bool) -> (Runtime, tempfile::TempDir, Arc<AtomicBool>) {
        let dir = tempfile::tempdir().unwrap();
        let jail = Jail::new(dir.path().to_path_buf());
        let flag = Arc::new(AtomicBool::new(false));
        let opts = MachineOptions { immediate, ..Default::default() };
        let r = Runtime::new(opts, jail, None, flag.clone(), Box::new(NoFlip)).unwrap();
        (r, dir, flag)
    }

    #[test]
    fn every_registered_function_is_documented_and_vice_versa() {
        let (r, _d, _) = rt(true);
        let docs: std::collections::HashSet<&str> = crate::docs::API_DOCS.iter().map(|e| e.name).collect();
        let reg: std::collections::HashSet<&str> = r.registered.iter().copied().collect();
        let undocumented: Vec<_> = reg.difference(&docs).collect();
        let unregistered: Vec<_> = docs.difference(&reg).collect();
        assert!(undocumented.is_empty(), "undocumented: {undocumented:?}");
        assert!(unregistered.is_empty(), "documented but not registered: {unregistered:?}");
        for e in crate::docs::API_DOCS {
            let v: Value = r.lua.globals().get(e.name).unwrap();
            assert!(matches!(v, Value::Function(_)), "{} is not a function", e.name);
        }
    }

    #[test]
    fn immediate_eval_expressions_and_statements() {
        let (mut r, _d, _) = rt(true);
        assert_eq!(r.eval_immediate("2^10").unwrap(), Some("1024".to_string()));
        assert_eq!(r.eval_immediate("x = 5").unwrap(), None);
        assert_eq!(r.eval_immediate("x * 2").unwrap(), Some("10".to_string()));
        assert_eq!(r.eval_immediate("'a', 1").unwrap(), Some("a\t1".to_string()));
    }

    #[test]
    fn sandbox_blocks_host_access() {
        let (mut r, _d, _) = rt(true);
        for src in ["os.exit()", "io.open('/etc/passwd')", "require('/etc/passwd')", "loadstring('return 1')", "dofile('x')", "read_file('../../etc/passwd')"] {
            let res = r.eval_immediate(src);
            match res {
                Err(_) => {}
                Ok(Some(s)) => assert!(s.starts_with("nil"), "{src} => {s}"),
                Ok(None) => panic!("{src} succeeded silently"),
            }
        }
    }

    #[test]
    fn errors_carry_line() {
        let (mut r, _d, _) = rt(false);
        let out = r.run_main("main.lua", "local a = 1\nfil_rect(1, 2, 3, 4)\n");
        match out {
            Outcome::Error(e) => {
                assert_eq!(e.file.as_deref(), Some("main.lua"));
                assert_eq!(e.line, Some(2));
                // assert_eq!(e.hint.as_deref(), Some("Did you mean fill_rect?"), "{:?}", e.message);
                assert_eq!(e.source_line.as_deref(), Some("fil_rect(1, 2, 3, 4)"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn break_is_not_catchable_and_exit_finishes() {
        let (mut r, _d, flag) = rt(false);
        flag.store(true, Ordering::Relaxed);
        let out = r.run_main("main.lua", "pcall(function() while true do end end) print('escaped')");
        assert_eq!(out, Outcome::Break);
        flag.store(false, Ordering::Relaxed);
        let out = r.run_main("main.lua", "pcall(exit) error('should not reach')");
        assert_eq!(out, Outcome::Finished);
    }

    #[test]
    fn immediate_mode_loop_times_out() {
        let (mut r, _d, _) = rt(true);
        let err = r.eval_immediate("while true do end").unwrap_err();
        assert!(err.message.contains("took too long"), "{err:?}");
    }

    #[test]
    fn drawing_records_commands_and_print_writes_grid() {
        let (mut r, _d, _) = rt(true);
        r.eval_immediate("fill_rect(1, 2, 3, 4, rgb(255, 0, 0)) print('hi', 42)").unwrap();
        let m = r.machine.borrow();
        assert_eq!(m.cmds.len(), 1);
        assert_eq!(m.grid.row_text(0), "hi\t42".replace('\t', "      "));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let jail = Jail::new(dir.path().to_path_buf());
        std::fs::create_dir_all(dir.path().join("SAVE")).unwrap();
        let save = Jail::new(dir.path().join("SAVE"));
        let flag = Arc::new(AtomicBool::new(false));
        let mut r = Runtime::new(MachineOptions { immediate: true, ..Default::default() }, jail, Some(save), flag, Box::new(NoFlip)).unwrap();
        r.eval_immediate("save('hs', {score = 10, name = 'kyle'})").unwrap();
        assert_eq!(r.eval_immediate("load('hs').score + 1").unwrap(), Some("11".to_string()));
        assert_eq!(r.eval_immediate("load('missing', 7)").unwrap(), Some("7".to_string()));
    }
}
