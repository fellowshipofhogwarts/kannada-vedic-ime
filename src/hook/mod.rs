//! hook — global keyboard interception, one backend per OS.
//!
//! Both backends implement the same contract:
//!
//! * run on their own thread (the UI thread must stay free for the tray),
//! * while the engine is OFF, change nothing except watching for the hotkey,
//! * while ON, swallow phonetic keys and inject the engine's edits,
//! * ignore events they injected themselves,
//! * flush the engine on mouse clicks, focus changes and ON/OFF flips,
//! * on Ctrl+Shift+K flip [`AppState`] and call `on_hotkey` so the UI thread can
//!   update the tray icon and show a toast.

use crate::inject::Mode;
use crate::state::AppState;
use std::sync::Arc;

pub struct HookConfig {
    pub state: Arc<AppState>,
    pub mode: Mode,
    /// Called on the hook thread right after the hotkey flipped the state.
    /// Must be quick and non-blocking (it just posts an event to the UI loop).
    pub on_hotkey: Arc<dyn Fn() + Send + Sync>,
}

// Pure and OS-independent so it can be unit-tested everywhere; only the Windows
// hook uses it.
#[cfg_attr(not(windows), allow(dead_code))]
mod filter;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{single_instance, spawn};

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::spawn;

#[cfg(all(unix, not(target_os = "macos")))]
pub fn single_instance() -> bool {
    true
}

#[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
compile_error!("kannada_ime supports Windows and Linux only");
