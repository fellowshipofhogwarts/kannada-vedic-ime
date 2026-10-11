//! hook/filter.rs — the "is this event ours?" decision, as a pure function.
//!
//! Lives outside `win.rs` so it compiles (and is unit-tested) on every OS even
//! though only the Windows hook calls it. This is the guard that prevents an
//! infinite typing loop: every key the injector synthesises with `SendInput`
//! comes straight back through our own `WH_KEYBOARD_LL` hook, and must be passed
//! on untouched instead of being fed to the engine again.

/// `KBDLLHOOKSTRUCT.flags` bit 4 (0x10): the event was injected, by any process.
/// (Same value as `windows_sys::...::LLKHF_INJECTED`.)
pub const LLKHF_INJECTED: u32 = 0x10;

/// `KBDLLHOOKSTRUCT.flags` bit 1 (0x02): injected by a process running at a
/// lower integrity level. Windows sets this *in addition to* `LLKHF_INJECTED`,
/// so testing it is belt and braces.
pub const LLKHF_LOWER_IL_INJECTED: u32 = 0x02;

/// Value we put in `dwExtraInfo` of every event we synthesise ("KIME").
pub const INJECT_TAG: usize = 0x4B49_4D45;

/// True if the event must bypass the engine: it was injected by *anyone*
/// (flag test) or carries our own tag (survives even if a driver or remote
/// session strips the flag).
///
/// Trade-off, by design: because the flag test covers every injected event,
/// keys sent by other software (AutoHotkey, on-screen keyboards, remote-desktop
/// clients) are also never transliterated. To transliterate those too, drop the
/// flag test and rely on `INJECT_TAG` alone.
#[inline]
pub fn is_injected(flags: u32, extra_info: usize) -> bool {
    flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED) != 0 || extra_info == INJECT_TAG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_values_match_the_win32_header() {
        assert_eq!(LLKHF_INJECTED, 0x10);
        assert_eq!(LLKHF_LOWER_IL_INJECTED, 0x02);
    }

    #[test]
    fn physical_keys_are_not_injected() {
        // Typical physical event: no injected bits (0x20 = ALT context, 0x80 = up).
        assert!(!is_injected(0, 0));
        assert!(!is_injected(0x20, 0));
        assert!(!is_injected(0x80, 0));
        // A foreign dwExtraInfo (some drivers set one) is not ours.
        assert!(!is_injected(0, 0xDEAD_BEEF));
    }

    #[test]
    fn simulated_events_are_ignored_by_flag() {
        assert!(is_injected(LLKHF_INJECTED, 0));
        assert!(is_injected(LLKHF_INJECTED | 0x80, 0)); // key-up
        assert!(is_injected(LLKHF_LOWER_IL_INJECTED, 0));
        assert!(is_injected(LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED, 0));
    }

    #[test]
    fn our_tag_is_ignored_even_without_the_flag() {
        assert!(is_injected(0, INJECT_TAG));
    }
}
