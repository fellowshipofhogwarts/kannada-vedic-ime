//! hook/linux.rs — Linux backend (beta, US keyboard layout only).
//!
//! Capture : `rdev::grab` (evdev, exclusive grab; works on X11 *and* Wayland
//!           because it reads /dev/input directly). Needs read/write access to
//!           /dev/input/* and /dev/uinput — add your user to the `input` group.
//! Inject  : `enigo` (X11 via x11rb/xdo; Wayland via the compositor's virtual
//!           keyboard / libei where supported). Not all Wayland compositors
//!           allow synthetic Unicode — see README "Linux notes", or use
//!           `--inject clipboard`.
//!
//! Keys are mapped from the *physical* US-QWERTY position (rdev gives key
//! identities, not layout-resolved characters). That makes the engine
//! independent of Caps Lock, but it also means this agent only suits a US
//! layout: on a German keyboard Y and Z are swapped, AltGr counts as Alt and
//! dead keys are not seen. The planned IBus input method (which receives
//! layout-resolved characters) is the proper fix for other layouts.
//!
//! Longer term, the "right" Linux design is an IBus/Fcitx5 engine; this agent
//! trades that for being a single self-contained binary.

use std::cell::RefCell;

use enigo::{Direction, Enigo, Key as EKey, Keyboard, Settings};
use rdev::{grab, Event, EventType, Key};

use super::HookConfig;
use crate::engine::{Engine, Outcome};
use crate::inject::{Injector, Keys};
use crate::state::AppState;
use std::sync::Arc;

pub struct LinuxKeys {
    enigo: Enigo,
}

impl Keys for LinuxKeys {
    fn backspace(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.enigo.key(EKey::Backspace, Direction::Click);
        }
    }

    fn paste_chord(&mut self) {
        let _ = self.enigo.key(EKey::Control, Direction::Press);
        let _ = self.enigo.key(EKey::Unicode('v'), Direction::Click);
        let _ = self.enigo.key(EKey::Control, Direction::Release);
    }

    fn unicode(&mut self, backspaces: usize, text: &str) {
        self.backspace(backspaces);
        if !text.is_empty() {
            let _ = self.enigo.text(text);
        }
    }
}

/// US-QWERTY character for the keys the scheme uses. `None` = not a text key we
/// care about (digits, space, Enter, arrows… all just end the current word).
fn key_char(key: Key, shift: bool) -> Option<char> {
    use Key::*;
    let lower = match key {
        KeyA => 'a',
        KeyB => 'b',
        KeyC => 'c',
        KeyD => 'd',
        KeyE => 'e',
        KeyF => 'f',
        KeyG => 'g',
        KeyH => 'h',
        KeyI => 'i',
        KeyJ => 'j',
        KeyK => 'k',
        KeyL => 'l',
        KeyM => 'm',
        KeyN => 'n',
        KeyO => 'o',
        KeyP => 'p',
        KeyQ => 'q',
        KeyR => 'r',
        KeyS => 's',
        KeyT => 't',
        KeyU => 'u',
        KeyV => 'v',
        KeyW => 'w',
        KeyX => 'x',
        KeyY => 'y',
        KeyZ => 'z',
        SemiColon => return Some(if shift { ':' } else { ';' }),
        Quote => return Some(if shift { '"' } else { '\'' }),
        BackQuote => return Some(if shift { '~' } else { '`' }),
        BackSlash => return Some(if shift { '|' } else { '\\' }),
        Minus => return Some(if shift { '_' } else { '-' }),
        Num6 => return Some(if shift { '^' } else { '6' }),
        _ => return None,
    };
    Some(if shift {
        lower.to_ascii_uppercase()
    } else {
        lower
    })
}

struct Ctx {
    engine: Engine,
    state: Arc<AppState>,
    on_hotkey: Arc<dyn Fn() + Send + Sync>,
    injector: Injector<LinuxKeys>,
    epoch: u32,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
    swallowed: Vec<Key>,
}

impl Ctx {
    fn handle(&mut self, ev: Event) -> Option<Event> {
        match ev.event_type {
            EventType::KeyPress(key) => {
                if self.on_press(key) {
                    self.swallowed.push(key);
                    None
                } else {
                    Some(ev)
                }
            }
            EventType::KeyRelease(key) => {
                self.track_modifier(key, false);
                if let Some(i) = self.swallowed.iter().position(|k| *k == key) {
                    self.swallowed.swap_remove(i);
                    None
                } else {
                    Some(ev)
                }
            }
            EventType::ButtonPress(_) => {
                self.engine.reset();
                Some(ev)
            }
            _ => Some(ev),
        }
    }

    fn track_modifier(&mut self, key: Key, pressed: bool) -> bool {
        match key {
            Key::ShiftLeft | Key::ShiftRight => self.shift = pressed,
            Key::ControlLeft | Key::ControlRight => self.ctrl = pressed,
            Key::Alt | Key::AltGr => self.alt = pressed,
            Key::MetaLeft | Key::MetaRight => self.meta = pressed,
            Key::CapsLock => {}
            _ => return false,
        }
        true
    }

    /// True = swallow this key press.
    fn on_press(&mut self, key: Key) -> bool {
        if self.track_modifier(key, true) {
            // Shift/Caps pressed mid-word must not end the word; Ctrl/Alt/Meta do.
            if self.ctrl || self.alt || self.meta {
                self.engine.reset();
            }
            return false;
        }

        let epoch = self.state.epoch();
        if epoch != self.epoch {
            self.epoch = epoch;
            self.engine.reset();
        }

        if key == Key::KeyK && self.ctrl && self.shift && !self.alt {
            self.state.toggle();
            self.epoch = self.state.epoch();
            self.engine.reset();
            (self.on_hotkey)();
            return true;
        }

        if !self.state.is_active() {
            return false;
        }
        self.engine.set_vedic(self.state.is_vedic());
        if self.ctrl || self.alt || self.meta {
            self.engine.reset();
            return false;
        }

        let outcome = if key == Key::Backspace {
            self.engine.on_backspace()
        } else {
            match key_char(key, self.shift) {
                Some(c) => self.engine.on_char(c),
                None => {
                    self.engine.reset();
                    return false;
                }
            }
        };

        match outcome {
            Outcome::Consumed(edit) => {
                self.injector.apply(&edit);
                true
            }
            Outcome::Pass => false,
        }
    }
}

/// Start the capture thread. Returns immediately.
pub fn spawn(cfg: HookConfig) {
    std::thread::Builder::new()
        .name("kime-hook".into())
        .spawn(move || {
            let enigo = match Enigo::new(&Settings::default()) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("kannada_ime: cannot create input injector: {e}");
                    (cfg.on_error)(format!("Cannot create the input injector: {e}"));
                    return;
                }
            };
            // RefCell keeps the closure `Fn`-compatible whichever bound rdev uses.
            let ctx = RefCell::new(Ctx {
                engine: Engine::new(),
                epoch: cfg.state.epoch(),
                state: cfg.state,
                on_hotkey: cfg.on_hotkey,
                injector: Injector::new(LinuxKeys { enigo }, cfg.mode),
                shift: false,
                ctrl: false,
                alt: false,
                meta: false,
                swallowed: Vec::new(),
            });
            if let Err(e) = grab(move |ev| ctx.borrow_mut().handle(ev)) {
                let msg = format!(
                    "Keyboard grab failed ({e:?}). Add your user to the 'input' group \
                     (sudo usermod -aG input $USER), then log out and back in."
                );
                eprintln!("kannada_ime: {msg}");
                (cfg.on_error)(msg);
            }
        })
        .expect("failed to spawn keyboard hook thread");
}
