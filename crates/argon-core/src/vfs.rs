//! The virtual disk: a plain host directory with a path jail (FR-F1, FR-F2).
//!
//! All program and shell file access goes through [`Jail::resolve`]. A jail
//! has a root; paths are virtual, use `/`, may not be absolute and may not
//! contain `..`. Symlinks that escape the root are rejected.

use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VfsError {
    #[error("invalid path '{0}'")]
    InvalidPath(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("not a file: {0}")]
    NotAFile(String),
    #[error("not a directory: {0}")]
    NotADirectory(String),
    #[error("{0} is read-only")]
    ReadOnly(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VfsError>;

/// A directory listing entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    /// Folder containing `cart.toml` (or `main.lua`).
    pub is_cart: bool,
}

#[derive(Clone, Debug)]
pub struct Jail {
    root: PathBuf,
    /// Virtual path prefixes that are read-only (e.g. "DEMOS").
    read_only: Vec<String>,
}

/// Normalize a virtual path into clean components. Rejects `..`, absolute
/// paths, NUL and empty components.
pub fn normalize(path: &str) -> Result<Vec<String>> {
    let p = path.replace('\\', "/");
    if p.contains('\0') {
        return Err(VfsError::InvalidPath(path.to_string()));
    }
    let mut out = Vec::new();
    for comp in p.split('/') {
        match comp {
            "" | "." => continue,
            ".." => return Err(VfsError::InvalidPath(path.to_string())),
            c if c.ends_with(' ') || c.ends_with('.') => return Err(VfsError::InvalidPath(path.to_string())),
            c => out.push(c.to_string()),
        }
    }
    Ok(out)
}

impl Jail {
    pub fn new(root: impl Into<PathBuf>) -> Jail {
        Jail { root: root.into(), read_only: Vec::new() }
    }

    pub fn with_read_only(mut self, prefixes: &[&str]) -> Jail {
        self.read_only = prefixes.iter().map(|s| s.to_ascii_uppercase()).collect();
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// A jail rooted at a sub directory of this one.
    pub fn sub(&self, vpath: &str) -> Result<Jail> {
        let host = self.resolve(vpath)?;
        Ok(Jail::new(host))
    }

    fn is_read_only(&self, comps: &[String]) -> bool {
        comps
            .first()
            .map(|c| self.read_only.iter().any(|ro| ro == &c.to_ascii_uppercase()))
            .unwrap_or(false)
    }

    /// Map a virtual path to a host path, verifying it stays inside the root
    /// even after resolving symlinks.
    pub fn resolve(&self, vpath: &str) -> Result<PathBuf> {
        let comps = normalize(vpath)?;
        let mut host = self.root.clone();
        for c in &comps {
            host.push(c);
        }
        let root_canon = fs::canonicalize(&self.root)?;
        // Canonicalize the deepest existing ancestor and make sure it is
        // inside the root. The non-existing tail cannot contain `..`.
        let mut probe = host.clone();
        let mut tail = Vec::new();
        while !probe.exists() {
            match (probe.file_name(), probe.parent()) {
                (Some(name), Some(parent)) => {
                    tail.push(name.to_owned());
                    probe = parent.to_path_buf();
                }
                _ => return Err(VfsError::InvalidPath(vpath.to_string())),
            }
        }
        let mut canon = fs::canonicalize(&probe)?;
        if !canon.starts_with(&root_canon) {
            return Err(VfsError::InvalidPath(vpath.to_string()));
        }
        for t in tail.iter().rev() {
            canon.push(t);
        }
        Ok(canon)
    }

    fn resolve_writable(&self, vpath: &str) -> Result<PathBuf> {
        let comps = normalize(vpath)?;
        if self.is_read_only(&comps) {
            return Err(VfsError::ReadOnly(vpath.to_string()));
        }
        self.resolve(vpath)
    }

    pub fn exists(&self, vpath: &str) -> bool {
        self.resolve(vpath).map(|p| p.exists()).unwrap_or(false)
    }

    pub fn is_dir(&self, vpath: &str) -> bool {
        self.resolve(vpath).map(|p| p.is_dir()).unwrap_or(false)
    }

    pub fn read(&self, vpath: &str) -> Result<Vec<u8>> {
        let p = self.resolve(vpath)?;
        if !p.exists() {
            return Err(VfsError::NotFound(vpath.to_string()));
        }
        if !p.is_file() {
            return Err(VfsError::NotAFile(vpath.to_string()));
        }
        Ok(fs::read(p)?)
    }

    pub fn read_string(&self, vpath: &str) -> Result<String> {
        Ok(String::from_utf8_lossy(&self.read(vpath)?).into_owned())
    }

    pub fn write(&self, vpath: &str, data: &[u8]) -> Result<()> {
        let p = self.resolve_writable(vpath)?;
        if p.is_dir() {
            return Err(VfsError::NotAFile(vpath.to_string()));
        }
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(p, data)?;
        Ok(())
    }

    pub fn append(&self, vpath: &str, data: &[u8]) -> Result<()> {
        use std::io::Write;
        let p = self.resolve_writable(vpath)?;
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = fs::OpenOptions::new().create(true).append(true).open(p)?;
        f.write_all(data)?;
        Ok(())
    }

    pub fn delete(&self, vpath: &str) -> Result<()> {
        let p = self.resolve_writable(vpath)?;
        if !p.exists() {
            return Err(VfsError::NotFound(vpath.to_string()));
        }
        if p.is_dir() {
            fs::remove_dir_all(p)?;
        } else {
            fs::remove_file(p)?;
        }
        Ok(())
    }

    pub fn mkdir(&self, vpath: &str) -> Result<()> {
        let p = self.resolve_writable(vpath)?;
        if p.exists() {
            return Err(VfsError::AlreadyExists(vpath.to_string()));
        }
        fs::create_dir_all(p)?;
        Ok(())
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<()> {
        let a = self.resolve_writable(from)?;
        let b = self.resolve_writable(to)?;
        if !a.exists() {
            return Err(VfsError::NotFound(from.to_string()));
        }
        if b.exists() {
            return Err(VfsError::AlreadyExists(to.to_string()));
        }
        fs::rename(a, b)?;
        Ok(())
    }

    pub fn copy(&self, from: &str, to: &str) -> Result<()> {
        let a = self.resolve(from)?;
        let b = self.resolve_writable(to)?;
        if !a.exists() {
            return Err(VfsError::NotFound(from.to_string()));
        }
        if b.exists() {
            return Err(VfsError::AlreadyExists(to.to_string()));
        }
        if a.is_dir() {
            copy_dir(&a, &b)?;
        } else {
            if let Some(parent) = b.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(a, b)?;
        }
        Ok(())
    }

    pub fn list(&self, vpath: &str) -> Result<Vec<DirEntry>> {
        let p = self.resolve(vpath)?;
        if !p.is_dir() {
            return Err(VfsError::NotADirectory(vpath.to_string()));
        }
        let mut out = Vec::new();
        for e in fs::read_dir(p)? {
            let e = e?;
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let meta = e.metadata()?;
            let is_dir = meta.is_dir();
            let is_cart = is_dir && (e.path().join("cart.toml").exists() || e.path().join("main.lua").exists());
            out.push(DirEntry { name, is_dir, size: if is_dir { 0 } else { meta.len() }, is_cart });
        }
        out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        Ok(out)
    }

    /// Find an entry in `dir` by name, case-insensitively, preferring exact.
    pub fn find_entry(&self, dir: &str, name: &str) -> Option<DirEntry> {
        let entries = self.list(dir).ok()?;
        entries
            .iter()
            .find(|e| e.name == name)
            .or_else(|| entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)))
            .cloned()
    }

    /// Resolve a virtual path component by component, case-insensitively,
    /// returning the path with the on-disk spelling. `None` if any component
    /// is missing.
    pub fn canonical_vpath(&self, vpath: &str) -> Option<String> {
        let comps = normalize(vpath).ok()?;
        let mut cur = String::new();
        for c in comps {
            let e = self.find_entry(&cur, &c)?;
            cur = join(&cur, &e.name);
        }
        Some(cur)
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let dest = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &dest)?;
        } else {
            fs::copy(e.path(), dest)?;
        }
    }
    Ok(())
}

