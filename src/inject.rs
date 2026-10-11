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
//!   clipboard content with nothing and blocks the hook for ~40 ms per edit.
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
}

impl<K: Keys> Injector<K> {
    pub fn new(keys: K, mode: Mode) -> Self {
        let clipboard = if mode == Mode::Clipboard {
            arboard::Clipboard::new().ok()
        } else {
            None
        };
        Self {
            keys,
            mode,
            clipboard,
        }
    }

    pub fn apply(&mut self, edit: &Edit) {
        if edit.is_noop() {
            return;
        }
        match (self.mode, self.clipboard.as_mut()) {
            (Mode::Clipboard, Some(cb)) if !edit.text.is_empty() => {
                self.keys.backspace(edit.backspaces);
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
                    // Clipboard busy: fall back to direct typing.
                    self.keys.unicode(0, &edit.text);
                }
            }
            _ => self.keys.unicode(edit.backspaces, &edit.text),
        }
    }
}
