#!/usr/bin/env bash
# Install the Kannada Vedic input method for IBus (Linux Mint / Ubuntu, X11).
# Installs for your user only. It asks for your password only if IBus cannot
# see the user-level file (then it offers one `sudo` copy).
set -u

SRC="$(cd "$(dirname "$0")" && pwd)"
# Files that live next to this script in a release zip, or one folder up in the repository.
find_file() {
  for d in "$SRC" "$SRC/.." "$SRC/../fonts"; do
    [ -f "$d/$1" ] && { printf '%s' "$d/$1"; return 0; }
  done
  return 1
}
DEST="${KV_DEST:-$HOME/.local/share/kannada-vedic-ibus}"
COMP_DIR="${KV_COMP_DIR:-$HOME/.local/share/ibus/component}"
FONT_DIR="${KV_FONT_DIR:-$HOME/.local/share/fonts}"
XML_NAME="kannada-vedic.xml"

say() { printf '\n%s\n' "$*"; }

say "== Kannada Vedic input method for IBus =="

# 1. IBus and its Python bindings must be there.
if [ -z "${KV_SKIP_GI_CHECK:-}" ]; then
  if ! python3 -c 'import gi; gi.require_version("IBus","1.0"); from gi.repository import IBus' 2>/dev/null; then
    say "The Python bindings for IBus are missing. Install them first:"
    echo "    sudo apt install ibus python3-gi gir1.2-ibus-1.0"
    echo "Then run this script again."
    exit 1
  fi
fi

# 2. Copy the program.
mkdir -p "$DEST" || { echo "Cannot create $DEST"; exit 1; }
for f in ibus_engine.py keyhandler.py kvengine.py kvtables.py kvsettings.py Kannada-Vedic-Writer.ott; do
  cp "$SRC/$f" "$DEST/$f"
done
for f in OFL.txt LICENSE-MIT LICENSE-APACHE; do
  p="$(find_file "$f")" && cp "$p" "$DEST/$f"
done
rm -f "$DEST/Kaveri.ttf"   # left over from the first test builds
chmod +x "$DEST/ibus_engine.py"
echo "Program copied to $DEST"

# 3. The font (needed to SEE the Vedic marks and the single sign ꣳ).
#    It is called "Arkavathi". Early test builds installed it under the old name "Kaveri",
#    which clashes with another font of that name, so remove that copy first.
if [ -f "$FONT_DIR/Kaveri.ttf" ]; then rm -f "$FONT_DIR/Kaveri.ttf"; echo "Removed the old test font Kaveri.ttf"; fi
FONT_SRC="$(find_file Arkavathi.ttf)" || { echo "Arkavathi.ttf not found next to this script."; exit 1; }
mkdir -p "$FONT_DIR" && cp "$FONT_SRC" "$FONT_DIR/Arkavathi.ttf" \
  && { command -v fc-cache >/dev/null && fc-cache -f "$FONT_DIR" >/dev/null 2>&1; echo "Font 'Arkavathi' installed for your user."; }

# 3b. A LibreOffice Writer template whose default text style uses Arkavathi for
#     Latin, Asian AND complex (Kannada) text. Writer needs all three.
TPL_DIR="${KV_TPL_DIR:-$HOME/.config/libreoffice/4/user/template}"
mkdir -p "$TPL_DIR" && cp "$SRC/Kannada-Vedic-Writer.ott" "$TPL_DIR/" && echo "Writer template installed: File > New > Templates > My Templates > Kannada-Vedic-Writer"

# 4. The component file that tells IBus about the engine.
mkdir -p "$COMP_DIR"
XML="$COMP_DIR/$XML_NAME"
cat > "$XML" <<XMLEOF
<?xml version="1.0" encoding="utf-8"?>
<component>
  <name>org.freedesktop.IBus.KannadaVedic</name>
  <description>Kannada Vedic phonetic input</description>
  <exec>/usr/bin/python3 $DEST/ibus_engine.py --ibus</exec>
  <version>0.5.0</version>
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
XMLEOF
echo "IBus component file written: $XML"

# 5. Restart IBus and check that it sees the engine.
restart_and_check() {
  if command -v ibus >/dev/null; then
    ibus restart >/dev/null 2>&1
    sleep 3
    ibus list-engine 2>/dev/null | grep -q "kannada-vedic"
  else
    return 1
  fi
}

if restart_and_check; then
  say "IBus can see the engine."
else
  say "IBus does not list the engine yet."
  echo "Some systems only read engine files from /usr/share/ibus/component."
  printf 'Copy the file there with sudo (asks for your password once)? [y/N] '
  read -r ans
  case "$ans" in
    y|Y|yes|YES)
      if sudo install -m 644 "$XML" "/usr/share/ibus/component/$XML_NAME"; then
        if restart_and_check; then say "IBus can see the engine."; else
          say "Still not listed. Please send me the output of:  ibus list-engine | grep -i kannada ; ls ~/.local/share/ibus/component /usr/share/ibus/component"; fi
      fi ;;
    *) echo "Skipped. Tell me and we will look at it together." ;;
  esac
fi

cat <<'NEXT'

== Next steps ==
 1. Run:  ibus-setup
    Tab "Input Method" -> "Add" -> choose "Kannada" -> "Kannada Vedic" -> Add.
    (If you do not see Kannada, log out and log in once, then try again.)
 2. Open LibreOffice Writer with the template:  File > New > Templates > My Templates >
    Kannada-Vedic-Writer  (or double-click  Kannada-Vedic-Writer.ott  in this folder).
 3. Switch the input method with Super+Space (or the IBus icon in the panel).
 4. Type:  namaste   then press Space.   You should see ನಮಸ್ತೆ
    While you type, the word is underlined; Space or Enter finishes it.

Log file (send it to me if something looks wrong):  ~/.cache/kannada-ime/ibus.log
To remove everything later:  ./uninstall.sh
NEXT
