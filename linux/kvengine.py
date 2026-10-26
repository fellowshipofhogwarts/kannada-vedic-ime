"""kvengine.py - the phonetic Kannada engine, ported from src/engine.rs.

Pure logic, no IBus and no GTK, so it can be tested anywhere. The tables come
from kvtables.py, which is generated from the Rust source (tools/gen_tables.py).

Windows types straight into the application and edits it with backspaces. IBus
works differently: the word being typed is shown as *preedit* (underlined) and is
committed when it ends. So this module only needs `render_with` and the small
state machine in `Core`; there is no diff and no backspace injection.
"""
from kvtables import (VOWELS, CONSONANTS, SIGNS, ACCENTS, VIRAMA, ANUSVARA,
                      VISARGA, AGAMA_ANUSVARA, BREAK_KEY, OTHERS, NUKTA)

MAX_TOKEN_LEN = 3
MAX_BUFFER = 48

UDATTA, ANUDATTA, DOUBLE_SVARITA = "॑", "॒", "᳚"

# token kinds: ("v", independent, matra) ("c", cluster) ("s", sign) ("o", char)
#              ("n",) nukta  ("a", mark)  ("b",) break
# Linux preview choice: g`M gives the plain ಗ್ಂ (the Windows agent gives the single
# sign ꣳ). Set to True to get the Windows behaviour (the parity test does).
AGAMA_ENABLED = False

# Kannada digits U+0CE6..U+0CEF
def kannada_digit(c):
    return chr(0x0CE6 + int(c)) if len(c) == 1 and "0" <= c <= "9" else None

TABLE = {}


def _put(key, tok):
    assert len(key) <= MAX_TOKEN_LEN, key
    assert key not in TABLE, "duplicate roman token %r" % key
    TABLE[key] = tok


for _keys, _ind, _matra in VOWELS:
    for _k in _keys:
        _put(_k, ("v", _ind, _matra))
for _keys, _cl in CONSONANTS:
    for _k in _keys:
        _put(_k, ("c", _cl))
for _keys, _s in SIGNS:
    for _k in _keys:
        _put(_k, ("s", _s))
for _keys, _ch in OTHERS:
    for _k in _keys:
        _put(_k, ("o", _ch))
_put("Q", ("n",))
for _keys, _mark in ACCENTS:
    for _k in _keys:
        _put(_k, ("a", _mark))
_put(BREAK_KEY, ("b",))

_ACCENT_FIRST = {k[0] for keys, _ in ACCENTS for k in keys}
PHONETIC_FIRST = {k[0] for k in TABLE} - _ACCENT_FIRST
ACCENT_SINGLE = {k for keys, _ in ACCENTS for k in keys if len(k) == 1}
MARKS = {m for _, m in ACCENTS}


def combining_class(c):
    if c in (UDATTA, DOUBLE_SVARITA):
        return 230
    if c == ANUDATTA:
        return 220
    return 0


def is_phonetic_key(c):
    return len(c) == 1 and ord(c) < 128 and c in PHONETIC_FIRST


def is_accent_key(c):
    return c in ACCENT_SINGLE


def _trailing_marks_start(s):
    i = len(s)
    while i > 0 and s[i - 1] in MARKS:
        i -= 1
    return i


def _sort_trailing_marks(s):
    i = _trailing_marks_start(s)
    marks = sorted(s[i:], key=combining_class)  # stable, like Rust's sort_by_key
    return s[:i] + "".join(marks)


def render_with(roman, vedic=True, agama=None):
    """Return (text, attachable). Same rules as render_with in engine.rs."""
    if agama is None:
        agama = AGAMA_ENABLED
    out = ""
    pending = False
    attachable = False
    after_break = False
    i = 0
    n = len(roman)
    while i < n:
        hit = None
        for ln in range(MAX_TOKEN_LEN, 0, -1):
            end = i + ln
            if end <= n:
                t = TABLE.get(roman[i:end])
                if t is not None:
                    if not vedic and t[0] == "a":
                        continue
                    hit = (t, ln)
                    break
        if hit is None:
            out += roman[i]
            pending = attachable = after_break = False
            i += 1
            continue
        tok, ln = hit
        kind = tok[0]
        if kind == "c":
            out += tok[1] + VIRAMA
            pending = True
            attachable = True
            after_break = False
        elif kind == "v":
            if pending and not after_break:
                out = out[:-1]
                if tok[2] is not None:
                    out += tok[2]
            else:
                out += tok[1]
            pending = False
            attachable = True
            after_break = False
        elif kind == "s":
            s = tok[1]
            if pending and after_break:
                if vedic and agama and s == ANUSVARA and out.endswith("ಗ" + VIRAMA):
                    out = out[:-2] + AGAMA_ANUSVARA
                else:
                    out += s
            elif pending:
                out = out[:-1] + s
            else:
                at = _trailing_marks_start(out)
                out = out[:at] + s + out[at:]
            pending = False
            attachable = True
            after_break = False
        elif kind == "o":
            out += tok[1]
            pending = attachable = after_break = False
        elif kind == "n":
            if pending and not out[:-1].endswith(NUKTA):
                out = out[:-1] + NUKTA + VIRAMA
            else:
                out += "Q"
                pending = attachable = False
            after_break = False
        elif kind == "a":
            if attachable:
                if pending:
                    out = out[:-1]
                    pending = False
                out += tok[1]
                out = _sort_trailing_marks(out)
            else:
                out += roman[i:i + ln]
                pending = False
            after_break = False
        else:  # break
            after_break = True
        i += ln
    return out, attachable


class Core:
    """The word buffer. Feed it characters; read `text` for the preedit.

    on_char returns "consumed" (the key is ours; show `text` as preedit) or
    "pass" (not ours: commit `text`, then let the key through; the buffer has
    been cleared). That is the same decision tree as Engine::on_char in Rust.
    """

    def __init__(self, vedic=True):
        self.buf = ""
        self.vedic = vedic
        self.text = ""

    def reset(self):
        self.buf = ""
        self.text = ""

    def set_vedic(self, on):
        if self.vedic != on:
            self.vedic = on
            self.reset()

    def _sync(self):
        self.text = render_with(self.buf, self.vedic)[0]

    def on_char(self, c):
        if c == "`" and self.buf.endswith("`"):
            return "consumed"  # a repeated break key changes nothing
        if is_phonetic_key(c):
            if len(self.buf) >= MAX_BUFFER:
                self.reset()
            self.buf += c
            self._sync()
            return "consumed"
        if self.vedic and is_accent_key(c) and render_with(self.buf, True)[1]:
            if len(self.buf) >= MAX_BUFFER:
                self.reset()
                return "pass"
            self.buf += c
            self._sync()
            return "consumed"
        return "pass"

    def on_backspace(self):
        if not self.buf:
            return "pass"
        self.buf = self.buf[:-1]
        self._sync()
        return "consumed"