/// Join virtual path components.
pub fn join(dir: &str, name: &str) -> String {
    let dir = dir.trim_matches('/');
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// Parent of a virtual path ("" for root).
pub fn parent(vpath: &str) -> String {
    let p = vpath.trim_matches('/');
    match p.rfind('/') {
        Some(i) => p[..i].to_string(),
        None => String::new(),
    }
}

pub fn file_name(vpath: &str) -> &str {
    vpath.trim_matches('/').rsplit('/').next().unwrap_or("")
}

pub fn strip_lua_ext(name: &str) -> &str {
    name.strip_suffix(".lua").unwrap_or(name)
}

/// Where a program lives once resolved by `RUN`/`EDIT`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramLocation {
    /// Virtual path of the main Lua file.
    pub main: String,
    /// Virtual path of the folder that is the program's jail (the cart folder
    /// or the containing directory for single files).
    pub dir: String,
    /// Virtual path of `cart.toml`, if a cart.
    pub manifest: Option<String>,
    pub is_cart: bool,
    /// Display name (file stem or cart folder name).
    pub name: String,
}

/// Resolve `RUN name` semantics: `name`, `name.lua`, `name/main.lua`.
pub fn resolve_program(jail: &Jail, cwd: &str, name: &str) -> Option<ProgramLocation> {
    let name = name.trim().trim_end_matches('/');
    if name.is_empty() {
        return None;
    }
    let (dir, base) = match name.rfind('/') {
        Some(i) => (jail.canonical_vpath(&join(cwd, &name[..i]))?, &name[i + 1..]),
        None => (cwd.to_string(), name),
    };
    let candidates = [base.to_string(), format!("{base}.lua")];
    for cand in &candidates {
        if let Some(entry) = jail.find_entry(&dir, cand) {
            let vpath = join(&dir, &entry.name);
            if entry.is_dir {
                let main = join(&vpath, "main.lua");
                if jail.exists(&main) {
                    let manifest = join(&vpath, "cart.toml");
                    return Some(ProgramLocation {
                        main,
                        dir: vpath.clone(),
                        manifest: jail.exists(&manifest).then_some(manifest),
                        is_cart: true,
                        name: entry.name.clone(),
                    });
                }
            } else if entry.name.to_lowercase().ends_with(".lua") {
                return Some(ProgramLocation {
                    main: vpath,
                    dir: dir.clone(),
                    manifest: None,
                    is_cart: false,
                    name: strip_lua_ext(&entry.name).to_string(),
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_jail() -> (tempfile::TempDir, Jail) {
        let dir = tempfile::Builder::new().prefix("argon-vfs").tempdir().unwrap();
        let jail = Jail::new(dir.path().to_path_buf()).with_read_only(&["DEMOS"]);
        (dir, jail)
    }

    #[test]
    fn normalize_rejects_escapes() {
        assert!(normalize("../x").is_err());
        assert!(normalize("a/../b").is_err());
        assert_eq!(normalize("/a//b/./c").unwrap(), vec!["a", "b", "c"]);
        assert!(normalize("a\0").is_err());
    }

    #[test]
    fn read_write_inside_root() {
        let (_d, j) = temp_jail();
        j.write("GAMES/pong.lua", b"print(1)").unwrap();
        assert_eq!(j.read_string("GAMES/pong.lua").unwrap(), "print(1)");
        assert!(j.list("").unwrap().iter().any(|e| e.name == "GAMES" && e.is_dir));
        assert!(j.write("../evil.lua", b"").is_err());
        assert!(j.read("/etc/passwd").is_err());
    }

    #[test]
    fn symlink_escape_rejected() {
        let (d, j) = temp_jail();
        let outside = tempfile::Builder::new().prefix("argon-outside").tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), b"s").unwrap();
        std::os::unix::fs::symlink(outside.path(), d.path().join("link")).unwrap();
        assert!(j.read("link/secret.txt").is_err());
    }

    #[test]
    fn demos_are_read_only() {
        let (_d, j) = temp_jail();
        std::fs::create_dir_all(j.root().join("DEMOS")).unwrap();
        std::fs::write(j.root().join("DEMOS/hello.lua"), b"print('hi')").unwrap();
        assert!(j.read("DEMOS/hello.lua").is_ok());
        assert!(matches!(j.write("DEMOS/hello.lua", b"x"), Err(VfsError::ReadOnly(_))));
        assert!(matches!(j.delete("demos/hello.lua"), Err(VfsError::ReadOnly(_))));
    }

    #[test]
    fn resolve_program_forms() {
        let (_d, j) = temp_jail();
        j.write("pong.lua", b"").unwrap();
        j.write("shooter/main.lua", b"").unwrap();
        j.write("shooter/cart.toml", b"").unwrap();
        let p = resolve_program(&j, "", "PONG").unwrap();
        assert_eq!(p.main, "pong.lua");
        assert!(!p.is_cart);
        let c = resolve_program(&j, "", "shooter").unwrap();
        assert!(c.is_cart && c.manifest.is_some() && c.dir == "shooter");
        assert!(resolve_program(&j, "", "nothing").is_none());
    }
}
