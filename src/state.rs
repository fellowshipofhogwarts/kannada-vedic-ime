//! state.rs — the tiny bit of state shared between the hook thread and the UI
//! thread: "is the engine ON", "is Vedic mode on", "Kannada digits", plus an
//! epoch counter.
//!
//! The epoch changes only on ON/OFF (`toggle`), so the hook thread flushes its
//! typing buffer when typing was switched behind its back (menu click);
//! otherwise a half-typed word could leak across an ON/OFF flip. Vedic mode and
//! digits need no epoch: the hook passes both to the engine before every key,
//! and the engine ends the word itself when either changes.
//!
//! `Relaxed` ordering is enough: each flag is read on its own, and the worst
//! case of a late read is one key handled with the old setting.

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

    /// The hook thread passes this to the engine on its next key; the engine
    /// ends the current word if the value changed.
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
