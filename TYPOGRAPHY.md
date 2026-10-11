# Typography & font setup for Vedic Kannada

`kannada_ime` emits plain Unicode: base consonant → matra → anusvara → Vedic mark
(U+0951 Udatta, U+0952 Anudatta, U+1CDA double Svarita). Whether the result
*looks* right is decided by the font and the application's line layout. This
guide covers the two things that go wrong most often: accents colliding with
neighbouring lines, and accents sitting in the wrong place on a custom font.

Test strings (typed keys → result, produced by the engine itself):

| Keys | Result |
|---|---|
| `kaM'` | ಕಂ॑ |
| `kiM_` | ಕಿಂ॒ |
| `kShE"` | ಕ್ಷೇ᳚ |
| `tatva^^` | ತತ್ವ᳚ |
| `saMskRRi_` | ಸಂಸ್ಕೃ॒ |
| `agni'mILE'` | ಅಗ್ನಿ॑ಮೀಳೇ॑ |

> I could not test any real font in the authoring environment (no Kannada Vedic
> font was available), so everything below is standard application/font-tool
> behaviour, not something verified against Goda, Kedage or any specific font.
> Menu names can differ slightly between versions.

---

## 0. Check the font first

The font must contain glyphs for U+0951, U+0952 and U+1CDA, plus GPOS
mark-positioning for them. Check coverage with fontTools (`pip install fonttools`):

```bash
python -c "from fontTools.ttLib import TTFont; import sys; \
c=TTFont(sys.argv[1]).getBestCmap(); \
print({hex(u): u in c for u in (0x951,0x952,0x1CDA,0xC82)})" YourFont.ttf
```

If a mark is missing, the application silently borrows it from a fallback font
(often a Devanagari one). Those glyphs have different metrics and anchors, which
is the main cause of uneven line heights (section 1.1). Prefer a font that
covers them, or add them in FontForge (section 2).

---

## 1. Line height and spacing

### 1.1 Why 1.5× prevents collisions and "line jumping"

* **Top marks need headroom.** Udatta and Svarita are positioned above the base
  glyph's top. On a syllable that already has a tall vowel sign (ಿ ೀ ೆ ೇ ೊ) the mark
  sits higher still. Default "single" spacing comes from the font's
  ascent/descent metrics, which are sized for ordinary text, not for stacked marks.
  The mark then touches or overlaps the line above.
* **The line above has things hanging below the baseline.** Ottaksharas
  (subjoined conjunct forms) and the Anudatta line extend below the baseline, so
  the clash is between the next line's top mark and this line's bottom
  ornaments.
* **Irregular gaps.** A line is as tall as its tallest glyph. If a mark comes
  from a fallback font with larger ascent/descent, only lines containing that
  mark grow, so spacing between lines varies and the text appears to jump.
  Generous proportional spacing absorbs much of this; fixed spacing removes it
  entirely.
* **Fixed spacing can clip.** "Exactly" / "Fixed" line spacing makes every line
  the same height, but glyph parts that stick out of that height may be cut off.
  Choose a fixed value of **at least 1.5× the font size** (12 pt text → 18 pt;
  14 pt → 21 pt; 16 pt → 24 pt) and prefer "At least" where it exists.

1.5 is a starting value, not a rule. If marks still touch, raise it (1.7–2.0
for heavily accented text) and judge by eye in your font.

### 1.2 Microsoft Word

1. Select the text (`Ctrl+A` for everything).
2. **Home →** click the small arrow at the bottom-right of the **Paragraph**
   group → **Indents and Spacing → Spacing → Line spacing**.
3. Choose **Multiple**, set **At: 1.5** (or another value). Alternative: choose
   **At least** with 18–24 pt. Use **Exactly** only with the clipping risk in mind.
4. To apply it everywhere: right-click the **Normal** style in the Styles
   gallery → **Modify… → Format → Paragraph** and set the same value.
