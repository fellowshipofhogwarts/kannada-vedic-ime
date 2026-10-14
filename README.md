# kannada_ime

A tiny background agent (Rust) that turns phonetic English keystrokes into
Kannada Unicode — Baraha-style, with Vedic accent marks — in any focused text
field. System-tray control, hotkey toggle (`Ctrl+Shift+K`), native toasts.

```
namaste → ನಮಸ್ತೆ     kannaDa → ಕನ್ನಡ     saMskRRita → ಸಂಸ್ಕೃತ     jnaana → ಜ್ಞಾನ
agni'   → ಅಗ್ನಿ॑      ka_ → ಕ॒              ka" → ಕ᳚                 ka^^ → ಕ᳚
```

## How this was made

This project was **vibe coded extensively with [Claude](https://claude.ai)** (an AI assistant by Anthropic).
Shashank Koundinya Banavathi Rajashekara set the goals, chose the typing scheme and the Vedic
conventions, tested every version on Windows, and decided what to keep. Claude wrote most of the code,
the tests, the web typer, the font-modification script and the documentation, in a long conversation.

What that means for you:

* **Tested by use, not by audit.** The typing engine has automated tests, and the Windows program and
  the Kaveri font have been used on one Windows PC. Nobody has done a professional code or security review.
  The program listens to all keystrokes while it is switched on, so read the source in `src/hook/` if that
  matters to you.
* **Linux and macOS are untested.** The Linux code exists but has not been run on a real desktop.
* **Check the Vedic text.** The mappings follow common Baraha-style conventions. Compare the output with
  your own printed edition before relying on it for recitation or publishing, and open an issue if a sign
  or accent order is wrong.
* **The font is third-party work.** Kaveri is a small modification of Goda, which is itself derived from
  Sampige (see Licence below). The credit for the letterforms belongs to those authors.

## Download and use (Windows)

1. Download `kannada-vedic-ime-vX.Y.Z-windows.zip` from the **Releases** page.
2. Unzip, double-click `Kaveri.ttf` → Install, then double-click `kannada_ime.exe`.
3. Pick the font **Kaveri** in your editor and type. `Ctrl+Shift+K` toggles.
4. Right-click the tray icon for **Vedic mode** (accent keys and ꣳ on or off, remembered),
   **Check for updates** and the quick reference.

No install? Try the web typer (GitHub Pages, `docs/index.html`). It also shows the text in
IAST, plain English or Devanagari, converts pasted Devanagari, IAST or phonetic text to
Kannada, and saves the result as a `.txt` file.

## Status — read this first

| Part | State |
|---|---|
| `mapping.rs`, `engine.rs`, `guide.rs`, `state.rs`, `settings.rs`, `update.rs`, `hook/filter.rs` | Dependency-free; **50 unit tests pass**; engine output verified NFC/NFD-stable over thousands of random inputs |
| `hook/win.rs`, `hook/linux.rs`, `inject.rs`, `tray.rs`, `main.rs` | Not compiled in the authoring sandbox (no crate access there); GitHub's CI builds them on Windows and Linux. The Windows hook, tray and update check have been used by hand only on one PC. Use the checklist in "Verify on Windows" below. |
| Icons | ON icon is the Kannada letter ಕ (generated from the Kaveri font), OFF icon is "EN". Regenerate with `tools/make_icons.py --font <Kannada .ttf>`. |

## Layout

```
Cargo.toml  build.rs  .cargo/config.toml
docs/index.html  web typer (served by GitHub Pages)
tools/web_typer_template.html  the web typer without its embedded font (build input)
docs/GITHUB_SETUP.md  publishing steps
.github/workflows  CI + release builds
src/main.rs      CLI flags, wiring
src/mapping.rs   Baraha + Vedic tables, lookup
src/engine.rs    buffer → render → minimal Edit (+ tests)
src/guide.rs     quick-reference HTML generated from the tables
src/state.rs     ON/OFF + Vedic-mode flags, epoch, shared by threads
src/settings.rs  Vedic mode + update check, remembered in a small text file
src/update.rs    "newer release on GitHub?" (system curl, version compare)
src/inject.rs    Edit → input (Unicode / clipboard strategies)
src/hook/        win.rs (WH_KEYBOARD_LL + SendInput), linux.rs (rdev + enigo),
                 filter.rs (LLKHF_INJECTED loop guard, pure + tested)
src/tray.rs      tao event loop, tray-icon, notify-rust toasts
assets/icons/    on_64.rgba, off_64.rgba, kannada_ime.ico
docs/ARCHITECTURE.md   flow + state diagrams
docs/Kaveri.pdf   font information sheet
fonts/           Kaveri.ttf + OFL.txt (font licence)
LICENSE-MIT, LICENSE-APACHE   code licence
CHANGELOG.md     what changed in each version
TYPOGRAPHY.md    line spacing (Word/LibreOffice/CSS/Inkscape) + FontForge GPOS anchors
```

## Typing scheme (full tables: tray menu → quick reference)

* Vowels `a aa/A i ii/I/ee u uu/U/oo RRi/Ru RRI/RU e E ai o O au`, `M` = ಂ, `H` = ಃ
* Consonants: `k K g G :g/~g c C j J :j/~j T TH D DH N t th d dh n p P b B m y r l L v/w S Sh/SH s h`
  (capitals are the aspirates: `K` ಖ, `G` ಘ, `J` ಝ, `TH` ಠ, `DH` ಢ, `P` ಫ, `B` ಭ, `S` ಶ, `Sh`/`SH` ಷ),
  plus `x` = ಕ್ಷ, `jn` = ಜ್ಞ, `f` = ಫ಼. The older spellings (`kh gh jh Th Dh ph bh sh`) still work.
  `th` and `dh` stay ಥ and ಧ. A visarga after ಶ ಟ ಡ needs the vowel: `SaH`, `TaH`, `DaH`.
* Conjuncts: just type the consonants (`tatva` → ತತ್ವ)
* `` ` `` = break (`s`ha` → ಸ್ಹ, not ಶ). On a German keyboard Shift + `´` is the break key without Space.
* **Vedic accents**, typed right after the syllable:
  `'` or `^` → U+0951 Udatta · `_` or `\` → U+0952 Anudatta ·
  `"` or `^^` → U+1CDA double Svarita. Edit `ACCENTS` in `mapping.rs` to change.
