#!/usr/bin/env python3
"""Smoke test of ibus_engine.py with a FAKE gi/IBus module.

This does not prove the real IBus API accepts every call (that is what the test
on a real Mint machine is for). It proves the file parses, the engine class is
wired to KeyHandler correctly, and the panel switch and settings file work.
"""
import os, sys, types, tempfile, unittest

HERE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
sys.path.insert(0, HERE)


class FakeEngine:
    def __init__(self):
        self.committed, self.pre, self.hidden, self.props = [], None, 0, None
    def commit_text(self, t): self.committed.append(t.s)
    def update_preedit_text(self, t, cur, vis): self.pre = t.s
    def hide_preedit_text(self): self.pre = ""; self.hidden += 1
    def register_properties(self, p): self.props = p
    def update_property(self, p): self.updated = p


class Text:
    def __init__(self, s): self.s = s; self.attrs = []
    @classmethod
    def new_from_string(cls, s): return cls(s)
    def append_attribute(self, *a): self.attrs.append(a)


class Prop:
    def __init__(self, **kw): self.__dict__.update(kw)
    def set_state(self, s): self.state = s


class PropList(list):
    def append(self, x): super().append(x)


IBus = types.SimpleNamespace(
    Engine=FakeEngine, Text=Text, Property=Prop, PropList=PropList,
    AttrType=types.SimpleNamespace(UNDERLINE=1), AttrUnderline=types.SimpleNamespace(SINGLE=1),
    PropType=types.SimpleNamespace(TOGGLE=1), PropState=types.SimpleNamespace(CHECKED=1, UNCHECKED=0),
    Bus=object, Factory=object, Component=object, EngineDesc=object, init=lambda: None)
gi = types.ModuleType("gi"); gi.require_version = lambda *a: None
repo = types.ModuleType("gi.repository"); repo.IBus = IBus
repo.GLib = types.SimpleNamespace(MainLoop=object); repo.GObject = types.SimpleNamespace(type_from_name=lambda n: n)
sys.modules.update({"gi": gi, "gi.repository": repo})

tmp = tempfile.mkdtemp()
os.environ["XDG_CONFIG_HOME"] = tmp
os.environ["XDG_CACHE_HOME"] = tmp
import ibus_engine  # noqa: E402


class Glue(unittest.TestCase):
    def test_typing_and_commit(self):
        e = ibus_engine.KannadaVedicEngine()
        for ch in "namaste":
            self.assertTrue(e.do_process_key_event(ord(ch), 0, 0))
        self.assertEqual(e.pre, "ನಮಸ್ತೆ")
        self.assertFalse(e.do_process_key_event(0x20, 0, 0))     # space passes on
        self.assertEqual(e.committed, ["ನಮಸ್ತೆ"])
        self.assertEqual(e.pre, "")

    def test_panel_switch_saves_and_applies(self):
        e = ibus_engine.KannadaVedicEngine()
        e.do_focus_in()
        self.assertEqual(len(e.props), 2)   # Vedic mode + Kannada digits
        e.do_property_activate("vedic", 0)                        # unchecked = Vedic off
        e.do_process_key_event(ord("k"), 0, 0); e.do_process_key_event(ord("a"), 0, 0)
        self.assertFalse(e.do_process_key_event(ord("'"), 0, 0))  # apostrophe passes on, as plain text
        self.assertEqual(e.committed[-1], "ಕ")
        text = open(os.path.join(tmp, "kannada-ime", "settings.txt"), encoding="utf-8").read()
        self.assertIn("vedic=0", text)
        e2 = ibus_engine.KannadaVedicEngine()
        self.assertFalse(e2._vedic)                               # remembered
        e.do_property_activate("vedic", 1)
        self.assertIn("vedic=1", open(os.path.join(tmp, "kannada-ime", "settings.txt"), encoding="utf-8").read())

    def test_digits_switch(self):
        e = ibus_engine.KannadaVedicEngine()
        e.do_focus_in()
        self.assertTrue(e.do_process_key_event(ord("5"), 0, 0))
        self.assertEqual(e.committed[-1], "೫")
        e.do_property_activate("digits", 0)
        self.assertFalse(e.do_process_key_event(ord("5"), 0, 0))
        self.assertIn("kannada_digits=0", open(os.path.join(tmp, "kannada-ime", "settings.txt"), encoding="utf-8").read())
        e.do_property_activate("digits", 1)

    def test_settings_keep_other_keys(self):
        p = os.path.join(tmp, "kannada-ime", "settings.txt")
        os.makedirs(os.path.dirname(p), exist_ok=True)
        open(p, "w").write("update_check=0\nvedic=1\n")
        import kvsettings
        kvsettings.save_vedic(False)
        t = open(p).read()
        self.assertIn("update_check=0", t); self.assertIn("vedic=0", t)

    def test_focus_out_commits(self):
        e = ibus_engine.KannadaVedicEngine()
        for ch in "ka":
            e.do_process_key_event(ord(ch), 0, 0)
        e.do_focus_out()
        self.assertEqual(e.committed, ["ಕ"])


if __name__ == "__main__":
    unittest.main(verbosity=1)
