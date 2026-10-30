//! update.rs — "is there a newer release on GitHub?".
//!
//! Deliberately tiny and dependency-free: the tray asks the system's own `curl`
//! (present on Windows 10+ and on practically every Linux) for the public
//! "latest release" record, reads the `tag_name` out of it and compares version
//! numbers. No personal data is collected and nothing is downloaded or
//! installed; like any web request, GitHub sees your IP address and the program
//! name. The tray only offers to open the release page in your browser.
//!
//! GitHub's `releases/latest` never returns drafts or pre-releases, so those can
//! never trigger a notice. If `curl` is missing or there is no internet, the
//! check quietly returns `None`.

use std::process::{Command, Stdio};

/// `owner/repo` on GitHub, written once (0.5.3); the URLs are built from it.
macro_rules! owner_repo {
    () => {
        "fellowshipofhogwarts/kannada-vedic-ime"
    };
}
#[cfg(test)]
const OWNER_REPO: &str = owner_repo!();

/// Public API record of the newest published release.
pub const API_URL: &str = concat!("https://api.github.com/repos/", owner_repo!(), "/releases/latest");
/// Page the tray opens for "Download update".
pub const RELEASES_URL: &str = concat!("https://github.com/", owner_repo!(), "/releases/latest");
/// The version this program was built as.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// `"v0.4.0"` / `"0.4"` / `"0.4.0-beta"` → `(0, 4, 0)`. Anything else → `None`.
pub fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Sort key: the numbers, then `true` for a final release. A pre-release such
/// as `0.5.0-beta` comes *before* `0.5.0` (0.5.3; it used to count as equal, so
/// beta users were never offered the final release). Build metadata (`+…`) is
/// ignored.
fn version_key(s: &str) -> Option<(u32, u32, u32, bool)> {
    let (major, minor, patch) = parse_version(s)?;
    let core = s.trim().split('+').next().unwrap_or("");
    Some((major, minor, patch, !core.contains('-')))
}

/// True only if both strings are valid versions and `latest` is strictly newer.
/// Numbers are compared as numbers, so 0.10.0 is newer than 0.9.0.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (version_key(latest), version_key(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Pull the value of `"tag_name"` out of the release JSON (no JSON parser needed).
/// The value must be a string right after the colon (0.5.3): `"tag_name": null`
/// or a number gives `None` instead of whatever string comes next.
pub fn extract_tag(json: &str) -> Option<String> {
    let key = json.find("\"tag_name\"")?;
    let rest = json[key + "\"tag_name\"".len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let tag = &rest[..rest.find('"')?];
    if tag.is_empty() || tag.contains('\\') {
        None
    } else {
        Some(tag.to_string())
    }
}

#[cfg(windows)]
fn hide_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
fn hide_window(_cmd: &mut Command) {}

/// Full path of Windows' own curl (0.5.3). Run by bare name, Windows would look
/// in the program's folder first, so a planted `curl.exe` there would run.
#[cfg(windows)]
fn curl() -> std::path::PathBuf {
    crate::update::system32().join("curl.exe")
}
#[cfg(not(windows))]
fn curl() -> &'static str {
    "curl"
}

/// `%SystemRoot%\System32` (falls back to `C:\Windows\System32`). Also used by
/// tray.rs for `reg.exe`.
#[cfg(windows)]
pub fn system32() -> std::path::PathBuf {
    let root = std::env::var_os("SystemRoot")
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| "C:\\Windows".into());
    std::path::PathBuf::from(root).join("System32")
}

/// Ask GitHub for the newest release tag. Blocks for up to ~10 seconds, so call
/// it from a background thread. `None` = could not find out.
pub fn fetch_latest_tag() -> Option<String> {
    let mut cmd = Command::new(curl());
    cmd.args([
        "-fsSL",
        "-m",
        "10",
        "-H",
        "User-Agent: kannada-vedic-ime",
        "-H",
        "Accept: application/vnd.github+json",
        API_URL,
    ])
    .stdin(Stdio::null())
    .stderr(Stdio::null());
    hide_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    extract_tag(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("v0.4.0"), Some((0, 4, 0)));
        assert_eq!(parse_version("0.4"), Some((0, 4, 0)));
        assert_eq!(parse_version(" V1.2.3 "), Some((1, 2, 3)));
        assert_eq!(parse_version("0.5.0-beta.1"), Some((0, 5, 0)));
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("latest"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn final_release_is_newer_than_its_pre_release() {
        assert!(is_newer("0.5.0", "0.5.0-beta"));
        assert!(is_newer("v0.5.0", "0.5.0-rc.1"));
        assert!(!is_newer("0.5.0-beta", "0.5.0"));
        assert!(!is_newer("0.5.0-beta.2", "0.5.0-beta.1")); // not ordered: no alarm
        assert!(is_newer("0.5.1-beta", "0.5.0"));
        assert!(!is_newer("0.5.0+build7", "0.5.0"));
    }

    #[test]
    fn compares_numbers_not_text() {
        assert!(is_newer("v0.10.0", "0.9.0"));
        assert!(is_newer("0.4.0", "0.3.1"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.4.0", "0.4.0"));
        assert!(!is_newer("0.3.0", "0.4.0"));
        // Garbage never raises an alarm.
        assert!(!is_newer("nightly", "0.4.0"));
        assert!(!is_newer("0.4.0", "unknown"));
    }

    #[test]
    fn finds_the_tag_in_release_json() {
        let json = r#"{"url":"https://x","id":1,"tag_name":"v0.5.0","name":"0.5.0","draft":false}"#;
        assert_eq!(extract_tag(json).as_deref(), Some("v0.5.0"));
        let spaced = "{\n  \"tag_name\" : \"v1.0.0\",\n  \"x\": 2\n}";
        assert_eq!(extract_tag(spaced).as_deref(), Some("v1.0.0"));
        assert_eq!(extract_tag("{}"), None);
        assert_eq!(extract_tag(r#"{"tag_name":""}"#), None);
        assert_eq!(extract_tag("<html>rate limited</html>"), None);
        // null or a non-string never picks up the next key's text.
        assert_eq!(extract_tag(r#"{"tag_name":null,"name":"v9.9.9"}"#), None);
        assert_eq!(extract_tag(r#"{"tag_name": 5, "name":"v9.9.9"}"#), None);
        assert_eq!(extract_tag(r#"{"tag_name""#), None);
    }

    #[test]
    fn urls_point_at_this_project() {
        assert!(API_URL.contains(OWNER_REPO));
        assert!(RELEASES_URL.contains(OWNER_REPO));
        assert!(parse_version(CURRENT).is_some());
    }
}
