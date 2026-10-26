//! inject.rs — turns an engine [`Edit`] into synthetic input in the focused window.
//!
//! Two strategies, selectable with `--inject`:
//!
//! * **Unicode** (default): backspaces + the text as synthetic Unicode key
//!   events. On Windows this is `SendInput` with `KEYEVENTF_UNICODE` (one atomic
//!   batch, no clipboard involved). On Linux it is `enigo`'s text typing.
//! * **Clipboard** (fallback for apps that ignore synthetic Unicode events):
//!   backspaces, then put the text on the clipboard and send Ctrl+V, then
//!   restore the previous *text* clipboard. Experimental: it replaces non-text
//!   clipboard content with nothing.
//!
//! ## Restoring the clipboard (0.5.1, fix C1)
//!
//! On Windows the injector runs *inside* the low-level keyboard hook. Windows
//! delivers the Ctrl+V we synthesise only after the hook has returned, so the
//! old code (sleep 40 ms, then restore) put the user's previous clipboard back
//! *before* the target app pasted: the app pasted the user's old text.
//!
//! [`Injector::with_deferred_restore`] fixes that: `apply` only sets the
//! clipboard and sends Ctrl+V, and remembers the user's original text. The
//! platform layer calls [`Injector::restore_clipboard`] later, from a timer,
//! after the hook has returned. While a restore is pending, further edits keep
//! the *original* saved text (they never save our own Kannada text by mistake).
//!
//! [`Injector::new`] keeps the old immediate behaviour (used by the Linux hook).
//!
//! Platform specifics live behind the small [`Keys`] trait.

use crate::engine::Edit;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Unicode,
    Clipboard,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "unicode" => Some(Mode::Unicode),
            "clipboard" | "paste" => Some(Mode::Clipboard),
            _ => None,
        }
    }
}

/// Platform key synthesis. Every method must be fast and must tag its events so
/// the keyboard hook can recognise and ignore them.
pub trait Keys {
    /// Press-and-release Backspace `n` times.
    fn backspace(&mut self, n: usize);
    /// Press Ctrl+V.
    fn paste_chord(&mut self);
    /// Backspaces followed by `text` as Unicode key events, ideally as one batch.
    fn unicode(&mut self, backspaces: usize, text: &str);
}

pub struct Injector<K: Keys> {
    keys: K,
    mode: Mode,
    clipboard: Option<arboard::Clipboard>,
    /// true: the caller restores the clipboard later via `restore_clipboard`.
    deferred_restore: bool,
    /// A restore is due (deferred mode only).
    restore_pending: bool,
    /// The user's own clipboard text, saved before our first paste.
    saved: Option<String>,
}

impl<K: Keys> Injector<K> {
    /// Immediate restore (sleep, then put the old text back). Linux backend.
    #[cfg_attr(windows, allow(dead_code))]
    pub fn new(keys: K, mode: Mode) -> Self {
        Self::build(keys, mode, false)
    }

    /// Deferred restore: the caller must call [`Self::restore_clipboard`] after
    /// the target app has had time to paste (Windows hook backend).
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn with_deferred_restore(keys: K, mode: Mode) -> Self {
        Self::build(keys, mode, true)
    }

    fn build(keys: K, mode: Mode, deferred_restore: bool) -> Self {
        let clipboard = if mode == Mode::Clipboard {
            arboard::Clipboard::new().ok()
        } else {
            None
        };
        Self {
            keys,
            mode,
            clipboard,
            deferred_restore,
            restore_pending: false,
            saved: None,
        }
    }

    pub fn apply(&mut self, edit: &Edit) {
        if edit.is_noop() {
            return;
        }
        match (self.mode, self.clipboard.as_mut()) {
            (Mode::Clipboard, Some(cb)) if !edit.text.is_empty() => {
                self.keys.backspace(edit.backspaces);
                if self.deferred_restore {
                    // Save the user's text only once per burst of edits.
                    if !self.restore_pending {
                        self.saved = cb.get_text().ok();
                    }
                    if cb.set_text(edit.text.clone()).is_ok() {
                        self.keys.paste_chord();
                        self.restore_pending = true;
                    } else {
                        // Clipboard busy: fall back to direct typing.
                        self.keys.unicode(0, &edit.text);
                    }
                } else {
                    let saved = cb.get_text().ok();
                    if cb.set_text(edit.text.clone()).is_ok() {
                        self.keys.paste_chord();
                        // Give the target app time to read the clipboard before we
                        // put the user's own text back.
                        std::thread::sleep(Duration::from_millis(40));
                        if let Some(prev) = saved {
                            let _ = cb.set_text(prev);
                        }
                    } else {
                        self.keys.unicode(0, &edit.text);
                    }
                }
            }
            _ => self.keys.unicode(edit.backspaces, &edit.text),
        }
    }

    /// True if the caller still has to call [`Self::restore_clipboard`].
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn restore_pending(&self) -> bool {
        self.restore_pending
    }

    /// Put the user's saved clipboard text back. Safe to call at any time; does
    /// nothing when no restore is pending.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn restore_clipboard(&mut self) {
        if !self.restore_pending {
            return;
        }
        self.restore_pending = false;
        let saved = self.saved.take();
        if let (Some(cb), Some(prev)) = (self.clipboard.as_mut(), saved) {
            let _ = cb.set_text(prev);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeKeys {
        log: Vec<String>,
    }

    impl Keys for FakeKeys {
        fn backspace(&mut self, n: usize) {
            self.log.push(format!("bs{n}"));
        }
        fn paste_chord(&mut self) {
            self.log.push("paste".into());
        }
        fn unicode(&mut self, backspaces: usize, text: &str) {
            self.log.push(format!("uni{backspaces}:{text}"));
        }
    }

    fn edit(backspaces: usize, text: &str) -> Edit {
        Edit {
            backspaces,
            text: text.into(),
        }
    }

    #[test]
    fn mode_parse() {
        assert_eq!(Mode::parse("Unicode"), Some(Mode::Unicode));
        assert_eq!(Mode::parse("PASTE"), Some(Mode::Clipboard));
        assert_eq!(Mode::parse("x"), None);
    }

    #[test]
    fn unicode_mode_sends_one_batch_and_never_needs_a_restore() {
        let mut inj = Injector::with_deferred_restore(FakeKeys::default(), Mode::Unicode);
        inj.apply(&edit(1, "ಕೀ"));
        inj.apply(&Edit::default()); // no-op sends nothing
        assert_eq!(inj.keys.log, vec!["uni1:ಕೀ".to_string()]);
        assert!(!inj.restore_pending());
        inj.restore_clipboard(); // harmless
        assert!(!inj.restore_pending());
    }

    #[test]
    fn backspace_only_edit_in_clipboard_mode_types_directly() {
        // No clipboard object needed: an empty text never goes via the clipboard.
        let mut inj = Injector::with_deferred_restore(FakeKeys::default(), Mode::Clipboard);
        inj.clipboard = None;
        inj.apply(&edit(2, ""));
        assert_eq!(inj.keys.log, vec!["uni2:".to_string()]);
        assert!(!inj.restore_pending());
    }
}
