//! mapping.rs — Baraha-style phonetic tables for Kannada plus Vedic accents.
//!
//! Pure data + one lazily-built lookup table. No dependencies, no I/O, so it can
//! be unit-tested anywhere. Everything the engine and the quick-reference guide
//! need is derived from the `static` tables below — edit them here and both the
//! typing behaviour and the generated guide follow.
//!
//! Code points (Unicode Kannada block U+0C80..U+0CFF):
//!   * Virama / halant is **U+0CCD** (್).  (U+0CDD is *not* the virama.)
//!   * Anusvara U+0C82 (ಂ), Visarga U+0C83 (ಃ), Nukta U+0CBC.

use std::collections::HashMap;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Special code points
// ---------------------------------------------------------------------------

pub const VIRAMA: char = '\u{0CCD}';
pub const ANUSVARA: char = '\u{0C82}';
pub const VISARGA: char = '\u{0C83}';

/// Vedic accents are non-spacing combining marks appended *after* the base
/// consonant / vowel sign. They are script-neutral Unicode marks that live
/// outside the Kannada block, so the font in use must carry them.
pub const UDATTA: char = '\u{0951}'; // DEVANAGARI STRESS SIGN UDATTA
pub const ANUDATTA: char = '\u{0952}'; // DEVANAGARI STRESS SIGN ANUDATTA
pub const DOUBLE_SVARITA: char = '\u{1CDA}'; // VEDIC TONE DOUBLE SVARITA

// ---------------------------------------------------------------------------
// Tables.  Each entry lists every roman spelling (aliases) of one sound.
// ---------------------------------------------------------------------------

/// (roman spellings, independent vowel, dependent vowel sign / matra).
/// The inherent vowel "a" has no matra: typing it after a consonant just
/// removes the pending virama.
pub static VOWELS: &[(&[&str], char, Option<char>)] = &[
    (&["a"], '\u{0C85}', None),
    (&["aa", "A"], '\u{0C86}', Some('\u{0CBE}')),
    (&["i"], '\u{0C87}', Some('\u{0CBF}')),
    (&["ii", "I", "ee"], '\u{0C88}', Some('\u{0CC0}')),
    (&["u"], '\u{0C89}', Some('\u{0CC1}')),
    (&["uu", "U", "oo"], '\u{0C8A}', Some('\u{0CC2}')),
    (&["RRi", "Ru"], '\u{0C8B}', Some('\u{0CC3}')),
    (&["RRI", "RU"], '\u{0CE0}', Some('\u{0CC4}')),
    (&["e"], '\u{0C8E}', Some('\u{0CC6}')),
    (&["E"], '\u{0C8F}', Some('\u{0CC7}')),
    (&["ai"], '\u{0C90}', Some('\u{0CC8}')),
    (&["o"], '\u{0C92}', Some('\u{0CCA}')),
    (&["O"], '\u{0C93}', Some('\u{0CCB}')),
    (&["au"], '\u{0C94}', Some('\u{0CCC}')),
];

