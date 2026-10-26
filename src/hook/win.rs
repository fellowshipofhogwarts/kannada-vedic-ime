//! hook/win.rs — Windows backend: WH_KEYBOARD_LL + WH_MOUSE_LL hooks and
//! `SendInput` injection, via `windows-sys` (the raw-bindings sibling of
//! `windows-rs`, from the same project; smaller and faster to compile).
//!
//! Threading: everything below runs on ONE dedicated thread that owns a Win32
//! message loop (required for low-level hooks). The hook procedures, timers and
//! the focus-event callback reach their state through `thread_local!`s, never a
//! global lock. All of them are called from this thread's message loop, so they
//! never run at the same time.
//!
//! Re-entrancy: our own `SendInput` events come back through the hook. They are
//! recognised by `LLKHF_INJECTED` / our `INJECT_TAG` and passed on *before*
//! touching any state.
//!
//! 0.5.1 changes:
//! * C1  clipboard mode: the clipboard is restored by a timer *after* the hook
//!       returned (see inject.rs), never inside the hook.
//! * H1  Ctrl / Alt / Win pressed on their own no longer end the word, so AltGr
//!       characters (e.g. `\` on a German keyboard) attach to the syllable.
//! * H2  a failed mouse hook is reported; a watchdog reinstalls the hooks if
//!       Windows silently removed them.
//! * H3  (risk reduction) the word is also forgotten when keyboard focus moves
//!       to another control inside the same window.
//! * M1  no more `mem::zeroed()` handles: null pointers instead.
//!
//! Limits (Windows rules, not bugs): a non-elevated process cannot hook input
//! going to elevated windows (UIPI) — run this exe as administrator if you need
//! to type into those; Windows may silently drop a low-level hook that blocks
//! longer than `LowLevelHooksTimeout` (the watchdog now reinstalls it).

use std::cell::{Cell, RefCell};
use std::ptr::{null, null_mut};
use std::sync::Arc;

use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyboardLayout, GetLastInputInfo, SendInput, ToUnicodeEx, INPUT, INPUT_0,
    INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, LASTINPUTINFO, VK_BACK,
    VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NUMLOCK,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    KillTimer, SetTimer, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    EVENT_OBJECT_FOCUS, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_KEYDOWN, WM_KEYUP,
    WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN,
};

use super::filter::{is_injected, INJECT_TAG};
use super::HookConfig;
use crate::engine::{Edit, Engine, Outcome};
use crate::inject::{Injector, Keys};
use crate::state::AppState;

const VK_K: u32 = 0x4B;

/// Clipboard mode: how long the target app gets to read our text before the
/// user's own clipboard text is put back. Restarted by every new edit.
const RESTORE_DELAY_MS: u32 = 300;
/// How often the watchdog checks that the hooks are still alive.
const WATCHDOG_MS: u32 = 2000;
/// Input newer than our last hook call by more than this counts as "missed".
const MISSED_INPUT_MS: u32 = 1000;
/// Consecutive watchdog ticks with missed input before the hooks are reinstalled
/// (one tick alone can be a hook call that is simply still queued).
const WATCHDOG_STRIKES: u8 = 2;

const KB_HOOK_FAILED: &str = "Windows refused the keyboard hook, so typing will not convert. \
     Close and reopen the program; if it keeps happening, security software may be blocking it.";
const MOUSE_HOOK_FAILED: &str = "Mouse clicks cannot be detected, so typing still converts but \
     the program cannot tell when you click somewhere else. After clicking, press Space or an \
     arrow key before typing, otherwise a few letters may be erased. Restart the program to try again.";

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

/// AltGr (German, French, … keyboards) is reported by Windows as Left-Ctrl and
/// Right-Alt held together. That is *not* a shortcut: it is how characters such
/// as `~ @ | { [ ] } \` are typed. A real Ctrl or the left Alt key rules it out.
fn altgr() -> bool {
    down(VK_RMENU) && down(VK_LCONTROL) && !down(VK_RCONTROL) && !down(VK_LMENU)
}

