//! User configuration (`~/.argon/config.toml`) and cart manifests
//! (`cart.toml`).

use crate::input::KeyMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    /// Display filter: none, glow, glass, scanlines, neon, crt.
    pub filter: String,
    /// Console font: tech (Share Tech Mono), proggy (ProggyClean), pixel (Unscii).
    pub font: String,
    /// Window scale factor at startup (integer multiple of the virtual
    /// resolution); 0 = fit screen.
    pub window_scale: u32,
    pub fullscreen: bool,
    /// Override of the disk location.
    pub disk: Option<PathBuf>,
    pub keymap: KeyMap,
    pub editor: EditorConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: "amber".to_string(),
            filter: "neon".to_string(),
            font: "tech".to_string(),
            window_scale: 0,
            fullscreen: false,
            disk: None,
            keymap: KeyMap::default(),
            editor: EditorConfig::default(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Config {
        match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
                eprintln!("argon: bad config {}: {e}; using defaults", path.display());
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let s = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, s)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub tab_size: usize,
    pub show_line_numbers: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        EditorConfig { tab_size: 2, show_line_numbers: true }
    }
}

/// Resolved host locations for Argon's data.
#[derive(Clone, Debug)]
pub struct Paths {
    pub base: PathBuf,
    pub disk: PathBuf,
    pub export: PathBuf,
    pub config: PathBuf,
    pub history: PathBuf,
}

impl Paths {
    /// `~/.argon` (or the platform data dir) unless portable mode.
    pub fn detect(portable: bool, disk_override: Option<&Path>) -> Paths {
        let base = if portable {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("argon-data")))
                .unwrap_or_else(|| PathBuf::from("argon-data"))
        } else {
            directories::UserDirs::new()
                .map(|u| u.home_dir().join(".argon"))
                .unwrap_or_else(|| PathBuf::from(".argon"))
        };
        Paths::at(base, disk_override)
    }

    pub fn at(base: PathBuf, disk_override: Option<&Path>) -> Paths {
        let disk = disk_override.map(|p| p.to_path_buf()).unwrap_or_else(|| base.join("disk"));
        Paths {
            export: base.join("export"),
            config: base.join("config.toml"),
            history: base.join("history.txt"),
            disk,
            base,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.disk)?;
        std::fs::create_dir_all(self.disk.join("SAVE"))?;
        std::fs::create_dir_all(&self.export)?;
        Ok(())
    }
}

/// `cart.toml` (FR-F3). Also parsed from an `--- argon:` header in single
/// files, see [`Manifest::from_header`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    pub title: String,
    pub author: String,
    pub api: u32,
    /// Preset name or "WxH".
    pub resolution: String,
    /// "truecolor" or a palette size as string? Keep two fields for clarity.
    pub color: String,
    pub palette: Option<u32>,
    pub fps: u32,
    /// Display filter override while the cart runs; empty = user's setting.
    pub filter: String,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            title: String::new(),
            author: String::new(),
            api: 1,
            resolution: "classic".to_string(),
            color: "truecolor".to_string(),
            palette: None,
            fps: 60,
            filter: String::new(),
        }
    }
}

impl Manifest {
    pub fn parse(toml_src: &str) -> Result<Manifest, toml::de::Error> {
        toml::from_str(toml_src)
    }

    /// Single-file programs may start with lines like
    /// `-- argon: resolution = "retro"` (Lua) or `// argon: fps = 30` (GLSL);
    /// each becomes a TOML line.
    pub fn from_header(src: &str) -> Manifest {
        let mut toml_lines = Vec::new();
        for line in src.lines().take(32) {
            let t = line.trim();
            let rest = t.strip_prefix("--").or_else(|| t.strip_prefix("//")).map(str::trim);
            if let Some(kv) = rest.and_then(|r| r.strip_prefix("argon:")) {
                toml_lines.push(kv.trim().to_string());
            }
        }
        if toml_lines.is_empty() {
            return Manifest::default();
        }
        Manifest::parse(&toml_lines.join("\n")).unwrap_or_default()
    }

    pub fn resolution(&self) -> (u32, u32) {
        crate::resolution_preset(&self.resolution).unwrap_or((crate::DEFAULT_WIDTH, crate::DEFAULT_HEIGHT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let c = Config::default();
        let s = toml::to_string(&c).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn header_manifest() {
        let m = Manifest::from_header("-- argon: resolution = \"retro\"\n-- argon: fps = 30\nprint(1)");
        assert_eq!(m.resolution(), (320, 240));
        assert_eq!(m.fps, 30);
        assert_eq!(Manifest::from_header("print(1)").resolution(), (640, 400));
    }
}
