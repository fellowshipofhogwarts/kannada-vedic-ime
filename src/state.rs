//! state.rs — the tiny bit of state shared between the hook thread and the UI
//! thread: "is the engine ON", "is Vedic mode on", plus an epoch counter so the hook thread knows to
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
    vedic: AtomicBool,
    digits: AtomicBool,
    epoch: AtomicU32,
}

impl AppState {
    pub fn new(active: bool, vedic: bool, digits: bool) -> Arc<Self> {
        Arc::new(Self {
            active: AtomicBool::new(active),
            vedic: AtomicBool::new(vedic),
            digits: AtomicBool::new(digits),
            epoch: AtomicU32::new(0),
        })
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Vedic mode: accent keys and the agama anusvara are active.
    #[inline]
    pub fn is_vedic(&self) -> bool {
        self.vedic.load(Ordering::Relaxed)
    }

    /// The hook thread notices the change on its next key and flushes its word.
    pub fn set_vedic(&self, on: bool) {
        self.vedic.store(on, Ordering::Relaxed);
    }

    /// Kannada digits (೧೨೩) instead of English digits.
    #[inline]
    pub fn is_digits(&self) -> bool {
        self.digits.load(Ordering::Relaxed)
    }

    pub fn set_digits(&self, on: bool) {
        self.digits.store(on, Ordering::Relaxed);
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
        let s = AppState::new(true, true, true);
        assert!(s.is_active());
        let e0 = s.epoch();
        assert!(!s.toggle());
        assert!(!s.is_active());
        assert_ne!(s.epoch(), e0);
        assert!(s.toggle());
    }

    #[test]
    fn digits_flag_is_independent() {
        let s = AppState::new(true, true, true);
        assert!(s.is_digits());
        s.set_digits(false);
        assert!(!s.is_digits() && s.is_vedic() && s.is_active());
    }

    #[test]
    fn vedic_flag_is_independent_of_on_off() {
        let s = AppState::new(true, true, true);
        assert!(s.is_vedic());
        s.set_vedic(false);
        assert!(!s.is_vedic());
        assert!(s.is_active());
        s.toggle();
        assert!(!s.is_vedic(), "turning typing off must not change Vedic mode");
        assert!(!AppState::new(false, false, true).is_vedic());
    }
}
