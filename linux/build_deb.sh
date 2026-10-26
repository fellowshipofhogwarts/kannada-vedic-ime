#!/usr/bin/env bash
# Build kannada-vedic-ibus_<version>_all.deb  (needs: dpkg-deb). Run from the repository:
#     linux/build_deb.sh            -> dist/kannada-vedic-ibus_0.5.0_all.deb
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
VERSION="${1:-0.5.0}"
OUT="${OUT_DIR:-$ROOT/dist}"
PKG="$(mktemp -d)"
chmod 755 "$PKG"
trap 'rm -rf "$PKG"' EXIT
APP=usr/share/kannada-vedic-ibus

install -d "$PKG/DEBIAN" "$PKG/$APP" "$PKG/usr/share/ibus/component" \
  "$PKG/usr/share/fonts/truetype/arkavathi" "$PKG/usr/share/doc/kannada-vedic-ibus" "$PKG/usr/bin"

for f in ibus_engine.py keyhandler.py kvengine.py kvtables.py kvsettings.py; do
  install -m 644 "$HERE/$f" "$PKG/$APP/$f"
done
chmod 755 "$PKG/$APP/ibus_engine.py"
install -m 644 "$HERE/Kannada-Vedic-Writer.ott" "$PKG/$APP/"
install -m 644 "$ROOT/fonts/Arkavathi.ttf" "$PKG/usr/share/fonts/truetype/arkavathi/Arkavathi.ttf"
install -m 644 "$ROOT/fonts/OFL.txt" "$PKG/$APP/OFL.txt"
install -m 644 "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$PKG/usr/share/doc/kannada-vedic-ibus/"
install -m 644 "$HERE/README-LINUX.md" "$PKG/usr/share/doc/kannada-vedic-ibus/README.md"
cat > "$PKG/usr/share/doc/kannada-vedic-ibus/copyright" <<C
Program: MIT OR Apache-2.0 (see LICENSE-MIT and LICENSE-APACHE in this folder).
Font Arkavathi: SIL Open Font License 1.1 (see /$APP/OFL.txt), derived from Goda
(Reserved Font Name), (c) Parankusacar Institute of Vedic Studies and Indian Institute of Science.
C

cat > "$PKG/usr/share/ibus/component/kannada-vedic.xml" <<X
<?xml version="1.0" encoding="utf-8"?>
<component>
  <name>org.freedesktop.IBus.KannadaVedic</name>
  <description>Kannada Vedic phonetic input</description>
  <exec>/usr/bin/python3 /$APP/ibus_engine.py --ibus</exec>
  <version>$VERSION</version>
  <author>Kannada Vedic IME</author>
  <license>MIT OR Apache-2.0</license>
  <homepage>https://github.com/fellowshipofhogwarts/kannada-vedic-ime</homepage>
  <textdomain>kannada-vedic-ibus</textdomain>
  <engines>
    <engine>
      <name>kannada-vedic</name>
      <language>kn</language>
      <license>MIT OR Apache-2.0</license>
      <author>Kannada Vedic IME</author>
      <icon>input-keyboard</icon>
      <layout>default</layout>
      <longname>Kannada Vedic</longname>
      <description>Phonetic Kannada typing with Vedic accents</description>
      <rank>0</rank>
    </engine>
  </engines>
</component>
X

cat > "$PKG/usr/bin/kannada-vedic-writer-template" <<'T'
#!/bin/sh
# Copy the LibreOffice Writer template (Arkavathi for Latin, Asian and Kannada text) into your user templates.
d="${XDG_CONFIG_HOME:-$HOME/.config}/libreoffice/4/user/template"
mkdir -p "$d" && cp /usr/share/kannada-vedic-ibus/Kannada-Vedic-Writer.ott "$d/" \
  && echo "Done. In Writer: File > New > Templates > My Templates > Kannada-Vedic-Writer"
T
chmod 755 "$PKG/usr/bin/kannada-vedic-writer-template"

cat > "$PKG/DEBIAN/control" <<C
Package: kannada-vedic-ibus
Version: $VERSION
Section: utils
Priority: optional
Architecture: all
Depends: python3, python3-gi, gir1.2-ibus-1.0, ibus, fontconfig
Maintainer: Shashank Koundinya Banavathi Rajashekara <noreply@users.noreply.github.com>
Homepage: https://github.com/fellowshipofhogwarts/kannada-vedic-ime
Description: Phonetic Kannada input method with Vedic accents (IBus)
 Type Kannada with English letters (Baraha style), including the Vedic accent
 marks. Installs the IBus engine "Kannada Vedic", the Arkavathi font and a
 LibreOffice Writer template. Experimental on Linux.
C
cat > "$PKG/DEBIAN/postinst" <<'P'
#!/bin/sh
set -e
command -v fc-cache >/dev/null 2>&1 && fc-cache -f /usr/share/fonts/truetype/arkavathi || true
command -v ibus >/dev/null 2>&1 && ibus write-cache >/dev/null 2>&1 || true
echo "Kannada Vedic: run 'ibus restart' (or log out and in), then add it in ibus-setup > Input Method > Kannada."
exit 0
P
cat > "$PKG/DEBIAN/postrm" <<'P'
#!/bin/sh
set -e
command -v fc-cache >/dev/null 2>&1 && fc-cache -f || true
command -v ibus >/dev/null 2>&1 && ibus write-cache >/dev/null 2>&1 || true
exit 0
P
chmod 755 "$PKG/DEBIAN/postinst" "$PKG/DEBIAN/postrm"

mkdir -p "$OUT"
dpkg-deb --root-owner-group --build "$PKG" "$OUT/kannada-vedic-ibus_${VERSION}_all.deb" >/dev/null
echo "Built $OUT/kannada-vedic-ibus_${VERSION}_all.deb"
