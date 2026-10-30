//! engine.rs — the phonetic state engine. Pure logic, no OS calls, no deps.
//!
//! # Model
//!
//! The engine keeps the *roman keystrokes of the word being typed* (`buf`) and
//! the *Unicode currently on screen for that word* (`shown`). Every key:
//!
//!   1. appends to / edits `buf`,
//!   2. re-renders the **whole buffer** from scratch with [`render`]
//!      (greedy longest-match tokenizer, see mapping.rs),
//!   3. diffs the new text against `shown` by common code-point prefix, and
//!   4. returns the minimal [`Edit`]: *N backspaces, then this text*.
//!
//! Re-rendering from scratch makes every multi-key combination fall out for
//! free (`k`→ಕ್, `a`→ಕ, `a`→ಕಾ, `ee` vs `e`, `kh` vs `k`+`h`, `^`→`^^` upgrade…)
//! and keeps the engine deterministic and trivial to test; the diff keeps the
//! number of injected backspaces tiny (usually 0 or 1).
//!
//! # Lifecycle of the buffer
//!
//! The buffer is cleared (`reset`) whenever the user does anything that is not
//! a phonetic key: space, Enter, arrows, mouse click, shortcut, focus change,
//! app toggle… The engine therefore never backspaces over text it did not type.
//!
//! A word longer than [`MAX_BUFFER`] keys is not broken (0.5.3): the oldest,
//! finished syllables are committed and the rest of the word stays live, with
//! exactly the same output as if the whole word had been kept (see
//! [`Engine::make_room`]).
//!
//! Typing an accent (`'` `^` `_` `\` `"` `^^`) or anusvara/visarga (`M` `H`)
//! does **not** end the word. The key is appended to the buffer like any other,
//! so the word stays live: Backspace removes just that mark, and further letters
//! keep extending the same word (`ka'` → ಕ॑, Backspace → ಕ, `ka'ra` → ಕ॑ರ).
//!
//! # Output ordering (what OpenType shapers expect)
//!
//! Each syllable is always emitted as
//!
//! ```text
//! base consonant (+ virama + consonant …) → matra → anusvara/visarga → Vedic marks
//! ```
//!
//! **regardless of the order the keys were typed in**: `ka'M` and `kaM'` both
//! produce ಕ + ಂ + U+0951. Because the engine re-renders the whole buffer on
//! every key, an `M` typed after an accent is simply slotted in *before* the
//! marks, and stacked marks are sorted by Unicode combining class (U+0952 = 220
//! before U+0951/U+1CDA = 230), so the text is already NFC/NFD-stable.
//!
//! Why this order keeps anusvara inline while the accent sits above the cluster:
//!
//! * Anusvara U+0C82 is a *spacing* combining mark (category Mc, ccc 0). It has
//!   its own advance width and is drawn as its own glyph (ಂ) to the right of the
//!   syllable. Shaping engines treat it as a syllable-final modifier, so it must
//!   follow the matra directly, inside the syllable, to stay inline.
//! * The Vedic marks U+0951 / U+0952 / U+1CDA are *non-spacing* marks (Mn, ccc
//!   230 above / 220 below, zero advance). They have no position of their own:
//!   the font's GPOS mark-to-base lookups anchor them to the preceding base
//!   glyph, above (230) or below (220) it. Placing them last means (a) they come
//!   after the syllable's modifiers, where Indic syllable grammars (HarfBuzz,
//!   DirectWrite, Uniscribe) accept accents, so they stay in the same syllable
//!   instead of being split off or shown on a dotted circle, and (b) they never
//!   sit between the matra and anusvara, where they could break the
//!   consonant+matra cluster the shaper is trying to form or push the spacing
//!   anusvara away from its syllable.
//! * Exactly *which* glyph the accent lands on (consonant, matra or anusvara) is
//!   decided by the font's GDEF classes and anchors, not by the engine. If an
//!   accent sits on the anusvara dot instead of above the letter, fix the font's
//!   anchors (see TYPOGRAPHY.md, FontForge section).

use crate::mapping::{self, Tok, VIRAMA};

/// Keystrokes kept in the live buffer. When a word grows past this, its oldest
/// finished syllables are committed and dropped from the buffer; the word
/// itself continues unchanged (0.5.3). Backspace can undo at most this many
/// keys of a word.
pub const MAX_BUFFER: usize = 48;

/// What the injector has to do: erase `backspaces` code points, then type `text`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Edit {
    pub backspaces: usize,
    pub text: String,
}

impl Edit {
    pub fn is_noop(&self) -> bool {
        self.backspaces == 0 && self.text.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Not ours: let the original key through (the engine has already reset).
    Pass,
    /// Swallow the physical key and apply this edit instead (may be a no-op).
    Consumed(Edit),
}

/// Result of rendering a roman buffer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Rendered {
    pub text: String,
    /// True if the last thing rendered is a Kannada unit an accent may attach to.
    pub attachable: bool,
}

/// Render a roman string to Kannada Unicode (Baraha scheme), Vedic mode on.
///
/// Rules, in one place:
/// * consonant  → cluster + virama (`pending` = virama may still be replaced)
/// * vowel      → after a pending consonant: replace virama by the matra (or drop
///   it for inherent `a`); otherwise the independent vowel
/// * M / H / :M → anusvara / visarga / candrabindu. With nothing to attach to
///   (start of a word, after ಽ ೱ ೲ or a literal) the keys stay literal text,
///   like the accent keys (0.5.3; no lone mark on a dotted circle). Right after
///   a bare consonant the virama is
///   dropped (full letter: `kM` = ಕಂ). With a break key in between the virama
///   stays (half letter: `` k`M `` = ಕ್ಂ). Otherwise the sign is inserted
///   *before* any Vedic marks already present (canonical order)
/// * `` g`M `` → with Vedic mode on, the single agama anusvara U+A8F3 (it already
///   contains the ga); with Vedic mode off, the plain ಗ್ಂ
/// * accent     → (Vedic mode only) combining mark appended after the unit (drops
///   a pending virama: an accent implies the inherent `a`); runs of marks
///   are kept sorted by combining class; with nothing to attach
///   to it stays a literal character
/// * `` ` ``    → break: next vowel is independent, nothing is emitted
/// * unknown    → kept literally (this is how partial tokens like `R` show up
///   until `RRi` completes them)
pub fn render(roman: &str) -> Rendered {
    render_with(roman, true)
}

