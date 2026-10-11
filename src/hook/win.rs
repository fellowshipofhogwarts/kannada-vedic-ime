//! hook/win.rs — Windows backend: WH_KEYBOARD_LL + WH_MOUSE_LL hooks and
//! `SendInput` injection, via `windows-sys` (the raw-bindings sibling of
//! `windows-rs`, from the same project; smaller and faster to compile).
//!
//! Threading: everything below runs on ONE dedicated thread that owns a Win32
//! message loop (required for low-level hooks). The hook procedures reach their
//! state through a `thread_local!`, never a global lock.
//!
//! Re-entrancy: our own `SendInput` events come back through the hook. They are
//! recognised by `LLKHF_INJECTED` / our `INJECT_TAG` and passed on *before*
//! touching any state.
//!
//! Limits (Windows rules, not bugs): a non-elevated process cannot hook input
//! going to elevated windows (UIPI) — run this exe as administrator if you need
//! to type into those; Windows may silently drop a low-level hook that blocks
//! longer than `LowLevelHooksTimeout`, which is why the handler does no I/O.

use std::cell::RefCell;
use std::ptr::null;
use std::sync::Arc;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyboardLayout, SendInput, ToUnicodeEx, INPUT, INPUT_0, INPUT_KEYBOARD,
    KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_LSHIFT,
    VK_LWIN, VK_MENU, VK_NUMLOCK, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
    WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN,
};

use super::filter::{is_injected, INJECT_TAG};
use super::HookConfig;
use crate::engine::{Edit, Engine, Outcome};
use crate::inject::{Injector, Keys};
use crate::state::AppState;

const VK_K: u32 = 0x4B;

/// A zeroed Win32 handle. `windows-sys` changed handle types between pointer and
/// integer across releases; a zeroed value is the NULL handle in both.
fn nil<T>() -> T {
    // SAFETY: only instantiated with pointer/integer handle types.
    unsafe { std::mem::zeroed() }
}

// ---------------------------------------------------------------------------
// Injection
// ---------------------------------------------------------------------------

pub struct WinKeys;

fn key_event(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECT_TAG,
            },
        },
    }
}

fn send(inputs: &[INPUT]) {
    if inputs.is_empty() {
        return;
    }
    // SAFETY: `inputs` is a valid slice of fully initialised INPUT structs.
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}

fn push_backspaces(v: &mut Vec<INPUT>, n: usize) {
    for _ in 0..n {
        v.push(key_event(VK_BACK, 0, 0));
        v.push(key_event(VK_BACK, 0, KEYEVENTF_KEYUP));
    }
}

impl Keys for WinKeys {
    fn backspace(&mut self, n: usize) {
        let mut v = Vec::with_capacity(n * 2);
        push_backspaces(&mut v, n);
        send(&v);
    }

    fn paste_chord(&mut self) {
        send(&[
            key_event(VK_CONTROL, 0, 0),
            key_event(b'V' as u16, 0, 0),
            key_event(b'V' as u16, 0, KEYEVENTF_KEYUP),
            key_event(VK_CONTROL, 0, KEYEVENTF_KEYUP),
        ]);
    }