* **Anusvara is capital `M`** (U+0C82); `H` = visarga. Lowercase `m` stays ಮ, because
  making it an anusvara would break `namaste`, `amma`, `kamala`.
* **Full or half consonant before `M` / `H`:** `kM` → ಕಂ and `cH` → ಚಃ (full letter). Put the break key
  in between for the half letter: `` k`M `` → ಕ್ಂ, `` c`H `` → ಚ್ಃ.
* **Āgama anusvāra** (U+A8F3 ꣳ, already contains the ga): `` g`M `` (`` g`M' `` → ꣳ + Udatta), Vedic mode on.
  Plain `gM` is ಗಂ like every other letter. Needs the tuned font from `tools/make_long_marks.py`.
* **Vedic mode** (tray menu, on by default): switches the accent keys and ꣳ on or off. Off, the keys
  `'` `"` `_` `^` type normally, so quotes and apostrophes work in poems and prose.
* **Output order is always** base → matra → anusvara/visarga → Vedic mark, whatever
  order you typed: `ka'M` and `kaM'` both give ಕಂ॑. Stacked marks follow Unicode
  canonical order (U+0952 before U+0951/U+1CDA).
* Accents and `M`/`H` do **not** end the word: Backspace removes just that mark,
  and typing continues in the same word.
* Backspace undoes the last keystroke of the current word.

## Build

### Windows (recommended target)

```powershell
rustup target add x86_64-pc-windows-msvc
python tools\make_icons.py --font C:\Windows\Fonts\Nirmala.ttf   # optional: real ಕ icon
cargo build --release
# -> target\release\kannada_ime.exe  (statically linked CRT, icon embedded)
```

Needs MSVC Build Tools (includes `rc.exe` for the icon). Optional extra shrink:
`upx --best --lzma target\release\kannada_ime.exe` (may trigger antivirus
heuristics — skip for distribution).

Cross-build from Linux: `rustup target add x86_64-pc-windows-gnu`,
`sudo apt install mingw-w64`, `cargo build --release --target x86_64-pc-windows-gnu`.

### Verify on Windows (do this first)

Open **PowerShell** in the project folder (the one containing `Cargo.toml`).

```powershell
rustup default stable-x86_64-pc-windows-msvc   # needs Visual Studio Build Tools (C++ workload)
cargo check                  # type-checks everything incl. the Win32 hook; fix any API drift here
cargo test                   # 50 tests; the engine, settings, update and filter tests run on any OS
cargo build --release        # -> target\release\kannada_ime.exe
.\target\release\kannada_ime.exe
```

Quick mapping check without hooks (works from a terminal with the release `.exe` too):

```powershell
cargo run -- --translit "agni'mILE'"        # -> ಅಗ್ನಿ॑ಮೀಳೇ॑
```

Manual hook checklist (Notepad, then Word and a browser text box):

1. Tray icon appears (saffron/ON). `Ctrl+Shift+K` → icon turns gray "EN", a toast
   appears; again → back to ON. Tray menu toggle does the same.
