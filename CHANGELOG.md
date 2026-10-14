# Changelog

## 0.4.0

### Typing
* **Capital letters for the aspirates:** `K` ಖ, `G` ಘ, `J` ಝ, `TH` ಠ, `DH` ಢ, `P` ಫ, `B` ಭ, `S` ಶ,
  `Sh` / `SH` ಷ. The older spellings (`kh gh jh Th Dh ph bh sh`) still work. `th` and `dh` stay ಥ and ಧ.
  Old `S` = ಷ is gone (it is ಶ now). A visarga after ಶ ಟ ಡ is typed `SaH`, `TaH`, `DaH`.
* **Half consonant before `M` / `H`:** `` k`M `` → ಕ್ಂ, `` c`H `` → ಚ್ಃ. Without the break key it is
  still the full letter (`kM` → ಕಂ).
* **`gM` is now ಗಂ** like every other letter. The single āgama sign ꣳ comes from `` g`M `` (Vedic mode on).
  `MM` no longer has a special meaning.
* `:g` and `:j` type ಙ and ಞ without the tilde (`~g`, `~j` still work).
* A repeated break key is ignored.
* **Vedic mode** (tray menu, on by default, remembered): off = the accent keys `' " _ ^` and ꣳ are
  switched off, so quotes and apostrophes type normally.

### German keyboard (Windows)
* AltGr is no longer treated as a Ctrl+Alt shortcut, so `~` (AltGr + `+`) works.
* The grave and circumflex dead keys (Shift + `´`, `^`) work as break and Udatta keys without Space.

### Desktop agent
* Tray: **Vedic mode**, **Check for updates** and **Check for updates at startup** items. The update
  check asks GitHub once (system `curl`), only shows a notice, never installs anything.
* Settings are saved to a small text file (`%APPDATA%\KannadaIME\settings.txt`, `~/.config/kannada-ime/settings.txt`).
* A toast tells you if the keyboard hook cannot start (before, typing silently did nothing).
* `--help`, `--version` and `--translit` now print in a terminal on Windows release builds.
* The quick-reference page is saved to a per-process file name and lists Kaveri first.
* Linux agent: `:` key added; documented as US-layout only until the IBus input method exists.

### Web typer
* **Transliteration panel:** the typed Kannada as IAST, plain English or Devanagari.
* **Convert to Kannada:** paste Devanagari, IAST (with diacritics) or phonetic English.
* **Save buttons:** `.txt` files of the typed text, the transliteration and the converted text.
* Same typing rules as the desktop agent, including Vedic mode.

### Project
* 50 unit tests (were 38). CI saves `Cargo.lock` as a downloadable artifact. The release workflow passes the tag
  name through an environment variable. Unused-code and comment warnings cleaned up.

## 0.3.1
* `Ru` and `RU` now also type ಋ and ೠ (the longer `RRi` / `RRI` still work).

## 0.3.0 — Kaveri font 1.2
* **Font renamed to "Kaveri"** (was "Kannada Vedic Long"); version 1.2. Glyphs are unchanged. Uninstall the old font before installing the new one.
* Licences added: code under MIT OR Apache-2.0, font under OFL 1.1 (`fonts/`).
* Font information sheet is now `docs/Kaveri.pdf`.
* GitHub setup: `.gitignore`, CI workflow (tests on Windows and Linux), release workflow (builds the Windows zip), web typer in `docs/index.html` for GitHub Pages, `docs/GITHUB_SETUP.md`, `packaging/QUICKSTART.txt`.
* Tray icon: the ON icon is now the Kannada letter ಕ instead of a Latin K (larger and slightly bolder so it stays readable at 16 px). `tools/make_icons.py` regenerates all icon files.

## 0.2.0 — Kannada Vedic Long font 1.1

### Typing engine
* **Āgama anusvāra (U+A8F3).** `gM` now gives the single sign ꣳ, which already
  contains the ga, so `gM'` = ꣳ + Udatta. `gaM` is unchanged (ಗ + ordinary
  anusvara), and other letters (`kM`) still get the ordinary anusvara.
* `MM` gives U+A8F3 on its own (no ga before it).
* New token kind `Tok::Agama` in `mapping.rs`; the rule lives in `engine.rs`
  (`ends_with_ga_virama`). 37 unit tests (was 36).
* Quick-reference guide lists the new key.

### Font (`tools/make_long_marks.py`, output "Kannada Vedic Long" 1.1)
* Vedic bars (Udatta, Svarita, double Svarita) are 1.4 × taller; the bottom
  edge stays where it was.
* The Devanagari script now has the same GPOS positioning features as Kannada
  in the font. Before, a Vedic mark after U+A8F3 was shaped in a separate
  Devanagari run with no positioning and floated or showed a dotted circle.
* Metadata: family "Kannada Vedic Long", version 1.1, original copyright and
  OFL notice kept. Goda is OFL with Reserved Font Name "Goda", so the copy
  has a new name.
* Not included: the experimental Anudatta raise. The code is in the script
  (fourth argument) but is off by default until it is tested on a large word list.

### Web client
* Browser version of the engine with the font embedded, live inspector,
  example chips and key reference.

### Project
* Version 0.1.0 → 0.2.0 in `Cargo.toml`.
* `docs/KannadaVedicLong.pdf`: font information sheet.

## 0.1.0
* First release: phonetic Kannada engine, Vedic accents, canonical mark order,
  `LLKHF_INJECTED` loop guard, tray, toasts, `Ctrl+Shift+K` toggle,
  `TYPOGRAPHY.md`.
