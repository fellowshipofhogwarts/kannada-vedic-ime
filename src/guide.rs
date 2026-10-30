//! guide.rs — builds the "Phonetic & Vedic quick reference" page straight from
//! the mapping tables, so the guide can never drift from real behaviour.
//! The tray menu writes this HTML to the temp dir and opens it in the browser.

use crate::engine::transliterate;
use crate::mapping::{ACCENTS, CONSONANTS, NUKTA, OTHERS, SIGNS, VOWELS};
use std::fmt::Write as _;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn keys(list: &[&str]) -> String {
    list.iter()
        .map(|k| format!("<kbd>{}</kbd>", esc(k)))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn html() -> String {
    let mut h = String::with_capacity(16 * 1024);
    h.push_str(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8">
<title>Kannada IME — quick reference</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
:root{--bg:#fff;--fg:#1c1c1e;--mut:#6b6b70;--line:#e3e3e8;--acc:#d97706}
@media(prefers-color-scheme:dark){:root{--bg:#18181b;--fg:#ececf0;--mut:#9a9aa3;--line:#2f2f36;--acc:#fbbf24}}
body{font:15px/1.5 system-ui,sans-serif;background:var(--bg);color:var(--fg);max-width:880px;margin:2rem auto;padding:0 1rem}
h1{font-size:1.5rem}h2{margin-top:2rem;font-size:1.1rem;border-bottom:1px solid var(--line);padding-bottom:.3rem}
table{border-collapse:collapse;width:100%}td,th{padding:.35rem .6rem;border-bottom:1px solid var(--line);text-align:left}
th{color:var(--mut);font-weight:600;font-size:.8rem;text-transform:uppercase}
kbd{font:600 .85rem ui-monospace,monospace;background:var(--line);border-radius:4px;padding:.05rem .4rem}
.kn{font-family:"Arkavathi","Noto Sans Kannada","Nirmala UI","Tunga","Sampige",sans-serif;font-size:1.25rem}
.note{color:var(--mut)}code{font-family:ui-monospace,monospace}
</style></head><body>
<h1>Kannada phonetic typing — quick reference</h1>
<p class="note">Toggle with <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>K</kbd> or the tray menu.
Baraha-style scheme. Consonants appear with a virama (ಕ್) until a vowel arrives;
<kbd>a</kbd> removes it (ಕ), other vowels replace it with a vowel sign.
Capital letters give the aspirates: <kbd>K</kbd> ಖ, <kbd>G</kbd> ಘ, <kbd>J</kbd> ಝ,
<kbd>TH</kbd> ಠ, <kbd>DH</kbd> ಢ, <kbd>P</kbd> ಫ, <kbd>B</kbd> ಭ, <kbd>S</kbd> ಶ,
<kbd>Sh</kbd> or <kbd>SH</kbd> ಷ. The older spellings (<kbd>kh</kbd>, <kbd>ph</kbd> …)
still work. <kbd>M</kbd> or <kbd>H</kbd> right after a bare consonant gives the full letter
(<code>kM</code> ಕಂ); put the break key in between for the half letter
(<code>k`M</code> ಕ್ಂ, <code>c`H</code> ಚ್ಃ). A visarga after ಶ ಟ ಡ needs the vowel:
<code>SaH</code>, <code>TaH</code>, <code>DaH</code>.
<kbd>:g</kbd> and <kbd>:j</kbd> give ಙ and ಞ; <kbd>zh</kbd> gives ೞ; <kbd>Q</kbd> right after a consonant adds the nukta (<code>jQa</code> ಜ಼).</p>
"#,
    );

    // Vowels
    h.push_str("<h2>Vowels</h2><table><tr><th>Type</th><th>Alone</th><th>After ಕ</th></tr>");
    for (ks, ind, matra) in VOWELS {
        let after = match matra {
            Some(m) => format!("ಕ{m}"),
            None => "ಕ".to_string(),
        };
        let _ = write!(
            h,
            "<tr><td>{}</td><td class=kn>{}</td><td class=kn>{}</td></tr>",
            keys(ks),
            ind,
            after
        );
    }
    for (ks, s) in SIGNS {
        let _ = write!(
            h,
            "<tr><td>{}</td><td class=kn>ಅ{s}</td><td class=kn>ಕ{s}</td></tr>",
            keys(ks)
        );
    }
    // g`M: the single agama sign. It already contains the ga, so typed alone it
    // is just ꣳ, and after ಕ (`kag`M`) it is ಕꣳ. Both come from the engine.
    let _ = write!(
        h,
        "<tr><td><kbd>g</kbd> <kbd>`</kbd> <kbd>M</kbd> <span class=note>(Vedic mode)</span></td>\
<td class=kn>{}</td><td class=kn>{}</td></tr>",
        transliterate("g`M"),
        transliterate("kag`M")
    );
    h.push_str("</table>");

    // Other letters and signs
    h.push_str("<h2>Other signs</h2><table><tr><th>Type</th><th>Result</th></tr>");
    for (ks, ch) in OTHERS {
        let _ = write!(h, "<tr><td>{}</td><td class=kn>{}</td></tr>", keys(ks), ch);
    }
    let _ = write!(
        h,
        "<tr><td><kbd>Z</kbd> <kbd>Z</kbd></td><td class=kn>ಽಽ</td></tr>\
<tr><td><kbd>j</kbd> <kbd>Q</kbd> <kbd>a</kbd></td><td class=kn>ಜ{NUKTA}</td></tr>"
    );
    h.push_str("</table>");

    // Consonants
    h.push_str("<h2>Consonants</h2><table><tr><th>Type</th><th>Result</th></tr>");
    for (ks, cluster) in CONSONANTS {
        let _ = write!(
            h,
            "<tr><td>{}</td><td class=kn>{}</td></tr>",
            keys(ks),
            cluster
        );
    }
    h.push_str("</table>");

    // Vedic
    h.push_str(
        "<h2>Vedic accents</h2>\
<p class=note>Type the accent key <b>right after</b> the syllable it belongs to. \
It is attached as a non-spacing combining mark after the consonant or vowel sign, \
also after conjuncts (ottaksharas). With nothing Kannada before it, the key is just \
punctuation. A bare consonant followed by an accent implies the inherent <i>a</i>. \
The stored order is always base, vowel sign, anusvara, accent, whichever you typed first \
(<code>ka'M</code> and <code>kaM'</code> both give the same text).</p>\
<table><tr><th>Type</th><th>Mark</th><th>Code point</th><th>Example</th></tr>",
    );
    for (ks, mark, name) in ACCENTS {
        let example_in = format!("agni{}", ks[0]);
        let example_out = transliterate(&example_in);
        let _ = write!(
            h,
            "<tr><td>{}</td><td>{}</td><td><code>U+{:04X}</code></td>\
<td><code>{}</code> → <span class=kn>{}</span></td></tr>",
            keys(ks),
            esc(name),
            *mark as u32,
            esc(&example_in),
            example_out
        );
    }
    h.push_str(
        "</table><p class=note><kbd>^</kbd> then another <kbd>^</kbd> upgrades Udatta to \
double Svarita in place. Accent glyphs need a font that carries U+0951, U+0952 and \
U+1CDA; if they show as boxes, install a Vedic-capable Kannada font.</p>",
    );

    // Tips
    h.push_str(
        "<h2>Tips</h2><ul>\
<li><kbd>`</kbd> (backtick) is a break: <code>s`ha</code> → ಸ್ಹ instead of ಶ. On a German keyboard <kbd>Shift</kbd>+<kbd>´</kbd> works as the break key without Space. At the start of a word it types a normal backtick.</li>\
<li><kbd>M</kbd>, <kbd>H</kbd> and <kbd>:M</kbd> need a letter before them; at the start of a word they type M, H and :M.</li>\
<li>Vedic mode (tray menu) switches the accent keys and ꣳ on or off. Off, the keys <kbd>'</kbd> <kbd>&quot;</kbd> <kbd>_</kbd> <kbd>\\</kbd> <kbd>^</kbd> type normally. On a German keyboard <kbd>\\</kbd> is <kbd>AltGr</kbd>+<kbd>ß</kbd>.</li>\
<li><kbd>Backspace</kbd> undoes the last <i>keystroke</i> of the current word.</li>\
<li>The number keys type Kannada digits ೦ ೧ ೨ … ೯ (numpad too). Untick \"Kannada digits\" in the tray menu for 0 1 2 … 9.</li>\
<li>Space, Enter, arrows, mouse clicks and shortcuts end the current word.</li>\
<li>Caps Lock is ignored; use <kbd>Shift</kbd> for capital letters (<kbd>T</kbd> ಟ, <kbd>t</kbd> ತ).</li>\
<li>Shortcuts (Ctrl/Alt/Win) always pass straight through.</li>\
</ul></body></html>",
    );
    h
}

#[cfg(test)]
mod tests {
    #[test]
    fn guide_renders_and_mentions_every_accent() {
        let h = super::html();
        assert!(h.contains("U+0951") && h.contains("U+0952") && h.contains("U+1CDA"));
        assert!(h.contains("ಕ್ಷ"));
        assert!(h.len() > 4000);
    }
}

#[cfg(test)]
mod tests_053 {
    use super::*;

    #[test]
    fn agama_row_shows_the_real_output() {
        let h = html();
        // Alone: just ꣳ (it contains the ga); after ಕ: ಕꣳ. Never ಅꣳ or ಗꣳ.
        assert!(h.contains("<td class=kn>\u{A8F3}</td><td class=kn>\u{0C95}\u{A8F3}</td>"));
        assert!(!h.contains("\u{0C97}\u{A8F3}"));
    }

    #[test]
    fn vedic_off_tip_lists_every_single_accent_key() {
        let h = html();
        for k in ["'", "&quot;", "_", "\\", "^"] {
            assert!(h.contains(&format!("<kbd>{k}</kbd> ")), "missing key {k}");
        }
    }
}
