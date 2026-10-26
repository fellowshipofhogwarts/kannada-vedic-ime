#!/usr/bin/env bash
# Remove the Kannada Vedic input method (user-level files, and the system copy if you made one).
set -u
DEST="${KV_DEST:-$HOME/.local/share/kannada-vedic-ibus}"
COMP_DIR="${KV_COMP_DIR:-$HOME/.local/share/ibus/component}"
FONT_DIR="${KV_FONT_DIR:-$HOME/.local/share/fonts}"
rm -rf "$DEST" "$COMP_DIR/kannada-vedic.xml" "$FONT_DIR/Kaveri.ttf" "$FONT_DIR/Arkavathi.ttf" "${KV_TPL_DIR:-$HOME/.config/libreoffice/4/user/template}/Kannada-Vedic-Writer.ott"
command -v fc-cache >/dev/null && fc-cache -f "$FONT_DIR" >/dev/null 2>&1
if [ -f /usr/share/ibus/component/kannada-vedic.xml ]; then
  printf 'Also remove /usr/share/ibus/component/kannada-vedic.xml with sudo? [y/N] '
  read -r ans
  case "$ans" in y|Y|yes|YES) sudo rm -f /usr/share/ibus/component/kannada-vedic.xml ;; esac
fi
command -v ibus >/dev/null && ibus restart >/dev/null 2>&1
echo "Removed. If 'Kannada Vedic' is still in your input-method list, delete it in ibus-setup."
echo "Your settings file (~/.config/kannada-ime/settings.txt) and log were left in place."
