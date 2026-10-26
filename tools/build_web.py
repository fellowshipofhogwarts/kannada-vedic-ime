#!/usr/bin/env python3
"""Build docs/index.html (the web typer) from tools/web_typer_template.html and fonts/Arkavathi.ttf.

    python3 tools/build_web.py

The template has the placeholder __FONT__ where the font is embedded as base64, so the
page is one self-contained file with no outside requests.
"""
import base64, os
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
t = open(os.path.join(root, "tools/web_typer_template.html"), encoding="utf-8").read()
font = base64.b64encode(open(os.path.join(root, "fonts/Arkavathi.ttf"), "rb").read()).decode()
head, rest = t.replace("__FONT__", font).split("</style>", 1)
html = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="description" content="Type phonetic Kannada with Vedic accents in your browser. Free, no sign-up, nothing leaves your device.">
<meta name="google-site-verification" content="fs5Xmeqc9OLu7mvtjh9ZfHWgdb7I1NBivu6H_I5oF-w" />
""" + head + "</style>\n</head>\n<body>" + rest + "\n</body>\n</html>\n"
open(os.path.join(root, "docs/index.html"), "w", encoding="utf-8").write(html)
print("wrote docs/index.html", len(html), "bytes")
