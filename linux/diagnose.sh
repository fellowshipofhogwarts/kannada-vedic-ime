#!/usr/bin/env bash
# Collects the facts I need when something does not work. Changes nothing.
# Run:  ./diagnose.sh   and paste everything it prints.
echo "== session";  echo "$XDG_SESSION_TYPE"; lsb_release -ds 2>/dev/null; uname -m
echo "== input method variables"; echo "GTK=$GTK_IM_MODULE QT=$QT_IM_MODULE XMOD=$XMODIFIERS"
echo "== ibus"; ibus version 2>&1; pgrep -a ibus-daemon
echo "== engine listed?"; ibus list-engine 2>/dev/null | grep -i -B1 kannada || echo "NOT LISTED"
echo "== files"; ls -l ~/.local/share/ibus/component/ /usr/share/ibus/component/kannada-vedic.xml 2>&1 | grep -i -E "kannada|No such" 
ls ~/.local/share/kannada-vedic-ibus 2>&1 | tr '\n' ' '; echo
echo "== engine process"; pgrep -af ibus_engine || echo "engine not running (it starts when you first select it)"
echo "== settings"; cat ~/.config/kannada-ime/settings.txt 2>&1
echo "== fonts named Kaveri or Arkavathi (there should be exactly ONE Arkavathi and no Kaveri)"
fc-list | grep -i -E "kaveri|arkavathi" || echo "none found"
echo "fontconfig picks for Arkavathi:"; fc-match "Arkavathi" family file
echo "== LibreOffice"; (libreoffice --version 2>/dev/null || soffice --version 2>/dev/null) | head -1
echo "== python / gi"; python3 --version; python3 -c 'import gi; gi.require_version("IBus","1.0"); from gi.repository import IBus; print("IBus python ok")' 2>&1 | tail -1
echo "== last log lines"; tail -n 25 ~/.cache/kannada-ime/ibus.log 2>&1
