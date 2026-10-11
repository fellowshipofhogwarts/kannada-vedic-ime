//! state.rs — the tiny bit of state shared between the hook thread and the UI
//! thread: "is the engine ON", plus an epoch counter so the hook thread knows to
//! flush its typing buffer whenever the state changed behind its back (menu
//! click) — otherwise a half-typed word could leak across an ON/OFF flip.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleSource {
    Hotkey,
    Menu,
}

#[derive(Debug)]
pub struct AppState {
    active: AtomicBool,
    epoch: AtomicU32,
}

impl AppState {
    pub fn new(active: bool) -> Arc<Self> {
        Arc::new(Self {
            active: AtomicBool::new(active),
            epoch: AtomicU32::new(0),
        })
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn epoch(&self) -> u32 {
        self.epoch.load(Ordering::Relaxed)
    }

    /// Flip the state; returns the *new* value.
    pub fn toggle(&self) -> bool {
        let new = !self.active.fetch_xor(true, Ordering::Relaxed);
        self.epoch.fetch_add(1, Ordering::Relaxed);
        new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips_and_bumps_epoch() {
        let s = AppState::new(true);
        assert!(s.is_active());
        let e0 = s.epoch();
        assert!(!s.toggle());
        assert!(!s.is_active());
        assert_ne!(s.epoch(), e0);
        assert!(s.toggle());
    }
}