5. Kannada is a complex script, so Word uses the *Complex scripts* font slot:
   **Home → Font dialog (`Ctrl+D`) → Font tab → Complex scripts → Font / Size**.
   Set your Vedic-capable font there, not only under "Latin text".

### 1.3 LibreOffice Writer

1. **Format → Paragraph… → Indents & Spacing → Line Spacing.**
2. Choose **Proportional** and enter **150%**. Alternatives: **At least** or
   **Fixed** with 18–24 pt (Fixed can clip).
3. To apply it document-wide, edit the style instead: `F11` → right-click
   **Default Paragraph Style → Edit Style… → Indents & Spacing**.
4. Enable complex text layout once: **Tools → Options → Language Settings →
   Languages → Enhanced language support → Enabled for complex text layout
   (CTL)**. Then set **Format → Character → Font → CTL Font** to your font.

### 1.4 Web CSS

```css
:lang(kn), .kn {
  font-family: "YourVedicKannadaFont", "Noto Sans Kannada", sans-serif;
  line-height: 1.5;          /* unitless: scales with font-size */
  letter-spacing: normal;    /* non-zero letter-spacing makes browsers drop ligatures */
}
```

* Mark the language: `<html lang="kn">` or `<span lang="kn">…</span>`.
* Don't put Vedic text in a box with `overflow: hidden` and a tight height;
  add `padding-block` so top marks are not clipped.
* Optional metric overrides for a self-hosted font (they change where glyphs sit
  in the line box and what `line-height: normal` means; supported in current
  browsers, verify yours):

```css
@font-face {
  font-family: "YourVedicKannadaFont";
  src: url("fonts/YourFont.woff2") format("woff2");
  ascent-override: 110%;    /* headroom for stacked top marks   */
  descent-override: 45%;    /* room for ottaksharas + Anudatta  */
  line-gap-override: 0%;
}
```

### 1.5 Inkscape

1. Select the **Text tool** (`T`) and click the text object.
2. Set **Line spacing** in the tool-controls bar, or in **Text → Text and Font…
   (`Ctrl+Shift+T`)**. In Inkscape 1.x set **150 %** (equivalently 1.5 in
   unitless "lines" mode, depending on the unit selector shown next to the field).
   Equivalent SVG: `style="line-height:1.5"` on the `<text>`.
3. Use Inkscape 1.x. Older 0.92 builds shape complex scripts less reliably.
4. Check the result **before** Path → Object to Path, which freezes the shaping.
   Re-edit the text object if a mark is misplaced.

---

## 2. FontForge: adjusting GPOS anchor points for Vedic marks

Use this to move or resize Udatta / Anudatta / Svarita on a font such as IISc
Goda or Kedage when marks sit too high, too low, off-centre or too large.

**Before you start**

* Check the font's licence permits modification (SIL OFL does, with a Reserved
  Font Name rule; some fonts do not). Modify a *copy* and rename it (step 9).
* Positioning is **mark-to-base**: the mark glyph is shifted so that *its*
  anchor lands exactly on the *base* glyph's anchor of the same anchor class.
  Hence: the **mark's** anchor decides the global offset for every base, the
  **base's** anchors fine-tune individual letters.
* Feature tags and lookup names differ per font, so step 2 is how you find out
  what yours uses.

**Steps**

1. **File → Open** your `.ttf`/`.otf` (for a `.ttc`, pick the face).
2. **Find the existing lookups.** **Element → Font Info… (`Ctrl+Shift+F`) →
   Lookups → GPOS** tab. Look for lookups of type **Mark to Base Attachment**
   (usually features `abvm` for marks above, `blwm` for below, or `mark`; script
   Kannada = `knd2` / `knda`). Expand one to see its subtables and **anchor
   classes** (names like `Above`, `Below`). If none covers U+0951/U+0952/U+1CDA,
   see step 8.
