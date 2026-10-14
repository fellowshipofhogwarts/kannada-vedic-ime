//! settings.rs — two on/off choices that survive a restart: Vedic mode and the
//! startup update check. Stored as a tiny text file (`key=value` lines), so there
//! is no extra dependency and you can edit it by hand.
//!
//! * Windows: `%APPDATA%\KannadaIME\settings.txt`
//! * Linux:   `~/.config/kannada-ime/settings.txt` (or `$XDG_CONFIG_HOME`)
//!
//! Reading and writing never fail loudly: a missing or unreadable file simply
//! means the defaults (both on).

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Vedic accent keys and the agama anusvara are active.
    pub vedic: bool,
    /// Ask GitHub once at startup whether a newer release exists.
    pub update_check: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            vedic: true,
            update_check: true,
        }
    }
}

impl Settings {
    /// Parse the file text. Unknown keys and comments are ignored; a missing key
    /// keeps its default.
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let on = matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "on" | "yes"
                );
                match key.trim() {
                    "vedic" => s.vedic = on,
                    "update_check" => s.update_check = on,
                    _ => {}
                }
            }
        }
        s
    }

    pub fn to_text(&self) -> String {
        format!(
            "# Kannada IME settings (1 = on, 0 = off)\nvedic={}\nupdate_check={}\n",
            self.vedic as u8, self.update_check as u8
        )
    }

    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, self.to_text());
        }
    }
}

#[cfg(windows)]
fn path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("KannadaIME").join("settings.txt"))
}

#[cfg(not(windows))]
fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("kannada-ime").join("settings.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_both_on() {
        let s = Settings::default();
        assert!(s.vedic && s.update_check);
        assert_eq!(Settings::parse(""), s);
    }

    #[test]
    fn round_trip() {
        for (v, u) in [(true, true), (true, false), (false, true), (false, false)] {
            let s = Settings {
                vedic: v,
                update_check: u,
            };
            assert_eq!(Settings::parse(&s.to_text()), s);
        }
    }

    #[test]
    fn parse_is_forgiving() {
        let s = Settings::parse("# comment\n\n  vedic = 0 \nunknown=1\nupdate_check=YES\n");
        assert!(!s.vedic);
        assert!(s.update_check);
        // A missing key keeps its default.
        assert!(Settings::parse("update_check=0").vedic);
        // Garbage values count as off, garbage lines are skipped.
        assert!(!Settings::parse("vedic=maybe").vedic);
        assert_eq!(Settings::parse("no equals sign here"), Settings::default());
    }
}