/// Keys that, pressed on their own, must not end the word.
///
/// H1: Ctrl, Alt and Win are included. Inside the hook `GetAsyncKeyState` does
/// not yet include the key being pressed, so a lone Ctrl/Alt used to fall
/// through to `translate`, which returned `None` and reset the word — and AltGr
/// (LCtrl + RAlt) reset it right before the AltGr character arrived. Real
/// shortcuts are still detected on the *next* key (Ctrl+C, Alt+Tab, Win+…),
/// where the modifier state is already visible.
fn is_pure_modifier(vk: u16) -> bool {
    matches!(
        vk,
        VK_SHIFT
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_CAPITAL
            | VK_NUMLOCK
            | VK_SCROLL
            | VK_CONTROL
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_MENU
            | VK_LMENU
            | VK_RMENU
            | VK_LWIN
            | VK_RWIN
    )
}

/// Resolve the character the user's layout would produce for this key, with
/// **Caps Lock ignored** (only Shift, and AltGr when held, count) and without
/// disturbing dead-key state (flag 0x4, Windows 10 1607+). `None` for
/// non-character keys.
///
/// Dead keys: the *grave* and *circumflex* dead keys (German Shift+´ and ^)
/// are returned as `` ` `` and `^`, so they work as the break key and the
/// Udatta key without pressing Space. Other dead keys (acute, diaeresis, …)
/// return `None` and keep their normal behaviour.
fn translate(kb: &KBDLLHOOKSTRUCT) -> Option<char> {
    let mut state = [0u8; 256];
    if shift() {
        state[VK_SHIFT as usize] = 0x80;
    }
    if altgr() {
        // ToUnicodeEx needs Ctrl+Alt in the key state to give the AltGr character.
        state[VK_CONTROL as usize] = 0x80;
        state[VK_MENU as usize] = 0x80;
    }
    let mut buf = [0u16; 4];
    // SAFETY: buffers are valid for the lengths passed.
    let n = unsafe {
        let tid = GetWindowThreadProcessId(GetForegroundWindow(), null_mut());
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
    } else if n < 0 {
        // Dead key: the spacing diacritic is stored in buf[0].
        match char::from_u32(buf[0] as u32) {
            Some(c @ ('`' | '^')) => Some(c),
            _ => None,
        }
    } else {
        None // 0 = no char, >1 = sequence: not ours
    }
}

/// Watchdog decision (pure, unit-tested): Windows saw input (`last_input`) more
/// than `MISSED_INPUT_MS` after the last event our hooks saw (`last_seen`).
/// Both are `GetTickCount` milliseconds and wrap after ~49.7 days; a `last_seen`
/// that is *newer* than `last_input` is never "behind".
fn hooks_look_dead(last_input: u32, last_seen: u32) -> bool {
    let gap = last_input.wrapping_sub(last_seen);
    gap > MISSED_INPUT_MS && gap < 0x8000_0000
}

// ---------------------------------------------------------------------------
// Hook context (lives on the hook thread)
// ---------------------------------------------------------------------------

struct Ctx {
    engine: Engine,
    state: Arc<AppState>,
    on_hotkey: Arc<dyn Fn() + Send + Sync>,
    on_error: Arc<dyn Fn(String) + Send + Sync>,
    injector: Injector<WinKeys>,
    epoch: u32,
    foreground: usize,
    held: [bool; 256],
    swallowed: [bool; 256],
    /// Running clipboard-restore timer (0 = none).
    restore_timer: usize,
    /// Each hook failure is reported once, not on every watchdog tick.
    reported_kb_failure: bool,
    reported_mouse_failure: bool,
}

thread_local! {
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
    /// (keyboard hook, mouse hook); null = not installed.
    static HOOKS: Cell<(HHOOK, HHOOK)> = const { Cell::new((null_mut(), null_mut())) };
    /// `time` of the last event either hook saw (GetTickCount clock).
    static LAST_SEEN: Cell<u32> = const { Cell::new(0) };
    static STRIKES: Cell<u8> = const { Cell::new(0) };
    /// (window, object id, child id) of the last focus event.
    static LAST_FOCUS: Cell<(usize, i32, i32)> = const { Cell::new((0, 0, 0)) };
}

/// Run `f` on the context if it exists and is not already borrowed.
fn with_ctx(f: impl FnOnce(&mut Ctx)) {
    CTX.with(|c| {
        if let Ok(mut g) = c.try_borrow_mut() {
            if let Some(ctx) = g.as_mut() {
                f(ctx);
            }
        }
    });
}

impl Ctx {
    fn inject(&mut self, e: &Edit) {
        self.injector.apply(e);
        if self.injector.restore_pending() {
            self.schedule_restore();
        }
    }

