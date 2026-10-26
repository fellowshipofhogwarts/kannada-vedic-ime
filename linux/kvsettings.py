"""kvsettings.py - the settings file shared with the Windows agent.

~/.config/kannada-ime/settings.txt  (or $XDG_CONFIG_HOME/kannada-ime/settings.txt)
Lines look like `vedic=1`. Unknown keys are kept when saving. Never raises.
"""
import os


def _path():
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.join(os.path.expanduser("~"), ".config")
    return os.path.join(base, "kannada-ime", "settings.txt")


def _on(value):
    return value.strip().lower() in ("1", "true", "on", "yes")


def load_bool(key, default=True):
    try:
        with open(_path(), encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    k, v = line.split("=", 1)
                    if k.strip() == key:
                        return _on(v)
    except OSError:
        pass
    return default


def save_bool(key, on):
    p = _path()
    try:
        with open(p, encoding="utf-8") as f:
            lines = [l.rstrip("\n") for l in f]
    except OSError:
        lines = ["# Kannada IME settings (1 = on, 0 = off)"]
    out, done = [], False
    for l in lines:
        if l.replace(" ", "").startswith(key + "="):
            out.append("%s=%d" % (key, 1 if on else 0))
            done = True
        else:
            out.append(l)
    if not done:
        out.append("%s=%d" % (key, 1 if on else 0))
    try:
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w", encoding="utf-8") as f:
            f.write("\n".join(out) + "\n")
    except OSError:
        pass


def load_vedic(default=True):
    return load_bool("vedic", default)


def save_vedic(on):
    save_bool("vedic", on)


def load_digits(default=True):
    return load_bool("kannada_digits", default)


def save_digits(on):
    save_bool("kannada_digits", on)
