#!/usr/bin/env python3
"""Build docs/index.html (the web typer) from tools/web_typer_template.html and fonts/Arkavathi.ttf.

    python3 tools/build_web.py        (on Windows: python tools/build_web.py)

The template has two placeholders:
  __FONT__     the font, embedded as base64, so the page is one self-contained
               file with no outside requests;
  __VERSION__  the version, read from [package] in Cargo.toml, so the page always
               shows the same version as the app.
The output always uses LF line endings, so building on Windows and on Linux gives
the same file (CI rebuilds it and checks that the committed copy is up to date).
"""
import base64
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
TEMPLATE = ROOT / "tools" / "web_typer_template.html"
FONT = ROOT / "fonts" / "Arkavathi.ttf"
CARGO = ROOT / "Cargo.toml"
OUT = ROOT / "docs" / "index.html"


def fail(msg):
    sys.exit("build_web.py: " + msg)


def read_text(path):
    try:
        return path.read_text(encoding="utf-8")
    except OSError as e:
        fail(f"cannot read {path.relative_to(ROOT)} ({e})")


def package_version():
    """The `version = "..."` line inside the [package] section of Cargo.toml."""
    section = None
    for line in read_text(CARGO).splitlines():
        line = line.strip()
        if line.startswith("["):
            section = line
        elif section == "[package]":
            m = re.match(r'version\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    fail('no version = "..." line found under [package] in Cargo.toml')


def replace_once(text, placeholder, value):
    n = text.count(placeholder)
    if n != 1:
        fail(f"the template must contain {placeholder} exactly once (found {n})")
    return text.replace(placeholder, value)


def main():
    template = read_text(TEMPLATE)
    try:
        font = base64.b64encode(FONT.read_bytes()).decode("ascii")
    except OSError as e:
        fail(f"cannot read {FONT.relative_to(ROOT)} ({e})")
    version = package_version()

    page = replace_once(template, "__VERSION__", version)
    page = replace_once(page, "__FONT__", font)
    if "</style>" not in page:
        fail("the template has no </style> tag")
    head, rest = page.split("</style>", 1)

    html = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="description" content="Type phonetic Kannada with Vedic accents in your browser. Free, no sign-up, nothing leaves your device.">
<meta name="google-site-verification" content="fs5Xmeqc9OLu7mvtjh9ZfHWgdb7I1NBivu6H_I5oF-w" />
""" + head + "</style>\n</head>\n<body>" + rest + "\n</body>\n</html>\n"

    html = html.replace("\r\n", "\n")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write(html)
    print(f"wrote docs/index.html ({len(html)} bytes, version {version})")


if __name__ == "__main__":
    main()
