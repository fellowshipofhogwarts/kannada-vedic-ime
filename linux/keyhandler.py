"""keyhandler.py - turns IBus key events into engine actions. No IBus import.

`handle(keyval, state)` returns True when the key was used up (IBus must not pass
it on) and False when the application should receive it. The word being typed
is sent to `preedit` (underlined text); `commit` inserts finished text.
"""
from kvengine import Core, MAX_BUFFER, kannada_digit

# GDK/X11 keysyms and IBus modifier bits (stable values).
KEY_BackSpace = 0xFF08
KEY_Shift_L, KEY_Hyper_R = 0xFFE1, 0xFFEE      # Shift/Control/Caps/Meta/Alt/Super/Hyper
KEY_Num_Lock, KEY_ISO_Level3_Shift = 0xFF7F, 0xFE03
KEY_dead_grave, KEY_dead_circumflex = 0xFE50, 0xFE52
KEY_dead_first, KEY_dead_last = 0xFE50, 0xFE5F
KEY_KP_0, KEY_KP_9 = 0xFFB0, 0xFFB9

SHIFT_MASK = 1 << 0
CONTROL_MASK = 1 << 2
MOD1_MASK = 1 << 3      # Alt
SUPER_MASK = 1 << 26
HYPER_MASK = 1 << 27
META_MASK = 1 << 28
RELEASE_MASK = 1 << 30
SHORTCUT_MASK = CONTROL_MASK | MOD1_MASK | SUPER_MASK | HYPER_MASK | META_MASK


class KeyHandler:
    def __init__(self, commit, preedit, vedic=True, log=None, digits=True):
        self._commit = commit
        self._preedit = preedit
        self._log = log or (lambda *a: None)
        self.core = Core(vedic)
        self.digits = digits

    # -- helpers -----------------------------------------------------------
    def _show(self):
        self._preedit(self.core.text)

    def flush(self):
        """Commit the word in progress (if any) and clear it."""
        text = self.core.text
        self.core.reset()
        if text:
            self._commit(text)
        self._preedit("")

    def discard(self):
        self.core.reset()
        self._preedit("")

    def set_vedic(self, on):
        if self.core.vedic != on:
            self.flush()
            self.core.set_vedic(on)

    def set_digits(self, on):
        if self.digits != on:
            self.flush()
            self.digits = on

    # -- the one entry point -----------------------------------------------
    def handle(self, keyval, state):
        if state & RELEASE_MASK:
            return False
        # Modifier keys on their own never end a word (Shift before a capital).
        if KEY_Shift_L <= keyval <= KEY_Hyper_R or keyval in (KEY_Num_Lock, KEY_ISO_Level3_Shift):
            return False
        if state & SHORTCUT_MASK:
            self.flush()
            return False
        if keyval == KEY_BackSpace:
            if self.core.on_backspace() == "consumed":
                self._show()
                return True
            return False
        if self.digits:
            d = None
            if 0x30 <= keyval <= 0x39:
                d = kannada_digit(chr(keyval))
            elif KEY_KP_0 <= keyval <= KEY_KP_9:
                d = kannada_digit(chr(0x30 + keyval - KEY_KP_0))
            if d:
                self.flush()
                self._commit(d)
                return True
        ch = None
        if 0x20 <= keyval <= 0x7E:
            ch = chr(keyval)
        elif keyval == KEY_dead_grave:        # German: Shift + acute/grave key
            ch = "`"
        elif keyval == KEY_dead_circumflex:   # German: the ^ key
            ch = "^"
        elif KEY_dead_first <= keyval <= KEY_dead_last:
            self.flush()
            return False
        if ch is None:                        # Enter, Tab, arrows, Esc, keypad, F-keys...
            self.flush()
            return False
        if len(self.core.buf) >= MAX_BUFFER:   # a very long word: commit what we have
            self.flush()
        if self.core.on_char(ch) == "consumed":
            self._show()
            return True
        self.flush()                          # not ours: finish the word, key passes on
        return False
