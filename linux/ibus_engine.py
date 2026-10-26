#!/usr/bin/python3
# ibus_engine.py - the IBus engine "Kannada Vedic".
#
# IBus starts this program itself (see the component file written by install.sh)
# with --ibus. For a quick manual try without installing, run it with
# --standalone from a terminal while ibus-daemon is running.
#
# All typing rules live in kvengine.py / keyhandler.py (no IBus there). This file
# only connects them to IBus: key events in, preedit and committed text out, plus
# one panel menu switch (Vedic mode).

import logging
import os
import sys
import traceback

HERE = os.path.dirname(os.path.realpath(__file__))
sys.path.insert(0, HERE)

import gi

gi.require_version("IBus", "1.0")
from gi.repository import GLib, GObject, IBus  # noqa: E402

import kvsettings  # noqa: E402
from keyhandler import KeyHandler  # noqa: E402

ENGINE_NAME = "kannada-vedic"
BUS_NAME = "org.freedesktop.IBus.KannadaVedic"
VERSION = "0.5.0-linux"


def _setup_log():
    base = os.environ.get("XDG_CACHE_HOME") or os.path.join(os.path.expanduser("~"), ".cache")
    d = os.path.join(base, "kannada-ime")
    try:
        os.makedirs(d, exist_ok=True)
        path = os.path.join(d, "ibus.log")
        if os.path.exists(path) and os.path.getsize(path) > 512 * 1024:
            os.remove(path)
        logging.basicConfig(filename=path, level=logging.INFO,
                            format="%(asctime)s %(levelname)s %(message)s")
    except OSError:
        logging.basicConfig(level=logging.WARNING)


class KannadaVedicEngine(IBus.Engine):
    __gtype_name__ = "KannadaVedicEngine"

    def __init__(self):
        super().__init__()
        self._vedic = kvsettings.load_vedic()
        self._digits = kvsettings.load_digits()
        self._h = KeyHandler(self._commit, self._preedit, self._vedic, digits=self._digits)
        self._props = None
        self._prop_vedic = None
        self._prop_digits = None
        logging.info("engine created, vedic=%s", self._vedic)

    # -- output ------------------------------------------------------------
    def _commit(self, text):
        self.commit_text(IBus.Text.new_from_string(text))

    def _preedit(self, text):
        if text:
            t = IBus.Text.new_from_string(text)
            t.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE, 0, len(text))
            self.update_preedit_text(t, len(text), True)
        else:
            self.hide_preedit_text()

    # -- panel menu --------------------------------------------------------
    def _make_props(self):
        self._props = IBus.PropList()
        self._prop_vedic = IBus.Property(
            key="vedic",
            prop_type=IBus.PropType.TOGGLE,
            label=IBus.Text.new_from_string("Vedic mode (accents and ꣳ)"),
            icon="",
            tooltip=IBus.Text.new_from_string("On: the accent keys ' \" _ ^ and ꣳ work. Off: they type normally."),
            sensitive=True,
            visible=True,
            state=IBus.PropState.CHECKED if self._vedic else IBus.PropState.UNCHECKED,
            sub_props=None,
        )
        self._props.append(self._prop_vedic)
        self._prop_digits = IBus.Property(
            key="digits",
            prop_type=IBus.PropType.TOGGLE,
            label=IBus.Text.new_from_string("Kannada digits (೧ ೨ ೩)"),
            icon="",
            tooltip=IBus.Text.new_from_string("On: the number keys type Kannada digits. Off: 1 2 3 as usual."),
            sensitive=True,
            visible=True,
            state=IBus.PropState.CHECKED if self._digits else IBus.PropState.UNCHECKED,
            sub_props=None,
        )
        self._props.append(self._prop_digits)

    def do_property_activate(self, prop_name, state):
        try:
            if prop_name == "vedic":
                self._vedic = state == IBus.PropState.CHECKED
                self._h.set_vedic(self._vedic)
                kvsettings.save_vedic(self._vedic)
                if self._prop_vedic is not None:
                    self._prop_vedic.set_state(state)
                    self.update_property(self._prop_vedic)
                logging.info("vedic mode set to %s", self._vedic)
            elif prop_name == "digits":
                self._digits = state == IBus.PropState.CHECKED
                self._h.set_digits(self._digits)
                kvsettings.save_digits(self._digits)
                if self._prop_digits is not None:
                    self._prop_digits.set_state(state)
                    self.update_property(self._prop_digits)
                logging.info("kannada digits set to %s", self._digits)
        except Exception:
            logging.error("property_activate failed:\n%s", traceback.format_exc())

    # -- events ------------------------------------------------------------
    def do_process_key_event(self, keyval, keycode, state):
        try:
            return self._h.handle(keyval, state)
        except Exception:
            logging.error("key event failed:\n%s", traceback.format_exc())
            try:
                self._h.discard()
            except Exception:
                pass
            return False

    def do_focus_in(self):
        try:
            # The settings file may have been changed by the Windows-style tray
            # switch or by hand; pick it up whenever a text field gets focus.
            v = kvsettings.load_vedic(self._vedic)
            if v != self._vedic:
                self._vedic = v
                self._h.set_vedic(v)
            d = kvsettings.load_digits(self._digits)
            if d != self._digits:
                self._digits = d
                self._h.set_digits(d)
            self._make_props()
            self.register_properties(self._props)
        except Exception:
            logging.error("focus_in failed:\n%s", traceback.format_exc())

    def do_focus_out(self):
        self._h.flush()

    def do_reset(self):
        self._h.discard()

    def do_enable(self):
        self._h.discard()

    def do_disable(self):
        self._h.flush()


class App:
    def __init__(self, exec_by_ibus):
        self._loop = GLib.MainLoop()
        self._bus = IBus.Bus()
        if not self._bus.is_connected():
            logging.error("cannot connect to ibus-daemon")
            sys.stderr.write("Cannot connect to ibus-daemon. Is IBus running? Try: ibus-daemon -drx\n")
            sys.exit(1)
        self._bus.connect("disconnected", lambda b: self._loop.quit())
        self._factory = IBus.Factory.new(self._bus.get_connection())
        self._factory.add_engine(ENGINE_NAME, GObject.type_from_name("KannadaVedicEngine"))
        if exec_by_ibus:
            self._bus.request_name(BUS_NAME, 0)
        else:
            comp = IBus.Component(
                name=BUS_NAME,
                description="Kannada Vedic phonetic input",
                version=VERSION,
                license="MIT OR Apache-2.0",
                author="Kannada Vedic IME",
                homepage="https://github.com/fellowshipofhogwarts/kannada-vedic-ime",
                command_line=os.path.join(HERE, "ibus_engine.py") + " --ibus",
                textdomain="kannada-vedic-ibus",
            )
            comp.add_engine(IBus.EngineDesc(
                name=ENGINE_NAME,
                longname="Kannada Vedic",
                description="Phonetic Kannada typing with Vedic accents",
                language="kn",
                license="MIT OR Apache-2.0",
                author="Kannada Vedic IME",
                icon="",
                layout="default",
            ))
            self._bus.register_component(comp)

    def run(self):
        logging.info("kannada-vedic ibus engine %s started (pid %d)", VERSION, os.getpid())
        self._loop.run()


def main():
    _setup_log()
    if "--version" in sys.argv:
        print(VERSION)
        return
    IBus.init()
    App(exec_by_ibus="--ibus" in sys.argv).run()


if __name__ == "__main__":
    main()
