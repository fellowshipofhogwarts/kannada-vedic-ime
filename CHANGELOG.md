# Changelog

## 0.5.3

### Typing (desktop app and web typer)
- Long words no longer restart after 48 keys: the word simply continues, so a vowel typed
  next is a vowel sign, not an independent letter.
- `M`, `H` and `:M` with nothing before them (start of a word, after ಽ ೱ ೲ) type the plain
  keys instead of a lone mark on a dotted circle.
- A backtick at the start of a word types a real backtick; inside a word it is still the
  break key.

### Web typer
- Works on phones and tablets (Android, iPhone, iPad; Chrome, Firefox, Safari, Edge,
  Samsung Internet): no sideways scrolling, short labels on the three switches, larger tap
  targets, the reference starts closed and the live inspector is hidden on phones.
- The theme button is now an icon next to the title (◐ automatic, ☀ light, 🌙 dark).
- The example text in the typing box is smaller and in the normal font.
- The font credit now shows that Goda is based on earlier work by the Indian Institute
  of Science. The duplicate credit line in the footer was removed (it is in About & credits).

### Windows app
- `kannada_ime --cleanup` removes the settings, the registry entry and the temp files,
  so the program can be removed without traces (see QUICKSTART).
- A typo in a hand-edited settings.txt keeps the default and is reported once at startup
  (it used to switch the choice off silently). Settings are saved safely (temporary file,
  then replace), and a failed save is reported.
- The update check now offers the final release to people on a pre-release
  (0.5.0-beta → 0.5.0), and reads GitHub's answer more strictly.
- `curl.exe` and `reg.exe` are run from the Windows folder by full path.
- The quick-reference guide file gets a fresh name each time, old ones are deleted, and
  the registry entry for notifications is written only once.
- `kannada_ime --help | more` shows the help text properly.
- Quick reference: the ꣳ row is correct, and the Vedic-mode tip lists the `\` key.

### Build
- Minimum Rust version is now 1.85 (the msrv check is green again).
- `winres` replaced by its maintained fork `winresource`; arboard is built without image
  support (smaller program).
- More tests: long words, literal signs and backtick, modifier and dead-key rules,
  settings typos, version ordering, release JSON.

### Known difference
- The Linux input method (Python) still restarts long words and types a lone mark for a
  word-initial `M`; it is replaced by the Rust engine in 0.6.0.


## 0.5.2

Paste this section at the top of CHANGELOG.md.

### Web typer
- AltGr characters (e.g. `\` for the anudatta on a German keyboard) now attach to the syllable.
  Pressing Ctrl, Alt, AltGr, Shift or Win on its own no longer ends the word.
- The German `^` dead key now types the udatta, as in the desktop app.
- Devanagari view: jihvamuliya and upadhmaniya now become U+1CF5 / U+1CF6.
- Convert text: Devanagari nukta letters (क़ ख़ ग़ ज़ ड़ ढ़ फ़ य़) now convert.
- Transliteration keeps the nukta (dot below) on every letter in ISO 15919 and IAST.
- Ctrl+Z (browser undo) works after typing Kannada.
- A word restarts after 48 keys, the same limit as the desktop app (was 40).
- Hints appear when Caps Lock is on and when a phone keyboard is predicting words.
- Ctrl+Shift+K only toggles while the typing box has focus.
- Kannada text is tagged as Kannada (lang="kn"); screen readers no longer read the
  transliteration box after every key.
- New "About & credits" section and footer credit line, with the full font notices.
- The page shows whether the Arkavathi font actually loaded (About & credits, and in
  the bug report).
- The version shown on the page now comes from Cargo.toml automatically.

### Windows app
- Clipboard mode: if the clipboard cannot be opened, a warning now says so (it used to
  switch to direct typing silently).

### Build and release
- Releases run all tests first; nothing is attached if a test fails.
- A release stops if its tag does not match the version in Cargo.toml.
- CI and releases build exactly the versions in Cargo.lock (`--locked`).
- CI checks that docs/index.html was rebuilt from the current template.
- CI reports (without blocking) whether the stated minimum Rust 1.80 still builds.
- GitHub Actions updated to versions that run on Node 24.

## 0.5.1

### Fixes (Windows)
- **Clipboard mode pasted your old clipboard text.** The clipboard is now restored
  about 0.3 s after the paste, outside the keyboard hook, so the right text is pasted
  and your own clipboard text comes back afterwards.
- **AltGr characters now attach to the syllable.** Pressing Ctrl, Alt, AltGr or Win on
  its own no longer ends the word, so `\` (anudatta) on a German keyboard works mid-word.
- **Mouse-click detection failures are reported.** If Windows refuses the mouse hook you
  get a warning instead of silent text deletion after a click. A watchdog reinstalls
  both hooks if Windows removes them in the background.
- **Fewer wrong backspaces.** The word is also forgotten when the keyboard focus moves
  to another field inside the same window.
- The tray warning title is now "Kannada IME: input problem".
- If the keyboard thread cannot start, you get a warning instead of a crash.

### Internal
- Removed the unsafe `nil<T>()` helper; null handles are now plain null pointers.
- New feature `Win32_UI_Accessibility` in Cargo.toml.
- New tests: modifier handling, watchdog decision, injector (Unicode path).


## 0.5.0

### Typing
* **New keys:** `Z` ಽ (`ZZ` gives ಽಽ), `:h` ೱ, `:H` ೲ, `zh` ೞ, `:l` ಌ, `:L` ೡ, `:M` ಁ (candrabindu), and `Q` right after a
  consonant adds the nukta (`jQa` ಜ಼).
* **Kannada digits are the default:** the number keys (and numpad) type ೦-೯. Untick "Kannada digits" in the tray menu for 0-9.
  The choice is remembered (`kannada_digits` in the settings file).
* `g`M` is unchanged (ꣳ with Vedic mode on, ಗ್ಂ with it off).

### Font
* **Renamed Kaveri to Arkavathi** (version 1.3, same glyphs). Uninstall Kaveri before installing Arkavathi.
  The font information sheet is now `docs/Arkavathi.pdf`.

### Web typer
* Key chart in the order of the Kannada varnamala, with headings, clickable boxes and the new keys.
* Transliteration: ISO 15919 is the default, then IAST, simple English and Devanagari. Vedic marks follow the Rigveda
  printing convention (grave for the upper bar, underline for the lower bar, double grave for the double bar), with a switch for the underline.
* New switch for Kannada digits, a version number, a **Copy bug report** button, and no outside requests (system fonts only).
* A note about Devanagari boxes on iPhone Safari.

### Linux (experimental)
* New `linux/` folder: an IBus input method with the same rules, a LibreOffice Writer template, install scripts and a `.deb` package.

### Project
* Bug-report form on GitHub, new README, a Linux job in CI, and a "Kannada IME" name for Windows toast pop-ups (falls back to the old behaviour if it cannot be registered).

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
