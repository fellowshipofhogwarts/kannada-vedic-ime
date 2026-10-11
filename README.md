# kannada_ime

A tiny background agent (Rust) that turns phonetic English keystrokes into
Kannada Unicode — Baraha-style, with Vedic accent marks — in any focused text
field. System-tray control, hotkey toggle (`Ctrl+Shift+K`), native toasts.

```
namaste → ನಮಸ್ತೆ     kannaDa → ಕನ್ನಡ     saMskRRita → ಸಂಸ್ಕೃತ     jnaana → ಜ್ಞಾನ
agni'   → ಅಗ್ನಿ॑      ka_ → ಕ॒              ka" → ಕ᳚                 ka^^ → ಕ᳚
```

## Download and use (Windows)

1. Download `kannada-vedic-ime-vX.Y.Z-windows.zip` from the **Releases** page.
2. Unzip, double-click `Kaveri.ttf` → Install, then double-click `kannada_ime.exe`.
3. Pick the font **Kaveri** in your editor and type. `Ctrl+Shift+K` toggles.

No install? Try the web typer (GitHub Pages, `docs/index.html`).

## Status — read this first

| Part | State |
|---|---|
| `mapping.rs`, `engine.rs`, `guide.rs`, `state.rs`, `hook/filter.rs` | Dependency-free; **37 unit tests pass**; engine output verified NFC/NFD-stable over 20,000 random inputs |
| `hook/win.rs`, `hook/linux.rs`, `inject.rs`, `tray.rs`, `main.rs` | Written against pinned crate versions but **not compiled or run by the author** (no crate access in the authoring sandbox). Expect small API fixes on first `cargo build`. Use the checklist in "Verify on Windows" below. |
| Icons | ON icon is the Kannada letter ಕ (generated from the Kaveri font), OFF icon is "EN". Regenerate with `tools/make_icons.py --font <Kannada .ttf>`. |

## Layout

```
Cargo.toml  build.rs  .cargo/config.toml
docs/index.html  web typer (served by GitHub Pages)
docs/GITHUB_SETUP.md  publishing steps
.github/workflows  CI + release builds
src/main.rs      CLI flags, wiring
src/mapping.rs   Baraha + Vedic tables, lookup
src/engine.rs    buffer → render → minimal Edit (+ tests)
src/guide.rs     quick-reference HTML generated from the tables
src/state.rs     ON/OFF flag + epoch shared by threads
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

* Vowels `a aa/A i ii/I/ee u uu/U/oo RRi e E ai o O au`, `M` = ಂ, `H` = ಃ
* Consonants as in Baraha (`k kh g gh ~g c ch j jh ~j T Th D Dh N t th d dh n p ph b bh m y r l v/w sh Sh s h L`), plus `x` = ಕ್ಷ, `jn` = ಜ್ಞ, `f` = ಫ಼
* Conjuncts: just type the consonants (`tatva` → ತತ್ವ)
* `` ` `` = break (`s`ha` → ಸ್ಹ, not ಶ)
* **Vedic accents**, typed right after the syllable:
  `'` or `^` → U+0951 Udatta · `_` or `\` → U+0952 Anudatta ·
  `"` or `^^` → U+1CDA double Svarita. Edit `ACCENTS` in `mapping.rs` to change.
* **Anusvara is capital `M`** (U+0C82); `H` = visarga. Lowercase `m` stays ಮ, because
  making it an anusvara would break `namaste`, `amma`, `kamala`.
* **Āgama anusvāra** (U+A8F3 ꣳ, already contains the ga): type `gM` (`gM'` → ꣳ + Udatta). With the vowel, `gaM` is ga + the ordinary anusvara ಗಂ. Other letters (`kM`) get the ordinary anusvara. `MM` alone also gives U+A8F3. Needs the tuned font from `tools/make_long_marks.py`.
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
cargo test                   # 37 tests; the engine and filter tests run on any OS
cargo build --release        # -> target\release\kannada_ime.exe
.\target\release\kannada_ime.exe
```

Quick mapping check without hooks. Use the *debug* build for this, because the
release `.exe` is a GUI-subsystem program and prints nothing to a console:

```powershell
cargo run -- --translit "agni'mILE'"        # -> ಅಗ್ನಿ॑ಮೀಳೇ॑
```

Manual hook checklist (Notepad, then Word and a browser text box):

1. Tray icon appears (saffron/ON). `Ctrl+Shift+K` → icon turns gray "EN", a toast
   appears; again → back to ON. Tray menu toggle does the same.
2. In Notepad type `kannaDa ` → ಕನ್ನಡ. `agni'` → ಅಗ್ನಿ॑. `kaM'` → ಕಂ॑.
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

Linux notes: capture works on X11 and Wayland (evdev), but synthetic Unicode
injection depends on the compositor; if text does not appear, try
`--inject clipboard`. Tray needs an AppIndicator-capable shell (GNOME needs the
AppIndicator extension). Long-term, an IBus/Fcitx5 engine is the better Linux design.

### Run

```
kannada_ime [--off] [--inject unicode|clipboard]
kannada_ime --translit "namaste"        # test the mapping without any hooks
```

Autostart on Windows: put a shortcut in `shell:startup`.

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
* **Keyboard layouts:** on layouts where `^`, `'` or `"` are *dead keys* (US-International,
  German `^`) the hook sees no character and ignores them, and AltGr-only keys (such as `\`
  on a German layout) count as Ctrl+Alt shortcuts. Use `'`, `"` and `_` (all reachable with
  Shift on German and US layouts) for the accents.
* Not implemented: lowercase `m` as anusvara (conflicts with ಮ, see the typing scheme above).
* Accent glyphs need a font carrying U+0951, U+0952, U+1CDA.
* Toast sender may read "Windows PowerShell" until an AppUserModelID is registered by an installer.

## Licence

* **Program code:** Copyright (c) 2026 Shashank Koundinya Banavathi Rajashekara, MIT OR Apache-2.0 (your choice) — see `LICENSE-MIT`, `LICENSE-APACHE`.
* **Font (`fonts/Kaveri.ttf`):** SIL Open Font License 1.1 — see `fonts/OFL.txt`. It is a modified
  copy of Goda (Parankusacar Institute of Vedic Studies, derived from Sampige by IISc) and is renamed
  because "Goda" is a Reserved Font Name.
