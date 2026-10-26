#!/usr/bin/env python3
"""Tests for kvengine / keyhandler. Mirrors the important cases of engine.rs."""
import os, sys, unittest
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from kvengine import render_with, Core
from keyhandler import KeyHandler, KEY_BackSpace, CONTROL_MASK, RELEASE_MASK, KEY_dead_grave, KEY_dead_circumflex

U, A, D = "॑", "॒", "᳚"


class Screen:
    """A fake text field: committed text + the underlined preedit at the end."""
    def __init__(self, vedic=True):
        self.text = ""
        self.pre = ""
        self.h = KeyHandler(self._commit, self._pre, vedic)

    def _commit(self, t):
        self.text += t

    def _pre(self, t):
        self.pre = t

    def key(self, ch):
        kv = ord(ch)
        if not self.h.handle(kv, 0):
            self.text += ch          # the application receives the key itself

    def bs(self):
        if not self.h.handle(KEY_BackSpace, 0):
            self.text = self.text[:-1]

    def type(self, s):
        for c in s:
            self.key(c)
        return self

    def shown(self):
        return self.text + self.pre


def typed(s, vedic=True):
    return Screen(vedic).type(s).shown()


class Rules(unittest.TestCase):
    def test_words(self):
        for k, v in [("namaste", "ನಮಸ್ತೆ"), ("kannaDa", "ಕನ್ನಡ"), ("saMskRRita", "ಸಂಸ್ಕೃತ"),
                     ("tatva", "ತತ್ವ"), ("jnaana", "ಜ್ಞಾನ"), ("xetra", "ಕ್ಷೆತ್ರ"), ("Baarata", "ಭಾರತ"),
                     ("Saale", "ಶಾಲೆ"), ("Sha", "ಷ"), ("aM", "ಅಂ"), ("aH", "ಅಃ"), (":ga", "ಙ"), (":ja", "ಞ"),
                     ("Siva", "ಶಿವ"), ("SaH", "ಶಃ"), ("kSha", "ಕ್ಷ"), ("OM", "ಓಂ")]:
            self.assertEqual(typed(k), v, k)

    def test_space_and_punctuation_end_the_word(self):
        self.assertEqual(typed("ka ka"), "ಕ ಕ")
        self.assertEqual(typed("ka a"), "ಕ ಅ")
        self.assertEqual(typed("q"), "q")
        self.assertEqual(typed("1.5"), "೧.೫")

    def test_accents(self):
        self.assertEqual(typed("agni'"), "ಅಗ್ನಿ" + U)
        self.assertEqual(typed("ka_"), "ಕ" + A)
        self.assertEqual(typed('ka"'), "ಕ" + D)
        self.assertEqual(typed("ka'_"), "ಕ" + A + U)
        self.assertEqual(typed("agni'mILE"), "ಅಗ್ನಿ" + U + "ಮೀಳೇ")
        self.assertEqual(typed("'"), "'")
        self.assertEqual(typed("q'"), "q'")

    def test_caret_upgrade(self):
        s = Screen().type("ka^")
        self.assertEqual(s.shown(), "ಕ" + U)
        s.key("^")
        self.assertEqual(s.shown(), "ಕ" + D)

    def test_break_key_and_agama(self):
        self.assertEqual(typed("g`M"), "ಗ್ಂ")
        self.assertEqual(typed("g`M'"), "ಗ್ಂ" + U)
        self.assertEqual(typed("gM"), "ಗಂ")
        self.assertEqual(typed("k`M"), "ಕ್ಂ")
        self.assertEqual(typed("c`H"), "ಚ್ಃ")
        self.assertEqual(typed("c``H"), typed("c`H"))
        self.assertEqual(typed("s`ha"), "ಸ್ಹ")

    def test_vedic_off(self):
        self.assertEqual(typed("ka'", vedic=False), "ಕ'")
        self.assertEqual(typed("don't", vedic=False), "ದೊನ್'ತ್")
        self.assertEqual(typed("g`M", vedic=False), "ಗ್ಂ")
        self.assertEqual(typed("namaste", vedic=False), "ನಮಸ್ತೆ")

    def test_backspace_undoes_one_key(self):
        s = Screen().type("kh")
        self.assertEqual(s.shown(), "ಖ್")
        s.bs()
        self.assertEqual(s.shown(), "ಕ್")
        s.bs()
        self.assertEqual(s.shown(), "")
        s = Screen().type("ka'ra")
        s.bs(); s.bs()
        self.assertEqual(s.shown(), "ಕ" + U)
        # empty buffer: the backspace goes to the application
        s = Screen(); s.text = "xyz"; s.bs()
        self.assertEqual(s.shown(), "xy")

    def test_shortcuts_and_release_pass_through(self):
        s = Screen().type("ka")
        self.assertFalse(s.h.handle(ord("c"), CONTROL_MASK))   # Ctrl+C: not ours
        self.assertEqual(s.text, "ಕ")                          # ...but the word was committed first
        self.assertEqual(s.pre, "")
        s = Screen().type("ka")
        self.assertFalse(s.h.handle(ord("a"), RELEASE_MASK))
        self.assertEqual(s.shown(), "ಕ")

    def test_shift_alone_does_not_end_the_word(self):
        s = Screen().type("k")
        self.assertFalse(s.h.handle(0xFFE1, 0))                # Shift_L
        self.assertEqual(s.pre, "ಕ್")                          # still in progress

    def test_german_dead_keys(self):
        s = Screen().type("c")
        self.assertTrue(s.h.handle(KEY_dead_grave, 0))         # break key
        s.type("H")
        self.assertEqual(s.shown(), "ಚ್ಃ")
        s = Screen().type("ka")
        self.assertTrue(s.h.handle(KEY_dead_circumflex, 0))    # Udatta
        self.assertEqual(s.shown(), "ಕ" + U)

    def test_enter_and_arrows_commit(self):
        s = Screen().type("ka")
        self.assertFalse(s.h.handle(0xFF0D, 0))                # Return
        self.assertEqual((s.text, s.pre), ("ಕ", ""))

    def test_vedic_switch_commits_the_word(self):
        s = Screen().type("ka")
        s.h.set_vedic(False)
        self.assertEqual((s.text, s.pre), ("ಕ", ""))
        s.type("'")
        self.assertEqual(s.shown(), "ಕ'")

    def test_long_input_never_crashes(self):
        out = typed("ka" * 200)
        self.assertEqual(out.count("ಕ"), 200)

    def test_every_table_token_renders(self):
        from kvengine import TABLE
        for k in TABLE:
            render_with(k, True); render_with(k, False)
        self.assertEqual(len(TABLE), 90)