/// [`render`] with the Vedic switch: when `vedic` is false the accent keys are
/// ordinary characters and `` g`M `` gives ಗ್ಂ instead of U+A8F3.
pub fn render_with(roman: &str, vedic: bool) -> Rendered {
    render_core(roman, vedic, false, None)
}

/// A point inside a roman buffer where it can be cut in two without changing
/// the output: rendering the part after it (starting with `attachable`) gives
/// exactly the rest of the full output. See [`Engine::make_room`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SafeCut {
    /// Byte offset into the roman buffer.
    roman: usize,
    /// Byte offset into the rendered text.
    text: usize,
    /// Whether the next token may attach to what came before.
    attachable: bool,
}

/// The renderer. `start_attachable` continues a word whose beginning was
/// already committed; `cuts`, when given, collects every [`SafeCut`].
///
/// A cut is safe right after a token when nothing that follows can reach back
/// into the text before it: no virama waiting to be replaced (`pending`), no
/// break key waiting for the next token, and the text does not end with Vedic
/// marks (a later sign or accent would be sorted in among them).
fn render_core(
    roman: &str,
    vedic: bool,
    start_attachable: bool,
    mut cuts: Option<&mut Vec<SafeCut>>,
) -> Rendered {
    let table = mapping::table();
    let mut out = String::with_capacity(roman.len() * 3);
    let mut pending = false; // out currently ends with a virama we appended
    let mut attachable = start_attachable;
    let mut after_break = false;

    let mut i = 0;
    while i < roman.len() {
        // Greedy longest match.
        let mut hit: Option<(Tok, usize)> = None;
        for len in (1..=mapping::MAX_TOKEN_LEN).rev() {
            let end = i + len;
            if end <= roman.len() && roman.is_char_boundary(end) {
                if let Some(t) = table.get(&roman[i..end]) {
                    if !vedic && matches!(t, Tok::Accent(_)) {
                        continue; // Vedic mode off: accent keys are plain text
                    }
                    hit = Some((*t, len));
                    break;
                }
            }
        }

        let Some((tok, len)) = hit else {
            // Literal: copy one char.
            let ch = roman[i..].chars().next().unwrap_or('\u{FFFD}');
            out.push(ch);
            pending = false;
            attachable = false;
            after_break = false;
            i += ch.len_utf8();
            note_cut(&mut cuts, i, &out, pending, after_break, attachable);
            continue;
        };

        match tok {
            Tok::Consonant(cluster) => {
                out.push_str(cluster);
                out.push(VIRAMA);
                pending = true;
                attachable = true;
                after_break = false;
            }
            Tok::Vowel { independent, matra } => {
                if pending && !after_break {
                    out.pop(); // the virama
                    if let Some(m) = matra {
                        out.push(m);
                    }
                } else {
                    out.push(independent);
                }
                pending = false;
                attachable = true;
                after_break = false;
            }
            Tok::Sign(s) => {
                if pending && after_break {
                    // Break key before the sign: keep the half consonant (virama stays).
                    if vedic && s == mapping::ANUSVARA && ends_with_ga_virama(&out) {
                        // U+A8F3 already *contains* the ga, so it replaces ga + virama.
                        out.pop(); // virama
                        out.pop(); // ga
                        out.push(mapping::AGAMA_ANUSVARA);
                    } else {
                        out.push(s);
                    }
                } else if pending {
                    out.pop(); // the virama: full consonant + sign
                    out.push(s);
                } else if attachable {
                    // Canonical order: sign goes before any Vedic marks typed earlier.
                    let at = trailing_marks_start(&out);
                    out.insert(at, s);
                } else {
                    // Nothing to sit on (start of a word, after ಽ ೱ ೲ or a
                    // literal): the keys stay literal text, like the accent keys.
                    out.push_str(&roman[i..i + len]);
                    after_break = false;
                    i += len;
                    note_cut(&mut cuts, i, &out, pending, after_break, attachable);
                    continue;
                }
                pending = false;
                attachable = true;
                after_break = false;
            }
            Tok::Accent(mark) => {
                if attachable {
                    if pending {
                        out.pop();
                        pending = false;
                    }
                    out.push(mark);
                    sort_trailing_marks(&mut out);
                } else {
                    out.push_str(&roman[i..i + len]);
                    pending = false;
                }
                after_break = false;
            }
            Tok::Other(ch) => {
                out.push(ch);
                pending = false;
                attachable = false;
                after_break = false;
            }
            Tok::Nukta => {
                // Only right after a bare consonant: the nukta goes before the
                // virama (ಜ + ಼ + ್), so a following vowel sign lands on ಜ಼.
                if pending && !out[..out.len() - VIRAMA.len_utf8()].ends_with(mapping::NUKTA) {
                    out.pop();
                    out.push(mapping::NUKTA);
                    out.push(VIRAMA);
                } else {
                    out.push('Q');
                    pending = false;
                    attachable = false;
                }
                after_break = false;
            }
            Tok::Break => after_break = true,
        }
        i += len;
        note_cut(&mut cuts, i, &out, pending, after_break, attachable);
    }

    Rendered {
        text: out,
        attachable,
    }
}

/// Record a [`SafeCut`] after the token that ended at roman byte `i`, if safe.
fn note_cut(
    cuts: &mut Option<&mut Vec<SafeCut>>,
    i: usize,
    out: &str,
    pending: bool,
    after_break: bool,
    attachable: bool,
) {
    if let Some(cuts) = cuts.as_deref_mut() {
        let ends_with_mark = out.chars().next_back().is_some_and(mapping::is_vedic_mark);
        if !pending && !after_break && !ends_with_mark {
            cuts.push(SafeCut {
                roman: i,
                text: out.len(),
                attachable,
            });
        }
    }
}