2. In Notepad type `kannaDa ` → ಕನ್ನಡ. `agni'` → ಅಗ್ನಿ॑. `kaM'` → ಕಂ॑. `` c`H `` → ಚ್ಃ.
   Tray menu → Vedic mode off: `ka'` now types ಕ' (a plain apostrophe).
   German keyboard: Shift + `´` then `H` after `c` gives ಚ್ಃ, and AltGr + `+` (`~`) works in `~g`.
3. **Loop guard:** type quickly for ten seconds. Output must not repeat or run on,
   and CPU in Task Manager stays near 0 %. (Own `SendInput` events carry
   `LLKHF_INJECTED` and are skipped by `hook/filter.rs`.)
4. Backspace after `ka'` removes only the accent; continuing to type extends the word.
5. While OFF, typing is unchanged. `Ctrl+C/V` and other shortcuts always pass through.
6. If text appears garbled or doubled in some app, retry with
   `kannada_ime.exe --inject clipboard`.

### Linux (beta)

```bash
sudo apt install build-essential pkg-config libgtk-3-dev libayatana-appindicator3-dev \
  libxdo-dev libxkbcommon-dev libx11-dev libxtst-dev libevdev-dev libdbus-1-dev libudev-dev
cargo build --release          # -> target/release/kannada_ime
sudo usermod -aG input $USER   # keyboard grab needs /dev/input + /dev/uinput; re-login
strip target/release/kannada_ime   # already stripped by the release profile
```

**The Linux agent is for US keyboard layouts only.** It reads the physical key position, so on a German
keyboard Y and Z are swapped and AltGr counts as Alt. The planned IBus input method will fix this.

Linux notes: capture works on X11 and Wayland (evdev), but synthetic Unicode
injection depends on the compositor; if text does not appear, try
`--inject clipboard`. Tray needs an AppIndicator-capable shell (GNOME needs the
AppIndicator extension). Long-term, an IBus/Fcitx5 engine is the better Linux design.

### Run

```
kannada_ime [--off] [--inject unicode|clipboard]
kannada_ime --translit "namaste"        # test the mapping without any hooks
kannada_ime --version
```

Autostart on Windows: put a shortcut in `shell:startup`.

Settings (Vedic mode, update check) are saved in `%APPDATA%\KannadaIME\settings.txt` on Windows and
`~/.config/kannada-ime/settings.txt` on Linux. The update check asks GitHub once at startup (through the
system `curl`) and only shows a notice; it never downloads or installs anything. Turn it off in the tray menu.

## Size / memory expectations (estimates, not measurements)

Release binary is likely in the low single-digit MB (opt-level z, LTO, strip).
The hot path (key → render → diff → inject) is microseconds. The <5 MB RAM
target is realistic for the engine itself, but the tray + toast stacks
(WinRT / GTK) usually push total resident memory above that — measure with Task
Manager / `ps` and trim with `--off` + fewer features if it matters.

## Limitations

* Elevated (admin) windows ignore a non-elevated hook (UIPI) — run elevated.
* `Ctrl+Shift+K` is swallowed system-wide (it is a Firefox DevTools shortcut).
* Right after a mapped letter, `'` is an accent, so English contractions need the engine OFF.
* **Keyboard layouts (Windows):** the grave and circumflex dead keys (German Shift + `´` and `^`) work as
  the break and Udatta keys without Space, and AltGr characters such as `~` are typed normally. Other dead
  keys (acute, diaeresis) keep their usual behaviour. `'`, `"` and `_` are reachable with Shift on German
  and US layouts.
* Not implemented: lowercase `m` as anusvara (conflicts with ಮ, see the typing scheme above).
* Accent glyphs need a font carrying U+0951, U+0952, U+1CDA.
* Toast sender may read "Windows PowerShell" until an AppUserModelID is registered by an installer.

## Licence

* **Program code:** Copyright (c) 2026 Shashank Koundinya Banavathi Rajashekara, MIT OR Apache-2.0 (your choice) — see `LICENSE-MIT`, `LICENSE-APACHE`.
* **Font (`fonts/Kaveri.ttf`):** SIL Open Font License 1.1 — see `fonts/OFL.txt`. It is a modified
  copy of Goda (Parankusacar Institute of Vedic Studies, derived from Sampige by IISc) and is renamed
  because "Goda" is a Reserved Font Name.

## Credits

* Goda font: Parankusacar Institute of Vedic Studies (derived from Sampige, Indian Institute of Science).
* Typing scheme: inspired by Baraha-style phonetic input.
* Code, tests, web typer and documentation: written with Claude (Anthropic), directed and tested by the project author.
