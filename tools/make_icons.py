#!/usr/bin/env python3
"""Generate tray icons (raw RGBA) and the Windows .ico.

    python3 tools/make_icons.py [--font /path/to/NotoSansKannada-Bold.ttf]

ON  icon: saffron badge with the Kannada letter ಕ.
OFF icon: gray badge with "EN".

If no Kannada-capable font is found the ON badge falls back to a Latin "K" so
the build never breaks; pass --font (or set KANNADA_FONT) to get the real ಕ.
Requires Pillow.
"""
import argparse, glob, os, subprocess, sys
from PIL import Image, ImageDraw, ImageFont

SIZE = 64
OUT = os.path.join(os.path.dirname(__file__), "..", "assets", "icons")
CANDIDATES = [
    os.environ.get("KANNADA_FONT", ""),
    r"C:\Windows\Fonts\Nirmala.ttf", r"C:\Windows\Fonts\Nirmala.ttc",
    r"C:\Windows\Fonts\tunga.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansKannada-Bold.ttf",
    "/usr/share/fonts/noto/NotoSansKannada-Bold.ttf",
    "/Library/Fonts/Kannada Sangam MN.ttc",
]


def find_kannada_font():
    for p in CANDIDATES:
        if p and os.path.exists(p):
            return p
    try:  # fontconfig
        p = subprocess.run(["fc-match", "-f", "%{file}", ":lang=kn"],
                           capture_output=True, text=True).stdout.strip()
        if p and "kannada" in p.lower() or "noto" in p.lower():
            return p
    except Exception:
        pass
    for p in glob.glob("/usr/share/fonts/**/*Kannada*", recursive=True):
        return p
    return None


def badge(fill, text, font, scale=4):
    s = SIZE * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2 * scale, 2 * scale, s - 2 * scale, s - 2 * scale],
                        radius=14 * scale, fill=fill)
    f = font(int(s * (0.74 if len(text) == 1 else 0.40)))
    sw = int(scale * 0.9) if len(text) == 1 else 0  # thicken ಕ so it stays readable at 16 px
    box = d.textbbox((0, 0), text, font=f, stroke_width=sw)
    w, h = box[2] - box[0], box[3] - box[1]
    d.text(((s - w) / 2 - box[0], (s - h) / 2 - box[1]), text, font=f, fill="white",
           stroke_width=sw, stroke_fill="white")
    return img.resize((SIZE, SIZE), Image.LANCZOS)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--font")
    a = ap.parse_args()
    kn_path = a.font or find_kannada_font()
    latin = lambda px: ImageFont.load_default(size=px)
    if kn_path:
        kn = lambda px: ImageFont.truetype(kn_path, px)
        on_text = "\u0c95"  # ಕ
        print("Kannada font:", kn_path)
    else:
        kn, on_text = latin, "K"
        print("WARNING: no Kannada font found; ON icon uses a Latin 'K' placeholder.",
              file=sys.stderr)

    on = badge((217, 119, 6, 255), on_text, kn)      # saffron
    off = badge((107, 114, 128, 255), "EN", latin)   # gray

    os.makedirs(OUT, exist_ok=True)
    for name, im in (("on", on), ("off", off)):
        open(os.path.join(OUT, f"{name}_64.rgba"), "wb").write(im.tobytes())
        im.save(os.path.join(OUT, f"{name}_64.png"))
    big = badge((217, 119, 6, 255), on_text, kn, scale=8).resize((256, 256), Image.LANCZOS)
    big.save(os.path.join(OUT, "kannada_ime.ico"),
             sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print("wrote icons to", os.path.abspath(OUT))


if __name__ == "__main__":
    main()
