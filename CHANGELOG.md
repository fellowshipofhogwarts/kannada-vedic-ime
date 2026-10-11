# Changelog

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