/// True if `s` ends with ಗ + virama (a bare "g" waiting for its vowel).
fn ends_with_ga_virama(s: &str) -> bool {
    s.ends_with("\u{0C97}\u{0CCD}")
}

/// Byte offset where the trailing run of Vedic marks in `s` starts
/// (`s.len()` when `s` does not end in a mark).
fn trailing_marks_start(s: &str) -> usize {
    let mut start = s.len();
    for (i, c) in s.char_indices().rev() {
        if mapping::is_vedic_mark(c) {
            start = i;
        } else {
            break;
        }
    }
    start
}

/// Put the trailing run of marks into canonical order (ascending combining
/// class, stable), e.g. udatta+anudatta → anudatta+udatta.
fn sort_trailing_marks(s: &mut String) {
    let start = trailing_marks_start(s);
    let mut marks: Vec<char> = s[start..].chars().collect();
    if marks.len() > 1 {
        marks.sort_by_key(|&c| mapping::combining_class(c)); // stable sort
        s.truncate(start);
        s.extend(marks);
    }
}

/// Convenience: whole-string transliteration (used by `--translit` and tests).
pub fn transliterate(roman: &str) -> String {
    render(roman).text
}

/// Minimal edit that turns `old` into `new` when only the tail may change.
pub fn diff(old: &str, new: &str) -> Edit {
    let mut common_bytes = 0;
    let mut old_it = old.chars();
    let mut new_it = new.chars();
    loop {
        match (old_it.clone().next(), new_it.clone().next()) {
            (Some(a), Some(b)) if a == b => {
                common_bytes += a.len_utf8();
                old_it.next();
                new_it.next();
            }
            _ => break,
        }
    }
    Edit {
        backspaces: old[common_bytes..].chars().count(),
        text: new[common_bytes..].to_string(),
    }
}