    /// One `SendInput` call for the whole edit: Windows inserts a batch into the
    /// input stream atomically, so keystrokes the user types meanwhile cannot
    /// interleave with our backspaces and text.
    fn unicode(&mut self, backspaces: usize, text: &str) {
        let mut v = Vec::with_capacity(backspaces * 2 + text.len() * 4);
        push_backspaces(&mut v, backspaces);
        for unit in text.encode_utf16() {
            v.push(key_event(0, unit, KEYEVENTF_UNICODE));
            v.push(key_event(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
        }
        send(&v);
    }
}

// ---------------------------------------------------------------------------
// Key state helpers
// ---------------------------------------------------------------------------

fn down(vk: u16) -> bool {
    // SAFETY: plain query.
    (unsafe { GetAsyncKeyState(vk as i32) } as u16) & 0x8000 != 0
}

fn shift() -> bool {
    down(VK_SHIFT)
}
fn ctrl() -> bool {
    down(VK_CONTROL)
}
fn alt() -> bool {
    down(VK_MENU)
}
fn win() -> bool {
    down(VK_LWIN) || down(VK_RWIN)
}

fn is_pure_modifier(vk: u16) -> bool {
    matches!(
        vk,
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT | VK_CAPITAL | VK_NUMLOCK | VK_SCROLL
    )
}

/// Resolve the character the user's layout would produce for this key, with
/// **Caps Lock ignored** (only Shift counts) and without disturbing dead-key
/// state (flag 0x4, Windows 10 1607+). `None` for non-character keys.
fn translate(kb: &KBDLLHOOKSTRUCT) -> Option<char> {
    let mut state = [0u8; 256];
    if shift() {
        state[VK_SHIFT as usize] = 0x80;
    }
    let mut buf = [0u16; 4];
    // SAFETY: buffers are valid for the lengths passed.
    let n = unsafe {
        let tid = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
        ToUnicodeEx(
            kb.vkCode,
            kb.scanCode,
            state.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as i32,
            0x4,
            GetKeyboardLayout(tid),
        )
    };
    if n == 1 {
        char::from_u32(buf[0] as u32).filter(|c| !c.is_control())
    } else {
        None // 0 = no char, <0 = dead key, >1 = sequence: not ours
    }
}

// ---------------------------------------------------------------------------
// Hook context (lives on the hook thread)
// ---------------------------------------------------------------------------

struct Ctx {
    engine: Engine,
    state: Arc<AppState>,
    on_hotkey: Arc<dyn Fn() + Send + Sync>,
    injector: Injector<WinKeys>,
    epoch: u32,
    foreground: usize,
    held: [bool; 256],
    swallowed: [bool; 256],
}

thread_local! {
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

impl Ctx {
    fn inject(&mut self, e: &Edit) {
        self.injector.apply(e);
    }

    /// Returns true if the event must be swallowed.
    fn on_key(&mut self, kb: &KBDLLHOOKSTRUCT, is_down: bool) -> bool {
        let vk = (kb.vkCode & 0xFF) as usize;

        if !is_down {
            self.held[vk] = false;
            return std::mem::take(&mut self.swallowed[vk]);
        }
        let repeat = std::mem::replace(&mut self.held[vk], true);

        // Flush on ON/OFF flips made elsewhere (menu) and on focus changes.
        let epoch = self.state.epoch();
        // SAFETY: plain query.
        let fg = unsafe { GetForegroundWindow() } as usize;
        if epoch != self.epoch || fg != self.foreground {
            self.epoch = epoch;
            self.foreground = fg;
            self.engine.reset();
        }

        // Global hotkey Ctrl+Shift+K — works in both ON and OFF state.
        if kb.vkCode == VK_K && ctrl() && shift() && !alt() {
            if !repeat {
                self.state.toggle();
                self.epoch = self.state.epoch();
                self.engine.reset();
                (self.on_hotkey)();
            }
            self.swallowed[vk] = true;
            return true;
        }

        if !self.state.is_active() {
            return false;
        }

        let vk16 = vk as u16;
        if is_pure_modifier(vk16) {
            return false; // pressing Shift mid-word must not end the word
        }
        if ctrl() || alt() || win() {
            self.engine.reset(); // shortcut: hands off
            return false;
        }

        let outcome = if vk16 == VK_BACK {
            self.engine.on_backspace()
        } else {
            match translate(kb) {
                Some(c) => self.engine.on_char(c),
                None => {
                    self.engine.reset(); // arrows, Enter, Tab, F-keys, dead keys…
                    return false;
                }
            }
        };

        match outcome {
            Outcome::Consumed(edit) => {
                self.inject(&edit);
                self.swallowed[vk] = true;
                true
            }
            Outcome::Pass => false,
        }
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, lparam points to a KBDLLHOOKSTRUCT.
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        // Loop guard: events we synthesised with SendInput re-enter this very
        // hook. LLKHF_INJECTED (or our INJECT_TAG) marks them; they are passed
        // on before any state is touched, so the engine never sees its own
        // output. See hook/filter.rs (unit-tested).
        let ours = is_injected(kb.flags, kb.dwExtraInfo);
        if !ours {
            let msg = wparam as u32;
            let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
            if is_down || is_up {
                let swallow = CTX.with(|c| match c.try_borrow_mut() {
                    Ok(mut g) => g.as_mut().is_some_and(|ctx| ctx.on_key(kb, is_down)),
                    Err(_) => false,
                });
                if swallow {
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(nil(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let msg = wparam as u32;
        if matches!(
            msg,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        ) {
            // The caret probably moved: forget the current word.
            CTX.with(|c| {
                if let Ok(mut g) = c.try_borrow_mut() {
                    if let Some(ctx) = g.as_mut() {
                        ctx.engine.reset();
                    }
                }
            });
        }
    }
    CallNextHookEx(nil(), code, wparam, lparam)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Start the hook thread. Returns immediately.
pub fn spawn(cfg: HookConfig) {
    std::thread::Builder::new()
        .name("kime-hook".into())
        .spawn(move || {
            CTX.with(|c| {
                *c.borrow_mut() = Some(Ctx {
                    engine: Engine::new(),
                    epoch: cfg.state.epoch(),
                    state: cfg.state,
                    on_hotkey: cfg.on_hotkey,
                    injector: Injector::new(WinKeys, cfg.mode),
                    foreground: 0,
                    held: [false; 256],
                    swallowed: [false; 256],
                })
            });
            // SAFETY: standard hook installation + message loop on this thread.
            unsafe {
                SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
                let module = GetModuleHandleW(null());
                let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
                let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
                if kb as usize == 0 {
                    eprintln!("kannada_ime: SetWindowsHookExW(WH_KEYBOARD_LL) failed");
                    return;
                }
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, nil(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                UnhookWindowsHookEx(kb);
                if ms as usize != 0 {
                    UnhookWindowsHookEx(ms);
                }
            }
        })
        .expect("failed to spawn keyboard hook thread");
}

/// False if another instance already owns the named mutex.
pub fn single_instance() -> bool {
    let name: Vec<u16> = "Local\\KannadaIME.SingleInstance\0"
        .encode_utf16()
        .collect();
    // SAFETY: valid NUL-terminated wide string; the handle is intentionally leaked
    // so the mutex lives as long as the process.
    unsafe {
        let h = CreateMutexW(null(), 0, name.as_ptr());
        if h as usize == 0 {
            return true; // can't tell; don't block startup
        }
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}
