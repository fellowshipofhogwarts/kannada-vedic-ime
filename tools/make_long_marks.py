#!/usr/bin/env python3
"""Make a tuned copy of the Goda font for Vedic reading. Needs: pip install fonttools

    python3 make_long_marks.py Goda.ttf Arkavathi.ttf [bar_factor] [anudatta_raise]

1. Taller Udatta / Svarita bars. The vertical bars (single and double) are scaled
   by `bar_factor` (default 1.4). Their bottom edge stays put, so only the top grows.

2. Anudatta (the bar below the letter) is raised by `anudatta_raise` font units
   (default 0 = off; not yet validated) *only when it follows a shallow letter*.
   Goda drops Anudatta a fixed 900 units under every letter so that it clears the
   deepest subscript (ottakshara) forms. That leaves it far from plain letters.
   Subscripts are separate marks in the font, and a Vedic mark cannot tell whether
   one is present, so the font itself cannot move it "just below the cluster".
   Instead we add a GPOS contextual rule: if the glyph right before Anudatta is
   a Kannada letter whose outline does not hang below the baseline (optionally
   followed by a vowel sign / anusvara / visarga that does not hang either),
   apply the raise. After letters with subscripts or tails (ಘ ಛ ಝ ಢ ಥ ...) nothing
   changes, so the original clearance is kept.

Goda is under the SIL Open Font License with Reserved Font Name "Goda", so the
copy is renamed and the original copyright/licence records are kept. Keep it
under the same licence if you share it.
"""
import sys

from fontTools.pens.boundsPen import BoundsPen
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables import otTables as ot

BAR_GLYPHS = ("dn_svarita", "dn_dSvarita", "kn_svarita", "kn_dSvarita")
ANUDATTA = ("dn_anudatta", "kn_anudatta")
VERSION = "1.3"
FAMILY = "Arkavathi"
PS_NAME = "Arkavathi-Regular"
SHALLOW = -150  # a glyph whose lowest point is above this does not "hang"


def lengthen_bars(f, factor):
    glyf = f["glyf"]
    for name in BAR_GLYPHS:
        g = glyf[name]
        assert not g.isComposite(), name
        pivot = min(y for _, y in g.coordinates)  # keep the bottom edge fixed
        g.coordinates = type(g.coordinates)(
            [(x, round(pivot + (y - pivot) * factor)) for x, y in g.coordinates]
        )
        g.recalcBounds(glyf)


def _coverage(f, glyphs):
    c = ot.Coverage()
    c.glyphs = sorted(set(glyphs), key=f.getGlyphID)
    return c


