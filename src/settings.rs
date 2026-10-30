//! settings.rs — three on/off choices that survive a restart: Vedic mode,
//! Kannada digits and the startup update check. Stored as a tiny text file
//! (`key=value` lines), so there is no extra dependency and you can edit it by
//! hand.
//!
//! * Windows: `%APPDATA%\KannadaIME\settings.txt`
//! * Linux:   `~/.config/kannada-ime/settings.txt` (or `$XDG_CONFIG_HOME`)
//!
//! A missing or unreadable file simply means the defaults (all on). A line the
//! program does not understand (a typo such as `vedic=tru`) keeps the default
//! for that choice and is reported once at startup (0.5.3; a typo used to
//! switch the choice off silently). Saving writes a temporary file first and
//! then replaces the old one, so a crash can never leave half a file behind.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Vedic accent keys and the agama anusvara are active.
    pub vedic: bool,
    /// The keys 0-9 (and the numpad) type Kannada digits ೦-೯; off = English digits.
    pub kannada_digits: bool,
    /// Ask GitHub once at startup whether a newer release exists.
    pub update_check: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            vedic: true,
            kannada_digits: true,
            update_check: true,
        }
    }
}

/// `1 true on yes` / `0 false off no` (any case). Anything else: not understood.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

impl Settings {
    /// Parse the file text. Comments and blank lines are skipped; a missing key
    /// keeps its default. Lines that are not understood are ignored here; see
    /// [`Settings::parse_report`] for the list of them.
    #[allow(dead_code)]
    pub fn parse(text: &str) -> Self {
        Self::parse_report(text).0
    }

    /// Parse the file text and also return one message per line that was not
    /// understood (unknown key, value that is not on/off, no `=`). Such a line
    /// changes nothing: the default (or an earlier valid line) stays.
    pub fn parse_report(text: &str) -> (Self, Vec<String>) {
        let mut s = Self::default();
        let mut problems = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let bad = |why: &str| format!("settings.txt line {}: \"{line}\" {why}", n + 1);
            let Some((key, value)) = line.split_once('=') else {
                problems.push(bad("is not understood and was ignored."));
                continue;
            };
            let slot = match key.trim() {
                "vedic" => &mut s.vedic,
                "kannada_digits" => &mut s.kannada_digits,
                "update_check" => &mut s.update_check,
                _ => {
                    problems.push(bad("has an unknown name and was ignored."));
                    continue;
                }
            };
            match parse_bool(value) {
                Some(on) => *slot = on,
                None => problems.push(bad(
                    "is not understood (use 1 or 0), so the default (on) is used.",
                )),
            }
        }
        (s, problems)
    }

    pub fn to_text(&self) -> String {
        format!(
            "# Kannada IME settings (1 = on, 0 = off)\nvedic={}\nkannada_digits={}\nupdate_check={}\n",
            self.vedic as u8, self.kannada_digits as u8, self.update_check as u8
        )
    }

    /// The saved settings (problems in the file are ignored).
    #[allow(dead_code)]
    pub fn load() -> Self {
        Self::load_report().0
    }

    /// The saved settings, plus messages for any lines that were not understood.
    pub fn load_report() -> (Self, Vec<String>) {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Self::parse_report(&t))
            .unwrap_or_default()
    }

    /// Save: write `settings.txt.tmp`, then replace `settings.txt` with it.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(p) = path() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no settings folder (APPDATA / HOME is not set)",
            ));
        };
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = p.with_extension("txt.tmp");
        std::fs::write(&tmp, self.to_text())?;
        std::fs::rename(&tmp, &p).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }
}

/// The folder that holds settings.txt (used by `--cleanup`).
pub fn folder() -> Option<PathBuf> {
    path().and_then(|p| p.parent().map(PathBuf::from))
}

#[cfg(windows)]
fn path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .filter(|a| !a.is_empty())
        .map(|a| PathBuf::from(a).join("KannadaIME").join("settings.txt"))
}

#[cfg(not(windows))]
fn path() -> Option<PathBuf> {
    // XDG: an empty or relative XDG_CONFIG_HOME must be ignored (0.5.3; an empty
    // one used to put settings.txt under the current folder).
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".config"))
        })?;
    Some(base.join("kannada-ime").join("settings.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_all_on() {
        let s = Settings::default();
        assert!(s.vedic && s.kannada_digits && s.update_check);
        assert_eq!(Settings::parse(""), s);
    }

    #[test]
    fn round_trip() {
        for (v, u) in [(true, true), (true, false), (false, true), (false, false)] {
            for d in [true, false] {
                let s = Settings {
                    vedic: v,
                    kannada_digits: d,
                    update_check: u,
                };
                assert_eq!(Settings::parse_report(&s.to_text()), (s, vec![]));
            }
        }
    }

    #[test]
    fn parse_is_forgiving() {
        let s = Settings::parse("# comment\n\n  vedic = 0 \nupdate_check=YES\n");
        assert!(!s.vedic);
        assert!(s.update_check);
        // A missing key keeps its default (an old 0.4 file has no digits key).
        assert!(Settings::parse("update_check=0").vedic);
        assert!(Settings::parse("vedic=0").kannada_digits);
        assert!(!Settings::parse("kannada_digits=0").kannada_digits);
        assert!(!Settings::parse("vedic=off").vedic);
        assert!(!Settings::parse("vedic=No").vedic);
    }

    #[test]
    fn typos_keep_the_default_and_are_reported() {
        // 0.5.3: a value that is not on/off no longer switches the choice off.
        let (s, problems) = Settings::parse_report("vedic=tru\nkannada_digits=0\n");
        assert!(s.vedic, "typo keeps the default (on)");
        assert!(!s.kannada_digits, "valid lines still apply");
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("line 1") && problems[0].contains("vedic=tru"));
        // Unknown names and lines without '=' are reported too, and change nothing.
        let (s, problems) = Settings::parse_report("vedc=0\nno equals sign here\n# ok\n");
        assert_eq!(s, Settings::default());
        assert_eq!(problems.len(), 2);
        assert!(problems[1].contains("line 2"));
        // A clean file reports nothing.
        assert!(Settings::parse_report(&Settings::default().to_text()).1.is_empty());
    }
}