3. **Open the mark glyph.** **Encoding → Goto…**, enter `0951` (then `0952`,
   `1CDA`), open the glyph (double-click). A mark glyph has an anchor point of
   type **Mark** in one of those classes, drawn as a labelled marker.
4. **Read or edit the anchor.** Select the anchor, then **Point → Get Info…
   (`Ctrl+I`)**. The *Anchor Point* dialog shows **Anchor Class**, **Type**,
   **X** and **Y**. Type exact numbers (font units) or drag it.
5. **Move the accent vertically for all letters:** change the **Y of the mark's
   anchor**. *Lowering* the mark anchor moves the drawn accent *up* on every base
   (the mark is placed so anchor meets anchor); raising it moves the accent
   down. Change **X** the same way to shift it sideways.
6. **Change the accent's size:** in the mark glyph, **Edit → Select All**, then
   **Element → Transformations → Transform…**, choose **Scale Uniformly**, enter
   e.g. `90` % → OK. Afterwards re-open the anchor (step 4) and confirm it still
   sits where the accent should attach (usually the middle of the stroke's
   bottom for top marks); retype X/Y if the transform moved it. Keep the glyph's
   **advance width at 0** so the mark stays non-spacing.
7. **Tune individual letters:** open a base glyph (e.g. `0C95` ಕ, or a matra
   such as `0CC6` ೆ that the mark stacks on). Its anchor is of type **Base
   Glyph** in the same class. Raise its **Y** to lift the accent on that glyph
   only. For marks that stack on other marks use a **Mark to Mark Attachment**
   lookup (`mkmk`).
8. **If the font has no mark positioning for these marks:** in Font Info →
   Lookups → GPOS, **Add Lookup** (type *Mark to Base Attachment*, feature `abvm`
   for above / `blwm` for below, script `knd2`+`knda`, language default), add a
   subtable, **Add Anchor Class** (`Above`, `Below`), then add a *Mark* anchor to
   each mark glyph and a *Base* anchor to every base glyph you need. This is a
   lot of work for a full Kannada set, so switching to a font that already
   includes the marks is usually quicker.
9. **Preview.** **Window → New Metrics Window**, type or paste a test string
   (see the table at the top), and use the script/feature selector at the top of
   that window to enable the Kannada features. Then **rename the font** (Font
   Info → *PS Names* and *TTF Names*), and **File → Generate Fonts…** as
   OpenType/TrueType with the **OpenType** option ticked so GPOS is written.
10. **Install and verify in a real renderer.** Install the font, restart the
    application (Word and browsers cache fonts), and look at the test strings.
    From a terminal, HarfBuzz's `hb-view YourFont.ttf "ಕಂ॑" -o out.png` shows what
    a shaping engine does with the sequence the engine produced.

**Symptom → fix**

| Symptom | Likely cause / fix |
|---|---|
| All accents too high or too low | Mark-glyph anchor **Y** (step 5) |
| Accent off-centre on a letter | Mark anchor **X**, or that base glyph's anchor **X** |
| Accent lands on the anusvara dot instead of above the letter | Base anchor of the anusvara glyph, or font treats ಂ as a base; adjust anchors (step 7) |
| Accent drawn as a separate spaced glyph or on a dotted circle ◌ | Advance width ≠ 0, wrong glyph class, or lookup not registered for the Kannada script (steps 2, 6, 8) |
| Accent too heavy/large | Scale the mark outline (step 6) |
| Udatta and Svarita at different heights | They have separate anchors; give both the same convention |

---

## 3. Input-method note: where accents go

The input engine always writes **base → matra → anusvara → Vedic mark**, and puts
stacked marks in Unicode canonical order (Anudatta U+0952 before Udatta/Svarita
U+0951 / U+1CDA). The result is stable under Unicode normalisation, so copy/paste,
search and font shaping all see the same sequence. See the comment at the top of
`src/engine.rs` for why this order keeps anusvara inline and accents above.