def raise_anudatta(f, units):
    gs = f.getGlyphSet()
    cls = f["GDEF"].table.GlyphClassDef.classDefs
    order = f.getGlyphOrder()

    def lowest(g):
        bp = BoundsPen(gs)
        gs[g].draw(bp)
        return bp.bounds[1] if bp.bounds else 0

    letters = [g for g in order if g.startswith("kn_") and cls.get(g) == 1 and lowest(g) >= SHALLOW]
    # vowel signs / anusvara / visarga that sit beside or on the letter, not below it
    signs = [
        g
        for g in order
        if g.startswith("kn_")
        and cls.get(g) == 3
        and (g.startswith("kn_c_") or g in ("kn_anusvara", "kn_visarga"))
        and lowest(g) >= SHALLOW
    ]

    gpos = f["GPOS"].table
    lookups = gpos.LookupList.Lookup

    # 1) single adjustment: move Anudatta up by `units`
    sp = ot.SinglePos()
    sp.Format = 1
    sp.Coverage = _coverage(f, ANUDATTA)
    sp.ValueFormat = 0x0002  # YPlacement
    sp.Value = ot.ValueRecord()
    sp.Value.YPlacement = units
    raise_lookup = ot.Lookup()
    raise_lookup.LookupType = 1
    raise_lookup.LookupFlag = 0
    raise_lookup.SubTable = [sp]
    raise_lookup.SubTableCount = 1
    lookups.append(raise_lookup)
    raise_idx = len(lookups) - 1

    # 2) chain rules (no mark skipping, so subscripts between letter and mark count)
    def rule(backtrack):  # backtrack[0] = glyph immediately before Anudatta
        cc = ot.ChainContextPos()
        cc.Format = 3
        cc.BacktrackGlyphCount = len(backtrack)
        cc.BacktrackCoverage = [_coverage(f, b) for b in backtrack]
        cc.InputGlyphCount = 1
        cc.InputCoverage = [_coverage(f, ANUDATTA)]
        cc.LookAheadGlyphCount = 0
        cc.LookAheadCoverage = []
        rec = ot.PosLookupRecord()
        rec.SequenceIndex = 0
        rec.LookupListIndex = raise_idx
        cc.PosCount = 1
        cc.PosLookupRecord = [rec]
        return cc

    chain = ot.Lookup()
    chain.LookupType = 8
    chain.LookupFlag = 0
    chain.SubTable = [rule([letters]), rule([signs, letters])]
    chain.SubTableCount = 2
    lookups.append(chain)
    chain_idx = len(lookups) - 1
    gpos.LookupList.LookupCount = len(lookups)

    # 3) switch the rule on wherever the 'dist' feature is used
    n = 0
    for fr in gpos.FeatureList.FeatureRecord:
        if fr.FeatureTag == "dist":
            fr.Feature.LookupListIndex.append(chain_idx)
            fr.Feature.LookupCount = len(fr.Feature.LookupListIndex)
            n += 1
    assert n, "no 'dist' feature found"
    return len(letters), len(signs)


def enable_deva_positioning(f):
    """Give the Devanagari script the same GPOS features as Kannada.

    U+A8F3 (agama anusvara) is a Devanagari-block character, so text shaping puts
    it, and any Vedic mark after it, into a separate 'deva' run. Goda's 'deva'
    script has no GPOS features, so the mark was never positioned (it showed up
    with a dotted circle, or floating at the wrong place). Pointing 'deva' at the
    same feature list as 'knda' makes the mark attach exactly as it does on letters.
    """
    gpos = f["GPOS"].table
    recs = {sr.ScriptTag: sr.Script.DefaultLangSys for sr in gpos.ScriptList.ScriptRecord}
    kn, dv = recs["knda"], recs["deva"]
    dv.FeatureIndex = list(kn.FeatureIndex)
    dv.FeatureCount = len(dv.FeatureIndex)


def rename(f, note):
    for rec in f["name"].names:
        if rec.nameID == 1:
            rec.string = FAMILY
        elif rec.nameID == 3:
            rec.string = "Arkavathi-%s;derived-from-Goda-1.0.5" % VERSION
        elif rec.nameID in (16,):
            rec.string = FAMILY
        elif rec.nameID == 4:
            rec.string = FAMILY
        elif rec.nameID == 5:
            rec.string = note
        elif rec.nameID == 6:
            rec.string = PS_NAME
        # IDs 0, 13, 14 (copyright + OFL licence) are intentionally unchanged.


def main(src, dst, factor=1.4, raise_units=0):
    f = TTFont(src)
    lengthen_bars(f, factor)
    enable_deva_positioning(f)
    nl, ns = (0, 0)
    if raise_units:
        nl, ns = raise_anudatta(f, raise_units)
    f["head"].fontRevision = float(VERSION)
    rename(f, "Version %s; bars x%.2f, Devanagari-script marks positioned, derived from Goda 1.0.5" % (VERSION, factor))
    f.save(dst)
    print("wrote", dst, "| rule covers", nl, "letters,", ns, "signs")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    a = sys.argv
    main(a[1], a[2], float(a[3]) if len(a) > 3 else 1.4, int(a[4]) if len(a) > 4 else 0)