#[derive(Debug)]
pub struct Engine {
    buf: String,
    shown: String,
    /// The start of the current word was committed by [`Engine::make_room`]
    /// and the next token may attach to it (e.g. an accent after ಕ).
    start_attachable: bool,
    /// Vedic mode: accent keys and the agama anusvara are active.
    vedic: bool,
    /// Kannada digits: the keys 0-9 type ೦-೯ (default). Off = English digits.
    digits: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            buf: String::new(),
            shown: String::new(),
            start_attachable: false,
            vedic: true,
            digits: true,
        }
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget the current word (caret moved, focus changed, toggled, …).
    pub fn reset(&mut self) {
        self.buf.clear();
        self.shown.clear();
        self.start_attachable = false;
    }

    fn render(&self) -> Rendered {
        render_core(&self.buf, self.vedic, self.start_attachable, None)
    }

    /// Make room for one more key in a full buffer without breaking the word
    /// (0.5.3; before, the word simply restarted, so the next vowel came out as
    /// an independent letter). The longest finished part of the word is
    /// committed: dropped from `buf` and from `shown`, while what stays renders
    /// exactly as before. Returns false if the buffer has no safe cut at all
    /// (e.g. 48 bare consonants); then the caller falls back to a reset.
    fn make_room(&mut self) -> bool {
        if self.buf.len() < MAX_BUFFER {
            return true;
        }
        let mut cuts = Vec::new();
        let r = render_core(&self.buf, self.vedic, self.start_attachable, Some(&mut cuts));
        debug_assert_eq!(r.text, self.shown);
        // A cut is only final if the token before it was matched with its full
        // look-ahead (`R` may still become `RRi`, `a` may still become `aa`).
        let stable = self.buf.len() + 1 - mapping::MAX_TOKEN_LEN;
        match cuts.iter().rev().find(|c| c.roman <= stable) {
            Some(cut) if cut.text <= self.shown.len() => {
                self.buf.drain(..cut.roman);
                self.shown.drain(..cut.text);
                self.start_attachable = cut.attachable;
                true
            }
            _ => false,
        }
    }

    /// Switch Vedic mode. A change ends the current word.
    pub fn set_vedic(&mut self, on: bool) {
        if self.vedic != on {
            self.vedic = on;
            self.reset();
        }
    }

    /// Choose Kannada (true) or English (false) digits. A change ends the word.
    pub fn set_digits(&mut self, on: bool) {
        if self.digits != on {
            self.digits = on;
            self.reset();
        }
    }

    #[cfg(test)]
    pub fn is_idle(&self) -> bool {
        self.buf.is_empty()
    }

    /// Feed one typed character (already resolved from the key + layout).
    pub fn on_char(&mut self, c: char) -> Outcome {
        if c == '`' && self.buf.is_empty() && !self.start_attachable {
            // At the start of a word the break key has nothing to break: type a
            // real backtick (0.5.3). Inside a word it stays the break key.
            self.reset();
            return Outcome::Pass;
        }
        if c == '`' && self.buf.ends_with('`') {
            // A repeated break key changes nothing: swallow it silently.
            return Outcome::Consumed(Edit::default());
        }
        if self.digits {
            if let Some(d) = mapping::kannada_digit(c) {
                // A digit ends the word and is typed as a Kannada digit.
                self.reset();
                return Outcome::Consumed(Edit {
                    backspaces: 0,
                    text: d.to_string(),
                });
            }
        }
        if mapping::is_phonetic_key(c) {
            if !self.make_room() {
                self.reset();
            }
            self.buf.push(c);
            return Outcome::Consumed(self.sync());
        }
        if self.vedic && mapping::is_accent_key(c) && self.render().attachable {
            // `'`/`^`/`_`/`"`… only act as accents right after a Kannada unit;
            // anywhere else they are ordinary punctuation and pass through.
            if !self.make_room() {
                self.reset();
                return Outcome::Pass;
            }
            self.buf.push(c);
            return Outcome::Consumed(self.sync());
        }
        self.reset();
        Outcome::Pass
    }

    /// Backspace = undo the last *keystroke* of the current word. With an empty
    /// buffer the key goes through untouched.
    pub fn on_backspace(&mut self) -> Outcome {
        if self.buf.is_empty() {
            self.reset(); // a native backspace: the committed start may be gone
            return Outcome::Pass;
        }
        self.buf.pop();
        Outcome::Consumed(self.sync())
    }

    fn sync(&mut self) -> Edit {
        let r = self.render();
        let edit = diff(&self.shown, &r.text);
        self.shown = r.text;
        edit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simulated text field: applies engine outcomes exactly like the injector.
    #[derive(Default)]
    struct Screen {
        text: Vec<char>,
        engine: Engine,
        backspaces_sent: usize,
    }

    impl Screen {
        fn key(&mut self, c: char) -> Outcome {
            let o = self.engine.on_char(c);
            self.apply(&o, Some(c));
            o
        }
        fn backspace(&mut self) {
            let o = self.engine.on_backspace();
            self.apply(&o, None);
            if o == Outcome::Pass {
                self.text.pop(); // native backspace
            }
        }
        fn apply(&mut self, o: &Outcome, raw: Option<char>) {
            match o {
                Outcome::Pass => {
                    if let Some(c) = raw {
                        self.text.push(c);
                    }
                }
                Outcome::Consumed(e) => {
                    for _ in 0..e.backspaces {
                        self.text.pop();
                    }
                    self.backspaces_sent += e.backspaces;
                    self.text.extend(e.text.chars());
                }
            }
        }
        fn type_str(&mut self, s: &str) -> &mut Self {
            for c in s.chars() {
                self.key(c);
            }
            self
        }
        fn s(&self) -> String {
            self.text.iter().collect()
        }
    }

    fn typed(s: &str) -> String {
        let mut sc = Screen::default();
        sc.type_str(s);
        sc.s()
    }

    #[test]
    fn independent_vowels() {
        for (k, v) in [
            ("a", "ಅ"),
            ("aa", "ಆ"),
            ("A", "ಆ"),
            ("i", "ಇ"),
            ("ee", "ಈ"),
            ("I", "ಈ"),
            ("u", "ಉ"),
            ("oo", "ಊ"),
            ("U", "ಊ"),
            ("RRi", "ಋ"),
            ("RRI", "ೠ"),
            ("e", "ಎ"),
            ("E", "ಏ"),
            ("ai", "ಐ"),
            ("o", "ಒ"),
            ("O", "ಓ"),
            ("au", "ಔ"),
        ] {
            assert_eq!(typed(k), v, "vowel {k}");
        }
    }

    #[test]
    fn matras_on_ka() {
        for (k, v) in [
            ("ka", "ಕ"),
            ("kaa", "ಕಾ"),
            ("ki", "ಕಿ"),
            ("kee", "ಕೀ"),
            ("ku", "ಕು"),
            ("koo", "ಕೂ"),
            ("kRRi", "ಕೃ"),
            ("ke", "ಕೆ"),
            ("kE", "ಕೇ"),
            ("kai", "ಕೈ"),
            ("ko", "ಕೊ"),
            ("kO", "ಕೋ"),
            ("kau", "ಕೌ"),
        ] {
            assert_eq!(typed(k), v, "syllable {k}");
        }
    }

    #[test]
    fn words_and_conjuncts() {
        assert_eq!(typed("namaste"), "ನಮಸ್ತೆ");
        assert_eq!(typed("kannaDa"), "ಕನ್ನಡ");
        assert_eq!(typed("saMskRRita"), "ಸಂಸ್ಕೃತ");
        assert_eq!(typed("tatva"), "ತತ್ವ");
        assert_eq!(typed("jnaana"), "ಜ್ಞಾನ");
        assert_eq!(typed("kShetra"), "ಕ್ಷೆತ್ರ");
        assert_eq!(typed("xetra"), "ಕ್ಷೆತ್ರ");
        assert_eq!(typed("bhaarata"), "ಭಾರತ");
        assert_eq!(typed("Baarata"), "ಭಾರತ");
        assert_eq!(typed("shaale"), "ಶಾಲೆ");
        assert_eq!(typed("Saale"), "ಶಾಲೆ");
        assert_eq!(typed("Sha"), "ಷ");
        assert_eq!(typed("SHa"), "ಷ");
        assert_eq!(typed("kh"), "ಖ್");
        assert_eq!(typed("aM"), "ಅಂ");
        assert_eq!(typed("aH"), "ಅಃ");
        assert_eq!(typed("~ga"), "ಙ");
        assert_eq!(typed(":ga"), "ಙ");
        assert_eq!(typed(":ja"), "ಞ");
        assert_eq!(typed(":"), ":");
        assert_eq!(typed("k"), "ಕ್"); // trailing virama stays
    }

    #[test]
    fn break_key_splits_tokens() {
        assert_eq!(typed("s`ha"), "ಸ್ಹ");
        assert_eq!(typed("a`i"), "ಅಇ");
        assert_eq!(typed("t`ha"), "ತ್ಹ");
    }

    #[test]
    fn non_phonetic_keys_pass_and_reset() {
        assert_eq!(typed("ka ka"), "ಕ ಕ");
        // Space ends the word: the second `a` is independent again.
        assert_eq!(typed("ka a"), "ಕ ಅ");
        // Unmapped Latin letters pass through untouched.
        assert_eq!(typed("q"), "q");
        assert_eq!(typed("1.5"), "೧.೫"); // Kannada digits are on by default
    }

    #[test]
    fn digits_can_be_switched_to_english() {
        let mut sc = Screen::default();
        sc.engine.set_digits(false);
        sc.type_str("ka1.5");
        assert_eq!(sc.s(), "ಕ1.5");
        sc.engine.set_digits(true);
        sc.type_str("0987654321");
        assert!(sc.s().ends_with("೦೯೮೭೬೫೪೩೨೧"));
    }

    #[test]
    fn digit_ends_the_word() {
        // After a digit the next vowel is independent again.
        assert_eq!(typed("k1a"), "ಕ್೧ಅ");
    }

    #[test]
    fn version_050_new_keys() {
        assert_eq!(typed("Z"), "ಽ");
        assert_eq!(typed("ZZ"), "ಽಽ");
        assert_eq!(typed("aZ"), "ಅಽ");
        assert_eq!(typed(":h"), "ೱ");
        assert_eq!(typed(":H"), "ೲ");
        assert_eq!(typed(":hka"), "ೱಕ");
        assert_eq!(typed("zha"), "ೞ");
        assert_eq!(typed("zhi"), "ೞಿ");
        assert_eq!(typed(":l"), "ಌ");
        assert_eq!(typed(":L"), "ೡ");
        assert_eq!(typed("k:l"), "ಕೢ");
        assert_eq!(typed("k:L"), "ಕೣ");
        assert_eq!(typed(":M"), ":M"); // nothing to sit on: literal (0.5.3)
        assert_eq!(typed("ka:M"), "ಕಁ");
        // Nukta: right after a consonant, before the virama / vowel sign.
        assert_eq!(typed("jQa"), "ಜ಼");
        assert_eq!(typed("jQi"), "ಜ಼ಿ");
        assert_eq!(typed("DQa"), "ಡ಼");
        assert_eq!(typed("jQ"), "ಜ಼್");
        // Q anywhere else stays a plain letter; a second Q too.
        assert_eq!(typed("kaQ"), "ಕQ");
        assert_eq!(typed("jQQ"), "ಜ಼್Q");
        // Dropped keys no longer exist.
        assert_eq!(typed("ra"), "ರ");
        assert_eq!(typed("z"), "z");
    }

    #[test]
    fn nukta_and_accent_stay_in_order() {
        assert_eq!(typed("jQa'"), "ಜ಼\u{0951}");
        assert_eq!(typed("jQ'"), "ಜ಼\u{0951}");
    }

    #[test]
    fn incremental_edits_are_minimal() {
        let mut sc = Screen::default();
        sc.type_str("ka");
        assert_eq!(sc.s(), "ಕ");
        let before = sc.backspaces_sent;
        sc.type_str("i"); // ಕ -> ಕಿ : pure append
        assert_eq!(sc.backspaces_sent, before);

        let mut sc = Screen::default();
        sc.type_str("ke");
        let before = sc.backspaces_sent;
        sc.type_str("e"); // ಕೆ -> ಕೀ : one backspace
        assert_eq!(sc.s(), "ಕೀ");
        assert_eq!(sc.backspaces_sent - before, 1);

        // "kSh": S is ಶ, and the h upgrades it to ಷ in place (two backspaces).
        let mut sc = Screen::default();
        sc.type_str("kS");
        assert_eq!(sc.s(), "ಕ್ಶ್");
        let before = sc.backspaces_sent;
        sc.type_str("h");
        assert_eq!(sc.s(), "ಕ್ಷ್");
        assert_eq!(sc.backspaces_sent - before, 2);
    }

    #[test]
    fn backspace_undoes_last_keystroke() {
        let mut sc = Screen::default();
        sc.type_str("kh");
        assert_eq!(sc.s(), "ಖ್");
        sc.backspace();
        assert_eq!(sc.s(), "ಕ್");
        sc.backspace();
        assert_eq!(sc.s(), "");
        // Empty buffer: native backspace passes through and deletes foreign text.
        let mut sc = Screen {
            text: "xyz".chars().collect(),
            ..Screen::default()
        };
        sc.backspace();
        assert_eq!(sc.s(), "xy");
    }

    #[test]
    fn partial_token_upgrades_in_place() {
        let mut sc = Screen::default();
        sc.type_str("RR");
        assert_eq!(sc.s(), "RR");
        sc.type_str("i");
        assert_eq!(sc.s(), "ಋ");
        sc.reset();
        sc.type_str("j");
        sc.type_str("n");
        assert_eq!(sc.s(), "ಜ್ಞ್");
    }

    impl Screen {
        fn reset(&mut self) {
            self.text.clear();
            self.engine.reset();
        }
    }

    // ---- Vedic accents -------------------------------------------------

    const U: char = '\u{0951}';
    const A: char = '\u{0952}';
    const D: char = '\u{1CDA}';

    #[test]
    fn accent_after_matra_and_consonant() {
        assert_eq!(typed("agni'"), format!("ಅಗ್ನಿ{U}"));
        assert_eq!(typed("agni^"), format!("ಅಗ್ನಿ{U}"));
        assert_eq!(typed("ka'"), format!("ಕ{U}"));
        assert_eq!(typed("ka_"), format!("ಕ{A}"));
        assert_eq!(typed("ka\\"), format!("ಕ{A}"));
        assert_eq!(typed("ka\""), format!("ಕ{D}"));
    }

    #[test]
    fn accent_on_conjunct_and_vowel_signs() {
        assert_eq!(typed("kShE'"), format!("ಕ್ಷೇ{U}"));
        assert_eq!(typed("tatva'"), format!("ತತ್ವ{U}"));
        assert_eq!(typed("a'"), format!("ಅ{U}"));
        assert_eq!(typed("aM'"), format!("ಅಂ{U}"));
        assert_eq!(typed("saMskRRi_"), format!("ಸಂಸ್ಕೃ{A}"));
        // Accent on a bare consonant implies the inherent a.
        assert_eq!(typed("k'"), format!("ಕ{U}"));
    }

    #[test]
    fn multiple_accents_and_following_text() {
        assert_eq!(typed("ka'_"), format!("ಕ{A}{U}")); // canonical: 220 before 230
        assert_eq!(typed("agni'mILE"), format!("ಅಗ್ನಿ{U}ಮೀಳೇ"));
        assert_eq!(typed("a'ka"), format!("ಅ{U}ಕ"));
    }

    #[test]
    fn every_vedic_trigger_maps_to_its_codepoint() {
        for (keys, want) in [
            ("'", 0x0951u32),
            ("^", 0x0951),
            ("_", 0x0952),
            ("\\", 0x0952),
            ("\"", 0x1CDA),
            ("^^", 0x1CDA),
        ] {
            let s = typed(&format!("ka{keys}"));
            let mut it = s.chars();
            assert_eq!(it.next(), Some('ಕ'), "trigger {keys:?}");
            assert_eq!(it.next().map(|c| c as u32), Some(want), "trigger {keys:?}");
            assert_eq!(it.next(), None, "trigger {keys:?} emitted extra text");
        }
    }

    #[test]
    fn lowercase_m_stays_a_consonant_capital_m_is_anusvara() {
        // `m` = ಮ in the Baraha scheme; making it an anusvara would break
        // namaste / amma / kamala. Anusvara is `M` only.
        assert_eq!(typed("m"), "ಮ್");
        assert_eq!(typed("ma"), "ಮ");
        assert_eq!(typed("namaste"), "ನಮಸ್ತೆ");
        assert_eq!(typed("amma"), "ಅಮ್ಮ");
        assert_eq!(typed("aM"), "ಅಂ");
        assert_eq!(typed("kaM"), "ಕಂ");
        assert_eq!(typed("aM").chars().next_back().unwrap() as u32, 0x0C82);
    }

    #[test]
    fn anusvara_always_precedes_vedic_marks() {
        // base → matra → anusvara/visarga → mark, whichever key came first.
        for (a, b, want) in [
            ("kaM'", "ka'M", format!("ಕಂ{U}")),
            ("kiM_", "ki_M", format!("ಕಿಂ{A}")),
            ("kEM\"", "kE\"M", format!("ಕೇಂ{D}")),
            ("kShEM'", "kShE'M", format!("ಕ್ಷೇಂ{U}")),
            ("kaM^^", "ka^^M", format!("ಕಂ{D}")),
            ("kaH'", "ka'H", format!("ಕಃ{U}")),
            ("kaM'_", "ka_'M", format!("ಕಂ{A}{U}")),
        ] {
            assert_eq!(typed(a), want, "typed {a:?}");
            assert_eq!(typed(b), want, "typed {b:?}");
        }
    }

    #[test]
    fn stacked_marks_are_in_canonical_order() {
        // ccc: anudatta 220 sorts before udatta / double svarita 230.
        assert_eq!(typed("ka'_"), format!("ಕ{A}{U}"));
        assert_eq!(typed("ka_'"), format!("ಕ{A}{U}"));
        assert_eq!(typed("ka\"_"), format!("ಕ{A}{D}"));
        assert_eq!(typed("ka_\""), format!("ಕ{A}{D}"));
    }

    #[test]
    fn anusvara_after_accent_is_one_small_edit() {
        let mut sc = Screen::default();
        sc.type_str("ka'");
        let before = sc.backspaces_sent;
        sc.type_str("M");
        assert_eq!(sc.s(), format!("ಕಂ{U}"));
        // Erase the accent, then type ಂ + accent: exactly one backspace.
        assert_eq!(sc.backspaces_sent - before, 1);
    }

    #[test]
    fn buffer_stays_live_after_accent_or_anusvara() {
        // Accent: not flushed; Backspace removes only the accent; typing continues.
        let mut sc = Screen::default();
        sc.type_str("ka'");
        assert!(!sc.engine.is_idle());
        sc.backspace();
        assert_eq!(sc.s(), "ಕ");
        assert!(!sc.engine.is_idle());
        sc.type_str("r"); // same word keeps growing: "kar" -> ಕರ್
        assert_eq!(sc.s(), "ಕರ್");

        // Accent then more letters extend the same word, and unwind one key at a time.
        let mut sc = Screen::default();
        sc.type_str("ka'ra");
        assert_eq!(sc.s(), format!("ಕ{U}ರ"));
        sc.backspace();
        assert_eq!(sc.s(), format!("ಕ{U}ರ್"));
        sc.backspace();
        assert_eq!(sc.s(), format!("ಕ{U}"));

        // Anusvara + accent unwind in reverse typing order.
        let mut sc = Screen::default();
        sc.type_str("kaM'");
        assert!(!sc.engine.is_idle());
        sc.backspace();
        assert_eq!(sc.s(), "ಕಂ");
        sc.backspace();
        assert_eq!(sc.s(), "ಕ");
        sc.backspace();
        assert_eq!(sc.s(), "ಕ್");
        sc.type_str("u");
        assert_eq!(sc.s(), "ಕು");
    }

    #[test]
    fn caret_caret_upgrades_to_double_svarita() {
        let mut sc = Screen::default();
        sc.type_str("ka^");
        assert_eq!(sc.s(), format!("ಕ{U}"));
        let before = sc.backspaces_sent;
        sc.type_str("^");
        assert_eq!(sc.s(), format!("ಕ{D}"));
        assert_eq!(sc.backspaces_sent - before, 1);
    }

    #[test]
    fn accent_keys_are_literal_without_a_kannada_unit() {
        assert_eq!(typed("'"), "'");
        assert_eq!(typed("^"), "^");
        assert_eq!(typed("_"), "_");
        // Documented trade-off: right after a mapped letter the apostrophe IS an
        // accent, so English contractions need the engine switched off.
        assert_eq!(typed("don't"), format!("ದೊನ{U}ತ್"));
    }

    #[test]
    fn apostrophe_after_latin_text_is_literal() {
        // `q` is unmapped -> passes through and resets, so the apostrophe has
        // nothing to attach to.
        assert_eq!(typed("q'"), "q'");
        assert_eq!(typed("ka q'"), "ಕ q'");
        assert_eq!(typed("ka 'x"), "ಕ 'ಕ್ಷ್");
    }

    // ---- Robustness ----------------------------------------------------

    #[test]
    fn output_is_always_in_canonical_order() {
        // Property test (tiny LCG, no deps): for random keystroke soup, nothing
        // that must precede a Vedic mark ever follows one, and runs of marks are
        // sorted by combining class.
        let alphabet: Vec<char> = "kgjtmnrsaAiueEoMHKGSP:'_^\"\\`".chars().collect();
        let mut seed: u64 = 0xD1B54A32D192ED03;
        for _ in 0..5000 {
            let mut s = String::new();
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let n = 1 + (seed >> 40) as usize % 24;
            for _ in 0..n {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                s.push(alphabet[(seed >> 33) as usize % alphabet.len()]);
            }
            let out: Vec<char> = transliterate(&s).chars().collect();
            for w in out.windows(2) {
                if mapping::is_vedic_mark(w[0]) {
                    assert!(
                        w[1] != mapping::ANUSVARA && w[1] != mapping::VISARGA,
                        "sign after mark for input {s:?}"
                    );
                    assert!(
                        !(0x0CBE..=0x0CCC).contains(&(w[1] as u32)),
                        "matra after mark for input {s:?}"
                    );
                    if mapping::is_vedic_mark(w[1]) {
                        assert!(
                            mapping::combining_class(w[0]) <= mapping::combining_class(w[1]),
                            "marks out of order for input {s:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn incremental_equals_batch_for_random_input() {
        // Tiny LCG; no external crates. Inputs go up to 200 keys, so long words
        // cross MAX_BUFFER many times. Keys the engine passes through (accent
        // keys with nothing to attach to, a backtick at the start of a word, Q
        // after Latin text …) are checked too (0.5.3): the expected screen is
        // the batch rendering of each run of consumed keys, joined by the
        // passed keys, exactly as the engine resets on them.
        let alphabet: Vec<char> = "kgcjTDNtdnpbmyrlvwshLxSfaAiIuUeEoO~MHQZ:KGJPB`'_^\"\\q"
            .chars()
            .collect();
        let mut seed: u64 = 0x9E3779B97F4A7C15;
        for _ in 0..3000 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let n = 1 + (seed >> 33) as usize % 200;
            let mut s = String::new();
            for _ in 0..n {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                s.push(alphabet[(seed >> 33) as usize % alphabet.len()]);
            }
            let mut sc = Screen::default();
            let mut expected = String::new();
            let mut run = String::new();
            for c in s.chars() {
                if let Outcome::Pass = sc.key(c) {
                    expected.push_str(&transliterate(&run));
                    expected.push(c);
                    run.clear();
                } else {
                    run.push(c);
                }
            }
            expected.push_str(&transliterate(&run));
            assert_eq!(sc.s(), expected, "input {s:?}");
        }
    }

    #[test]
    fn buffer_limit_does_not_panic_or_corrupt() {
        let long = "ka".repeat(200);
        let out = typed(&long);
        assert_eq!(out.chars().filter(|&c| c == 'ಕ').count(), 200);
        // 200 bare consonants have no safe cut: the old reset still applies.
        let bare = "k".repeat(200);
        assert_eq!(typed(&bare).chars().filter(|&c| c == 'ಕ').count(), 200);
    }

    #[test]
    fn long_words_continue_past_the_buffer_limit() {
        // Before 0.5.3 a word restarted after 48 keys, so a vowel typed next
        // came out as an independent letter. Now the word continues.
        for word in [
            "ka".repeat(30) + "a",                    // vowel right at the limit
            "namaskaaramu".repeat(6),                 // ordinary long word
            "agni'mILE'purOhita'M".repeat(4),         // accents and anusvara
            "k".repeat(40) + "aiu" + &"ta".repeat(20), // long consonant run first
            "saMskRuta".repeat(8) + "M'",             // sign + accent after the limit
        ] {
            assert_eq!(typed(&word), transliterate(&word), "word {word:?}");
        }
        // An accent right after the cut still attaches to the syllable before it.
        let w = "ka".repeat(24) + "'";
        assert_eq!(typed(&w), "ಕ".repeat(24) + "\u{0951}");
    }

    #[test]
    fn backspace_after_the_cut_only_undoes_the_live_part() {
        let mut sc = Screen::default();
        sc.type_str(&"ka".repeat(30)); // 60 keys: the first ones were committed
        assert!(!sc.engine.is_idle());
        for _ in 0..200 {
            sc.backspace(); // live keys first, then native backspaces
        }
        assert_eq!(sc.s(), "");
        assert!(sc.engine.is_idle());
        // A native backspace forgets the committed start: an accent now passes.
        sc.type_str(&"ka".repeat(30));
        while !sc.engine.is_idle() {
            sc.backspace();
        }
        sc.backspace();
        assert_eq!(sc.key('\''), Outcome::Pass);
    }

    #[test]
    fn signs_with_nothing_to_sit_on_stay_literal() {
        assert_eq!(typed("M"), "M");
        assert_eq!(typed("H"), "H");
        assert_eq!(typed("ZM"), "ಽM");
        assert_eq!(typed(":hH"), "ೱH");
        assert_eq!(typed("Ma"), "Mಅ");
        // With something before them they are signs, as always.
        assert_eq!(typed("aM"), "ಅಂ");
        assert_eq!(typed("kaH"), "ಕಃ");
        assert_eq!(typed("ka`M"), "ಕಂ");
        // Space ends the word, so a sign after it is literal again.
        assert_eq!(typed("ka M"), "ಕ M");
    }

    #[test]
    fn backtick_at_the_start_of_a_word_is_a_real_backtick() {
        assert_eq!(typed("`"), "`");
        assert_eq!(typed("``"), "``");
        assert_eq!(typed("ka `x`"), "ಕ `ಕ್ಷ್");
        // Inside a word it is still the break key.
        assert_eq!(typed("s`ha"), "ಸ್ಹ");
        assert_eq!(typed("c`H"), "\u{0C9A}\u{0CCD}\u{0C83}");
    }

    #[test]
    fn render_never_panics_on_odd_input() {
        for s in ["", "~", ":", "RR", "'", "^^^", "ಕ", "k😀a", "\u{0}", "`", "``a", "c`", "g`M`"] {
            let _ = render(s);
        }
    }

    #[test]
    fn diff_basics() {
        assert_eq!(
            diff("", "ಕ"),
            Edit {
                backspaces: 0,
                text: "ಕ".into()
            }
        );
        assert_eq!(
            diff("ಕ್", "ಕ"),
            Edit {
                backspaces: 1,
                text: "".into()
            }
        );
        assert_eq!(
            diff("ಕೆ", "ಕೀ"),
            Edit {
                backspaces: 1,
                text: "ೀ".into()
            }
        );
        assert!(diff("ಕ", "ಕ").is_noop());
    }

    #[test]
    fn agama_anusvara_comes_only_from_g_break_m() {
        // Break key + M after a bare ga: the single sign U+A8F3 (it contains the ga).
        assert_eq!(render("g`M").text, "\u{A8F3}");
        assert_eq!(render("g`M'").text, "\u{A8F3}\u{0951}");
        assert_eq!(render("g`M_").text, "\u{A8F3}\u{0952}");
        assert_eq!(render("ag`M").text, "\u{0C85}\u{A8F3}");
        // Without the break key, gM is the ordinary full letter + anusvara, like kM.
        assert_eq!(render("gM").text, "\u{0C97}\u{0C82}");
        assert_eq!(render("gM'").text, "\u{0C97}\u{0C82}\u{0951}");
        assert_eq!(render("gaM").text, "\u{0C97}\u{0C82}");
        // MM has no special meaning any more.
        assert_eq!(render("gMM").text, "\u{0C97}\u{0C82}\u{0C82}");
        assert!(!render("saMskRuta").text.contains('\u{A8F3}'));
        // Other letters never give the agama sign.
        assert_eq!(render("k`M").text, "\u{0C95}\u{0CCD}\u{0C82}");
    }

    #[test]
    fn break_key_keeps_the_half_consonant_before_signs() {
        for (typed_, want) in [
            ("c`H", "\u{0C9A}\u{0CCD}\u{0C83}"),
            ("k`M", "\u{0C95}\u{0CCD}\u{0C82}"),
            ("c`M", "\u{0C9A}\u{0CCD}\u{0C82}"),
            ("g`H", "\u{0C97}\u{0CCD}\u{0C83}"),
            // Without the break key: the full consonant, as before.
            ("cH", "\u{0C9A}\u{0C83}"),
            ("kM", "\u{0C95}\u{0C82}"),
            ("cM", "\u{0C9A}\u{0C82}"),
            ("gH", "\u{0C97}\u{0C83}"),
        ] {
            assert_eq!(render(typed_).text, want, "typed {typed_:?}");
        }
        // The half form is stable under later keys and accents.
        assert_eq!(render("c`H'").text, "\u{0C9A}\u{0CCD}\u{0C83}\u{0951}");
        // A vowel before the break key leaves nothing to keep.
        assert_eq!(render("ka`M").text, "\u{0C95}\u{0C82}");
    }

    #[test]
    fn repeated_break_key_is_ignored() {
        assert_eq!(typed("c``H"), typed("c`H"));
        assert_eq!(typed("g```M"), "\u{A8F3}");
        let mut e = Engine::new();
        e.on_char('c');
        e.on_char('`');
        assert!(matches!(e.on_char('`'), Outcome::Consumed(ed) if ed.is_noop()));
    }

    #[test]
    fn vedic_mode_off_turns_accents_and_agama_off() {
        fn typed_plain(s: &str) -> String {
            let mut sc = Screen::default();
            sc.engine.set_vedic(false);
            sc.type_str(s);
            sc.s()
        }
        // Accent keys are ordinary characters.
        assert_eq!(typed_plain("ka'"), "ಕ'");
        assert_eq!(typed_plain("ka_"), "ಕ_");
        assert_eq!(typed_plain("ka\""), "ಕ\"");
        assert_eq!(typed_plain("ka^"), "ಕ^");
        assert_eq!(typed_plain("don't"), "ದೊನ್'ತ್");
        // No agama sign: the half letter is plain ಗ್ಂ.
        assert_eq!(typed_plain("g`M"), "\u{0C97}\u{0CCD}\u{0C82}");
        // Everything else is unchanged.
        assert_eq!(typed_plain("namaste"), "ನಮಸ್ತೆ");
        assert_eq!(typed_plain("kM"), "\u{0C95}\u{0C82}");
        assert_eq!(typed_plain("c`H"), "\u{0C9A}\u{0CCD}\u{0C83}");
        // Switching back on restores the accents.
        let mut sc = Screen::default();
        sc.engine.set_vedic(false);
        sc.engine.set_vedic(true);
        sc.type_str("ka'");
        assert_eq!(sc.s(), format!("ಕ{U}"));
    }

    #[test]
    fn new_capital_keys_and_old_spellings() {
        for (new_, old, want) in [
            ("Ka", "kha", "ಖ"),
            ("Ga", "gha", "ಘ"),
            ("Ja", "jha", "ಝ"),
            ("THa", "Tha", "ಠ"),
            ("DHa", "Dha", "ಢ"),
            ("Pa", "pha", "ಫ"),
            ("Ba", "bha", "ಭ"),
            ("Sa", "sha", "ಶ"),
        ] {
            assert_eq!(typed(new_), want, "{new_}");
            assert_eq!(typed(old), want, "{old}");
        }
        assert_eq!(typed("Sha"), "ಷ");
        assert_eq!(typed("SHa"), "ಷ");
        assert_eq!(typed("tha"), "ಥ"); // lowercase th / dh stay ಥ / ಧ
        assert_eq!(typed("dha"), "ಧ");
        assert_eq!(typed("Ta"), "ಟ");
        assert_eq!(typed("Da"), "ಡ");
        // A visarga after ಟ ಡ ಶ needs the vowel: the capital pairs are letters.
        assert_eq!(typed("SaH"), "ಶಃ");
        assert_eq!(typed("TaH"), "ಟಃ");
        assert_eq!(typed("DaH"), "ಡಃ");
        // Conjuncts through the new keys.
        assert_eq!(typed("kSha"), "ಕ್ಷ");
        assert_eq!(typed("Siva"), "ಶಿವ");
    }

    #[test]
    fn short_spellings_for_vocalic_r() {
        assert_eq!(render("Ru").text, "\u{0C8B}");
        assert_eq!(render("RU").text, "\u{0CE0}");
        assert_eq!(render("RRi").text, render("Ru").text);
        assert_eq!(render("kRu").text, "\u{0C95}\u{0CC3}");
        assert_eq!(render("kRU").text, "\u{0C95}\u{0CC4}");
        assert_eq!(render("saMskRRita").text, render("saMskRuta").text);
    }
}