/// (roman spellings, consonant cluster string *without* trailing virama).
/// Multi-code-point values are legitimate: ksha, jna, nukta forms.
pub static CONSONANTS: &[(&[&str], &str)] = &[
    (&["k"], "\u{0C95}"),
    (&["K", "kh"], "\u{0C96}"),
    (&["g"], "\u{0C97}"),
    (&["G", "gh"], "\u{0C98}"),
    (&["~g", ":g"], "\u{0C99}"),
    (&["c", "ch"], "\u{0C9A}"),
    (&["C", "Ch", "chh"], "\u{0C9B}"),
    (&["j"], "\u{0C9C}"),
    (&["J", "jh"], "\u{0C9D}"),
    (&["~j", ":j"], "\u{0C9E}"),
    (&["T"], "\u{0C9F}"),
    (&["TH", "Th"], "\u{0CA0}"),
    (&["D"], "\u{0CA1}"),
    (&["DH", "Dh"], "\u{0CA2}"),
    (&["N"], "\u{0CA3}"),
    (&["t"], "\u{0CA4}"),
    (&["th"], "\u{0CA5}"),
    (&["d"], "\u{0CA6}"),
    (&["dh"], "\u{0CA7}"),
    (&["n"], "\u{0CA8}"),
    (&["p"], "\u{0CAA}"),
    (&["P", "ph"], "\u{0CAB}"),
    (&["b"], "\u{0CAC}"),
    (&["B", "bh"], "\u{0CAD}"),
    (&["m"], "\u{0CAE}"),
    (&["y"], "\u{0CAF}"),
    (&["r"], "\u{0CB0}"),
    (&["l"], "\u{0CB2}"),
    (&["L"], "\u{0CB3}"),
    (&["v", "w"], "\u{0CB5}"),
    (&["S", "sh"], "\u{0CB6}"),
    (&["Sh", "SH"], "\u{0CB7}"),
    (&["s"], "\u{0CB8}"),
    (&["h"], "\u{0CB9}"),
    // Conjunct shortcuts (virama is part of the cluster string).
    (&["x"], "\u{0C95}\u{0CCD}\u{0CB7}"),  // ಕ್ಷ  ksha
    (&["jn"], "\u{0C9C}\u{0CCD}\u{0C9E}"), // ಜ್ಞ  jna
    // Loan-word nukta form.
    (&["f"], "\u{0CAB}\u{0CBC}"), // ಫ಼
];

/// Anusvara / visarga.
pub static SIGNS: &[(&[&str], char)] = &[(&["M"], ANUSVARA), (&["H"], VISARGA)];

/// Agama anusvara U+A8F3 (a Devanagari-block sign that Vedic Kannada texts use).
/// It already contains the ga. The engine produces it only from the keys
/// `g`, break key, `M` while Vedic mode is on (see engine.rs); it has no
/// spelling of its own. A Vedic mark typed after it attaches to it. Needs a font
/// that positions marks on it (see TYPOGRAPHY.md, Goda fix).
pub const AGAMA_ANUSVARA: char = '\u{A8F3}';

/// (trigger spellings, combining mark, display name).
///
/// `"` and `^^` both give DOUBLE_SVARITA. A single `^` is Udatta, so typing a
/// second `^` *upgrades* the mark that was just shown (the engine re-renders
/// the whole buffer, so the diff is "backspace one, type the new mark").
pub static ACCENTS: &[(&[&str], char, &str)] = &[
    (&["'", "^"], UDATTA, "Udatta"),
    (&["_", "\\"], ANUDATTA, "Anudatta"),
    (&["\"", "^^"], DOUBLE_SVARITA, "Svarita / double svarita"),
];

/// True for the Vedic combining marks this crate can emit (the `ACCENTS` marks).
pub fn is_vedic_mark(c: char) -> bool {
    ACCENTS.iter().any(|(_, mark, _)| *mark == c)
}

/// Unicode canonical combining class (UCD `ccc`) of the marks this crate emits.
/// Verified against the UCD: U+0951 = 230 (above), U+0952 = 220 (below),
/// U+1CDA = 230 (above); everything else we emit is class 0 (or the virama, 9,
/// which is always removed before a mark is attached).
///
/// Used to keep runs of stacked marks in *canonical order* (lower class first),
/// so the output is already NFC/NFD-stable: normalising it never reorders it.
pub fn combining_class(c: char) -> u8 {
    match c {
        UDATTA | DOUBLE_SVARITA => 230,
        ANUDATTA => 220,
        _ => 0,
    }
}

/// Segmentation break: consumed, emits nothing, but stops the next letters from
/// merging into the previous token (`s`h` = ಸ್ಹ instead of ಶ). Before `M` or `H`
/// it keeps the half consonant (`c`H` = ಚ್ಃ).
pub const BREAK_KEY: &str = "`";

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Vowel {
        independent: char,
        matra: Option<char>,
    },
    Consonant(&'static str),
    Sign(char),
    Accent(char),
    Break,
}

/// Longest roman token in the tables, in bytes.
pub const MAX_TOKEN_LEN: usize = 3;

