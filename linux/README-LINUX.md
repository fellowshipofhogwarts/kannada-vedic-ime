# Kannada Vedic for Linux (experimental)

An IBus input method for Linux Mint and Ubuntu (X11). It follows the same typing rules as the
Windows app and the web typer, and it is tested only on Linux Mint 22 XFCE with LibreOffice Writer so far.
Please report what you find.

**Differences from Windows:** the word you are typing is underlined and is finished when you press
Space, Enter or punctuation (this is how IBus works). `g`M` gives the plain ಗ್ಂ here, not the single sign ꣳ.

## Install

**Option 1: the .deb file** (from the Releases page).
Double-click `kannada-vedic-ibus_0.5.0_all.deb` and install it, or run
`sudo apt install ./kannada-vedic-ibus_0.5.0_all.deb`. Then run `ibus restart`.

**Option 2: the script** (this folder, no administrator rights needed).
`sudo apt install ibus python3-gi gir1.2-ibus-1.0` once, then `./install.sh`.

Do not use both. If you used the script before, run `./uninstall.sh` first.

## Set up

1. Run `ibus-setup`, tab *Input Method*, *Add*, *Kannada*, *Kannada Vedic*.
2. Switch input methods with `Super+Space`.
3. In LibreOffice Writer open the template: File > New > Templates > My Templates > *Kannada-Vedic-Writer*.
   With the .deb, first run `kannada-vedic-writer-template` once. The template sets the font Arkavathi for
   Western, Asian **and** complex text. Writer needs all three, otherwise the Vedic marks look wrong.
4. Type `namaste` and press Space: ನಮಸ್ತೆ.

The tray-style switches (Vedic mode, Kannada digits) are in the IBus panel menu.
The settings file is `~/.config/kannada-ime/settings.txt` (`vedic=1`, `kannada_digits=1`).

## If a mark looks wrong

Run `fc-list | grep -i -E "kaveri|arkavathi"`. You should see one Arkavathi line and no Kaveri line
(Kaveri is the old name; uninstall it). Then run `./diagnose.sh` and attach the output to a bug report.
The log is `~/.cache/kannada-ime/ibus.log`.

## For developers

* `kvengine.py` is a port of `src/engine.rs`. `kvtables.py` is generated from `src/mapping.rs`:
  `python3 tools/gen_tables.py ../src/mapping.rs > kvtables.py`.
* Tests: `python3 tests/test_engine.py` and `python3 tests/test_glue.py`.
  `tests/parity.py` prints code points in the format of a Rust `dump` binary, to compare both engines
  (0 differences on 30,000 random inputs for 0.5.0, with `agama=True`).
* Build the package: `linux/build_deb.sh` (needs `dpkg-deb`).
* Why Python and not Rust: IBus engines talk to the daemon through GObject, which Python does well; the
  logic is small and the parity test keeps it identical.
