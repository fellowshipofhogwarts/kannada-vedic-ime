#!/usr/bin/env python3
"""Print kvengine.render_with output in the same format as the Rust `dump` binary
(one line of hex code points per input line; first char = Vedic mode 1/0)."""
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from kvengine import render_with
for line in sys.stdin:
    line = line.rstrip("\n")
    mode, s = line[0], line[1:]
    out = render_with(s, mode == "1", agama=True)[0]
    print(" ".join("%x" % ord(c) for c in out))