class New050(unittest.TestCase):
    def test_new_keys(self):
        t = lambda x: render_with(x)[0]
        self.assertEqual(t("ZZ"), "ಽಽ")
        self.assertEqual(t(":h"), "ೱ"); self.assertEqual(t(":H"), "ೲ")
        self.assertEqual(t("zha"), "ೞ"); self.assertEqual(t(":l"), "ಌ")
        self.assertEqual(t("k:L"), "ಕೣ"); self.assertEqual(t("ka:M"), "ಕಁ")
        self.assertEqual(t("jQi"), "ಜ಼ಿ"); self.assertEqual(t("kaQ"), "ಕQ")

    def test_digits_and_keypad(self):
        out = []
        h = KeyHandler(out.append, lambda t: None, True)
        self.assertTrue(h.handle(ord("7"), 0)); self.assertEqual(out, ["೭"])
        self.assertTrue(h.handle(0xFFB3, 0)); self.assertEqual(out, ["೭", "೩"])
        h.set_digits(False)
        self.assertFalse(h.handle(ord("7"), 0))

    def test_digit_flushes_word(self):
        sc = Screen()
        for c in "ka": sc.key(c)
        sc.key("1")
        self.assertEqual(sc.shown(), "ಕ೧")


if __name__ == "__main__":
    unittest.main(verbosity=1)