/// roman spelling -> token. Built once; panics at first use (and in tests) if
/// two entries claim the same spelling.
pub fn table() -> &'static HashMap<&'static str, Tok> {
    static T: OnceLock<HashMap<&'static str, Tok>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m: HashMap<&'static str, Tok> = HashMap::new();
        let mut put = |k: &'static str, t: Tok| {
            assert!(
                k.len() <= MAX_TOKEN_LEN,
                "token {k:?} longer than MAX_TOKEN_LEN"
            );
            assert!(m.insert(k, t).is_none(), "duplicate roman token {k:?}");
        };
        for (keys, independent, matra) in VOWELS {
            for k in *keys {
                put(
                    k,
                    Tok::Vowel {
                        independent: *independent,
                        matra: *matra,
                    },
                );
            }
        }
        for (keys, cluster) in CONSONANTS {
            for k in *keys {
                put(k, Tok::Consonant(cluster));
            }
        }
        for (keys, sign) in SIGNS {
            for k in *keys {
                put(k, Tok::Sign(*sign));
            }
        }
        for (keys, mark, _) in ACCENTS {
            for k in *keys {
                put(k, Tok::Accent(*mark));
            }
        }
        put(BREAK_KEY, Tok::Break);
        m
    })
}

/// True for the single-character keys that trigger a Vedic accent.
pub fn is_accent_key(c: char) -> bool {
    ACCENTS
        .iter()
        .any(|(keys, _, _)| keys.iter().any(|k| k.len() == 1 && k.starts_with(c)))
}

/// True if `c` can start (or be part of) a phonetic token and is not an accent
/// trigger. These are the keys the engine swallows while active.
pub fn is_phonetic_key(c: char) -> bool {
    static K: OnceLock<[bool; 128]> = OnceLock::new();
    let k = K.get_or_init(|| {
        let mut a = [false; 128];
        for key in table().keys() {
            a[key.as_bytes()[0] as usize] = true;
        }
        for (keys, _, _) in ACCENTS {
            for key in *keys {
                a[key.as_bytes()[0] as usize] = false;
            }
        }
        a
    });
    (c as u32) < 128 && k[c as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_builds_without_duplicates() {
        // 21 vowel + 52 consonant (incl. x, jn, f) + 2 sign + 6 accent + 1 break
        // spellings. Update this number when you edit the tables.
        assert_eq!(table().len(), 82);
    }

    #[test]
    fn mark_classes_match_the_ucd() {
        assert_eq!(combining_class(UDATTA), 230);
        assert_eq!(combining_class(ANUDATTA), 220);
        assert_eq!(combining_class(DOUBLE_SVARITA), 230);
        assert_eq!(combining_class('\u{0C95}'), 0);
        assert!(is_vedic_mark(UDATTA) && is_vedic_mark(ANUDATTA) && is_vedic_mark(DOUBLE_SVARITA));
        assert!(!is_vedic_mark(ANUSVARA) && !is_vedic_mark('\u{0CBE}'));
    }

    #[test]
    fn virama_is_u0ccd() {
        assert_eq!(VIRAMA as u32, 0x0CCD);
    }

    #[test]
    fn key_classes() {
        for c in "kgRaAMH~:`xjSsfTKGJPB".chars() {
            assert!(is_phonetic_key(c), "{c} should be phonetic");
        }
        for c in "'\"^_\\".chars() {
            assert!(is_accent_key(c), "{c} should be an accent key");
            assert!(!is_phonetic_key(c), "{c} must not be phonetic");
        }
        for c in " 1.,?!;QqZzWYXFV".chars() {
            assert!(!is_phonetic_key(c), "{c:?} should pass through");
        }
    }

    #[test]
    fn all_codepoints_in_expected_blocks() {
        for (_, ind, matra) in VOWELS {
            assert!((0x0C80..=0x0CFF).contains(&(*ind as u32)));
            if let Some(m) = matra {
                assert!((0x0C80..=0x0CFF).contains(&(*m as u32)));
            }
        }
        for (_, cl) in CONSONANTS {
            assert!(cl.chars().all(|c| (0x0C80..=0x0CFF).contains(&(c as u32))));
        }
        for (_, mark, _) in ACCENTS {
            let u = *mark as u32;
            assert!(u == 0x0951 || u == 0x0952 || (0x1CD0..=0x1CFF).contains(&u));
        }
    }
}