    /// (Re)start the one-shot clipboard-restore timer. Every new edit pushes the
    /// restore back, so a fast typist's original clipboard is restored once,
    /// after a short pause.
    fn schedule_restore(&mut self) {
        // SAFETY: thread timers with a callback; this thread runs a message loop.
        unsafe {
            if self.restore_timer != 0 {
                KillTimer(null_mut(), self.restore_timer);
            }
            self.restore_timer =
                SetTimer(null_mut(), 0, RESTORE_DELAY_MS, Some(restore_timer_proc));
        }
        // If SetTimer failed the restore stays pending; the next edit retries,
        // and the hook thread restores it on exit.
    }

    fn report_hook_state(&mut self, (kb, ms): (HHOOK, HHOOK)) {
        if kb.is_null() {
            if !self.reported_kb_failure {
                self.reported_kb_failure = true;
                (self.on_error)(KB_HOOK_FAILED.into());
            }
        } else {
            self.reported_kb_failure = false;
        }
        if ms.is_null() {
            if !self.reported_mouse_failure {
                self.reported_mouse_failure = true;
                (self.on_error)(MOUSE_HOOK_FAILED.into());
            }
        } else {
            self.reported_mouse_failure = false;
        }
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
        self.engine.set_vedic(self.state.is_vedic());
        self.engine.set_digits(self.state.is_digits());

        let vk16 = vk as u16;
        if is_pure_modifier(vk16) {
            return false; // Shift / Ctrl / Alt / AltGr / Win alone never end the word
        }
        // AltGr is Ctrl+Alt to Windows but it only *types* characters; real
        // shortcuts (Ctrl+C, Alt+Tab, Win+…) still pass straight through.
        if ((ctrl() || alt()) && !altgr()) || win() {
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

// ---------------------------------------------------------------------------
// Callbacks (all run on the hook thread, from its message loop)
// ---------------------------------------------------------------------------

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, lparam points to a KBDLLHOOKSTRUCT.
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        LAST_SEEN.set(kb.time);
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
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_MOUSE_LL with HC_ACTION, lparam points to an MSLLHOOKSTRUCT.
        let ms = &*(lparam as *const MSLLHOOKSTRUCT);
        LAST_SEEN.set(ms.time);
        let msg = wparam as u32;
        if matches!(
            msg,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        ) {
            // The caret probably moved: forget the current word.
            with_ctx(|ctx| ctx.engine.reset());
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

/// H3 risk reduction: keyboard focus moved to another control (e.g. a second
/// text box in the same dialog, or an autocomplete list). Only a *different*
/// focus target resets the word, so apps that re-announce the same field while
/// you type do not break words apart.
unsafe extern "system" fn focus_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _thread: u32,
    _time: u32,
) {
    let target = (hwnd as usize, id_object, id_child);
    if LAST_FOCUS.replace(target) != target {
        with_ctx(|ctx| ctx.engine.reset());
    }
}

/// C1: one-shot timer that puts the user's clipboard text back after the
/// target app has pasted ours.
unsafe extern "system" fn restore_timer_proc(_hwnd: HWND, _msg: u32, id: usize, _time: u32) {
    KillTimer(null_mut(), id);
    with_ctx(|ctx| {
        if ctx.restore_timer == id {
            ctx.restore_timer = 0;
        }
        ctx.injector.restore_clipboard();
    });
}

/// H2: Windows removes a low-level hook without telling us if it ever times
/// out. If Windows keeps seeing input that our hooks did not, reinstall them.
unsafe extern "system" fn watchdog_proc(_hwnd: HWND, _msg: u32, _id: usize, _time: u32) {
    let Some(last_input) = last_input_tick() else {
        return;
    };
    if !hooks_look_dead(last_input, LAST_SEEN.get()) {
        STRIKES.set(0);
        return;
    }
    let strikes = STRIKES.get() + 1;
    if strikes < WATCHDOG_STRIKES {
        STRIKES.set(strikes);
        return;
    }
    STRIKES.set(0);
    reinstall_hooks();
}

// ---------------------------------------------------------------------------
// Hook installation
// ---------------------------------------------------------------------------

fn last_input_tick() -> Option<u32> {
    let mut li = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `li` is a valid, correctly sized LASTINPUTINFO.
    (unsafe { GetLastInputInfo(&mut li) } != 0).then_some(li.dwTime)
}

/// Count everything up to now as "seen", so the watchdog starts from a clean slate.
fn mark_seen_now() {
    if let Some(t) = last_input_tick() {
        LAST_SEEN.set(t);
    }
}

unsafe fn install_hooks() -> (HHOOK, HHOOK) {
    let module = GetModuleHandleW(null());
    let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
    let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
    (kb, ms)
}

unsafe fn remove_hooks((kb, ms): (HHOOK, HHOOK)) {
    if !kb.is_null() {
        UnhookWindowsHookEx(kb);
    }
    if !ms.is_null() {
        UnhookWindowsHookEx(ms);
    }
}

unsafe fn reinstall_hooks() {
    remove_hooks(HOOKS.get()); // fails harmlessly for a hook Windows already removed
    let hooks = install_hooks();
    HOOKS.set(hooks);
    mark_seen_now();
    with_ctx(|ctx| {
        // Whatever was typed while the hooks were gone is unknown: start clean.
        ctx.engine.reset();
        ctx.held = [false; 256];
        ctx.swallowed = [false; 256];
        ctx.report_hook_state(hooks);
    });
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Start the hook thread. Returns immediately.
pub fn spawn(cfg: HookConfig) {
    let on_error = Arc::clone(&cfg.on_error);
    let started = std::thread::Builder::new()
        .name("kime-hook".into())
        .spawn(move || run_hook_thread(cfg));
    if let Err(e) = started {
        on_error(format!(
            "Could not start the keyboard thread ({e}), so typing will not convert. \
             Close and reopen the program."
        ));
    }
}

fn run_hook_thread(cfg: HookConfig) {
    let on_error = Arc::clone(&cfg.on_error);
    CTX.with(|c| {
        *c.borrow_mut() = Some(Ctx {
            engine: Engine::new(),
            epoch: cfg.state.epoch(),
            state: cfg.state,
            on_hotkey: cfg.on_hotkey,
            on_error: cfg.on_error,
            injector: Injector::with_deferred_restore(WinKeys, cfg.mode),
            foreground: 0,
            held: [false; 256],
            swallowed: [false; 256],
            restore_timer: 0,
            reported_kb_failure: false,
            reported_mouse_failure: false,
        })
    });

    // SAFETY: standard hook installation + message loop on this thread.
    unsafe {
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
        let hooks = install_hooks();
        if hooks.0.is_null() {
            remove_hooks(hooks); // do not leave half the hooks installed
            on_error(KB_HOOK_FAILED.into());
            return;
        }
        HOOKS.set(hooks);
        mark_seen_now();
        with_ctx(|ctx| ctx.report_hook_state(hooks)); // warns if only the mouse hook failed

        let focus = SetWinEventHook(
            EVENT_OBJECT_FOCUS,
            EVENT_OBJECT_FOCUS,
            null_mut(),
            Some(focus_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        let watchdog = SetTimer(null_mut(), 0, WATCHDOG_MS, Some(watchdog_proc));

        // MSG is a plain C struct; all-zero is a valid value for it.
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if watchdog != 0 {
            KillTimer(null_mut(), watchdog);
        }
        if !focus.is_null() {
            UnhookWinEvent(focus);
        }
        remove_hooks(HOOKS.get());
        with_ctx(|ctx| ctx.injector.restore_clipboard());
    }
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
        if h.is_null() {
            return true; // can't tell; don't block startup
        }
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lone_modifiers_do_not_end_the_word() {
        for vk in [
            VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_CAPITAL, VK_NUMLOCK, VK_SCROLL, VK_CONTROL,
            VK_LCONTROL, VK_RCONTROL, VK_MENU, VK_LMENU, VK_RMENU, VK_LWIN, VK_RWIN,
        ] {
            assert!(is_pure_modifier(vk), "vk {vk:#x}");
        }
        for vk in [VK_BACK, 0x41 /* A */, VK_K as u16, 0x20 /* Space */, 0x0D /* Enter */] {
            assert!(!is_pure_modifier(vk), "vk {vk:#x}");
        }
    }

    #[test]
    fn watchdog_decision() {
        // Hooks are current, or only slightly behind: fine.
        assert!(!hooks_look_dead(5_000, 5_000));
        assert!(!hooks_look_dead(5_900, 5_000));
        // Our hooks saw something newer than GetLastInputInfo: never "dead".
        assert!(!hooks_look_dead(5_000, 9_000));
        // Clearly behind.
        assert!(hooks_look_dead(9_000, 5_000));
        // Across the 49.7-day wrap of the tick counter.
        assert!(hooks_look_dead(500, u32::MAX - 1_000));
        assert!(!hooks_look_dead(100, u32::MAX - 100));
    }
}
