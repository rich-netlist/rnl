"""The fonts shipped with the core (decision 0007): Noto Sans SC Regular and Bold, the Simplified
Chinese subset of Noto Sans CJK, under the SIL Open Font License 1.1.

The fonts are not in the repository. `fetch` downloads them from the noto-cjk repository at a pinned
tag into target/fonts/, which Git ignores, and checks their SHA-256 sums. From them:

- `metrics` writes crates/rnl-render/fonts/noto-sans-sc.metrics, what the renderer lays out text
  with (crates/rnl-render/src/text.rs reads it). The file is committed; `metrics --check` fails
  when it is not what the pinned fonts give.
- `web` writes the web fonts of the viewers' page into apps/viewer-ui/fonts/, which Git ignores:
  WOFF2 files split by Unicode ranges, so that a browser downloads only the slices a drawing uses,
  fonts.css with their @font-face rules, and OFL.txt, the licence with the fonts' copyright.

The metrics file, little-endian:
    "RNLM", u16 version (1), u16 number of faces F (Regular, then Bold)
    per face: u16 units per em, i16 ascender, i16 descender, i16 line gap (hhea),
              i16 top of the em box (OS/2 sTypoAscender), u16 advance of .notdef
    u32 number of runs N
    N runs, by their first code point: u32 first code point, u16 number of code points,
              F × u16 advance in each face
A run is a range of code points that every face maps, each with the same advance in a face as the
others of the range. The faces map the same code points.

Usage, from the repository root (fetch needs only the standard library; metrics and web need
fontTools, and web needs brotli as well: pip install fonttools brotli):
    python scripts/fonts.py fetch             download the fonts, unless they are there already
    python scripts/fonts.py metrics           write the metrics file
    python scripts/fonts.py metrics --check   check that it is current, write nothing
    python scripts/fonts.py web [--out DIR]   write the web fonts, into DIR instead if given
metrics and web fetch the fonts first when they are missing.
"""
import argparse
import hashlib
import io
import shutil
import struct
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "target" / "fonts"
METRICS = ROOT / "crates" / "rnl-render" / "fonts" / "noto-sans-sc.metrics"
WEB = ROOT / "apps" / "viewer-ui" / "fonts"
LICENSE = ROOT / "LICENSES" / "OFL-1.1.txt"

FAMILY = "Noto Sans SC"
TAG = "Sans2.004"
URL = "https://github.com/notofonts/noto-cjk/raw/%s/Sans/SubsetOTF/SC/%s"
# (file, CSS font weight, SHA-256 of the file at TAG)
FACES = [
    ("NotoSansSC-Regular.otf", 400, "faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9"),
    ("NotoSansSC-Bold.otf", 700, "c6cb5a93abaa9edc8ee7463b7ebb7f42d618d40e6ed2f7a5371c97b0b64767c0"),
]
COPYRIGHT = "Copyright 2014-2021 Adobe (http://www.adobe.com/). Noto is a trademark of Google Inc."


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def fetch():
    """The paths of the fonts in the cache, downloaded first when missing or not as pinned."""
    CACHE.mkdir(parents=True, exist_ok=True)
    paths = []
    for name, _, want in FACES:
        path = CACHE / name
        if not path.exists() or sha256(path) != want:
            url = URL % (TAG, name)
            print("downloading %s" % url, file=sys.stderr)
            part = path.with_name(name + ".part")
            with urllib.request.urlopen(url, timeout=300) as r, open(part, "wb") as f:
                shutil.copyfileobj(r, f)
            got = sha256(part)
            if got != want:
                part.unlink()
                sys.exit("%s: SHA-256 %s, not the pinned %s" % (url, got, want))
            part.replace(path)
        paths.append(path)
    return paths


def load(paths):
    from fontTools.ttLib import TTFont

    return [TTFont(p, lazy=True) for p in paths]


def metrics_table(fonts):
    """The metrics file's bytes."""
    out = [b"RNLM", struct.pack("<HH", 1, len(fonts))]
    advances = []
    for font in fonts:
        hhea, os2, hmtx = font["hhea"], font["OS/2"], font["hmtx"].metrics
        out.append(struct.pack("<HhhhhH", font["head"].unitsPerEm, hhea.ascent, hhea.descent, hhea.lineGap,
                               os2.sTypoAscender, hmtx[".notdef"][0]))
        advances.append({cp: hmtx[glyph][0] for cp, glyph in font.getBestCmap().items()})
    if any(a.keys() != advances[0].keys() for a in advances):
        sys.exit("the faces do not map the same code points")
    runs = []
    for cp in sorted(advances[0]):
        adv = tuple(a[cp] for a in advances)
        if runs and runs[-1][0] + runs[-1][1] == cp and runs[-1][2] == adv and runs[-1][1] < 0xFFFF:
            runs[-1][1] += 1
        else:
            runs.append([cp, 1, adv])
    out.append(struct.pack("<I", len(runs)))
    for first, count, adv in runs:
        out.append(struct.pack("<IH%dH" % len(adv), first, count, *adv))
    return b"".join(out)


def metrics(check):
    table = metrics_table(load(fetch()))
    name = METRICS.relative_to(ROOT).as_posix()
    if check:
        if not METRICS.exists() or METRICS.read_bytes() != table:
            sys.exit("%s is not what the pinned fonts give; run python scripts/fonts.py metrics" % name)
        print("%s is current (%d bytes)" % (name, len(table)))
        return
    METRICS.parent.mkdir(parents=True, exist_ok=True)
    METRICS.write_bytes(table)
    print("%s: %d bytes" % (name, len(table)))


def gb2312(level):
    """The hanzi of GB 2312 level 1, the 3755 most common, or level 2, 3008 more, in its order."""
    out = []
    for row in range(16, 56) if level == 1 else range(56, 88):
        for cell in range(1, 95):
            try:
                out.append(ord(bytes([0xA0 + row, 0xA0 + cell]).decode("gb2312")))
            except UnicodeDecodeError:
                pass
    return out


def chunks(name, cps, size):
    return [("%s-%02d" % (name, k // size + 1), cps[k:k + size]) for k in range(0, len(cps), size)]


def slices(mapped):
    """The slices of the code points the fonts map, by name: Latin with the punctuation and symbols
    of the markup (13.6) first, then the other symbols, the CJK punctuation and full-width forms,
    the common hanzi in small slices and the less common in larger ones, and the rest in Unicode
    order. Each code point is in one slice."""
    left = set(mapped)

    def take(cps):
        got = [cp for cp in cps if cp in left]
        left.difference_update(got)
        return got

    def within(*ranges):
        return take(sorted(cp for cp in left if any(a <= cp <= b for a, b in ranges)))

    out = [("latin", within((0, 0x3FF), (0x1E00, 0x1EFF), (0x2000, 0x23FF), (0xFEFF, 0xFEFF), (0xFFFD, 0xFFFD)))]
    out.append(("symbols", within((0x400, 0x2E7F))))
    out.append(("cjk-punctuation", within((0x3000, 0x303F), (0xFE10, 0xFE1F), (0xFE30, 0xFE4F), (0xFF00, 0xFFEF))))
    out += chunks("hanzi-a", take(gb2312(1)), 50)
    out += chunks("hanzi-b", take(gb2312(2)), 250)
    out += chunks("cjk", sorted(left), 1000)
    return [(name, cps) for name, cps in out if cps]


def unicode_range(cps):
    """The code points as a CSS unicode-range, in runs."""
    runs = []
    for cp in sorted(cps):
        if runs and runs[-1][1] + 1 == cp:
            runs[-1][1] = cp
        else:
            runs.append([cp, cp])
    return ", ".join("U+%X" % a if a == b else "U+%X-%X" % (a, b) for a, b in runs)


def web(out):
    from fontTools import subset
    from fontTools.ttLib import TTFont

    paths = fetch()
    mapped = sorted(load(paths)[0].getBestCmap())
    parts = slices(mapped)
    if out.exists():
        for old in list(out.glob("*.woff2")) + [out / "fonts.css", out / "OFL.txt"]:
            old.unlink(missing_ok=True)
    out.mkdir(parents=True, exist_ok=True)
    css = [
        "/* %s, Regular and Bold, split by Unicode ranges: written by scripts/fonts.py from the\n"
        "   fonts of noto-cjk %s, whose metrics the core lays out text with (decision 0007).\n"
        "   %s\n   Licensed under the SIL Open Font License 1.1: see OFL.txt. */\n" % (FAMILY, TAG, COPYRIGHT)
    ]
    sizes = {}
    for (name, weight, _), path in zip(FACES, paths):
        for part, cps in parts:
            options = subset.Options()
            options.flavor = "woff2"
            # no kerning, ligatures or other substitutions: the core lays out by the advances
            options.layout_features = []
            options.drop_tables += ["vhea", "vmtx", "VORG"]
            # the names, the copyright, the trademark and the licence (OFL 2, 4)
            options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 7, 13, 14]
            # with the font's own head.modified rather than now, so that the files are the same
            # every time
            font = TTFont(path, recalcTimestamp=False)
            subsetter = subset.Subsetter(options)
            subsetter.populate(unicodes=cps)
            subsetter.subset(font)
            data = io.BytesIO()
            font.save(data)
            file = "noto-sans-sc-%d-%s.woff2" % (weight, part)
            (out / file).write_bytes(data.getvalue())
            sizes[file] = len(data.getvalue())
            css.append(
                '@font-face {\n  font-family: "%s";\n  font-style: normal;\n  font-weight: %d;\n'
                '  font-display: swap;\n  src: url("%s") format("woff2");\n  unicode-range: %s;\n}\n'
                % (FAMILY, weight, file, unicode_range(cps))
            )
    (out / "fonts.css").write_text("".join(css), encoding="utf-8", newline="\n")
    (out / "OFL.txt").write_text("%s\n\n%s" % (COPYRIGHT, LICENSE.read_text(encoding="utf-8")),
                                 encoding="utf-8", newline="\n")

    def kib(n):
        return "%.0f KiB" % (n / 1024)

    total = sum(sizes.values())
    print("%s: %d slices of %d code points in %d weights, %s; fonts.css %s" % (
        out, len(parts), len(mapped), len(FACES), kib(total), kib((out / "fonts.css").stat().st_size)))
    print("the Latin slice: %s" % ", ".join("%s %s" % (f, kib(n)) for f, n in sizes.items() if "-latin." in f))


def main():
    ap = argparse.ArgumentParser(description="Fetch the fonts shipped with the core, and write their "
                                             "metrics and web fonts.")
    sub = ap.add_subparsers(dest="command", required=True)
    sub.add_parser("fetch", help="download the fonts into target/fonts/, unless they are there already")
    m = sub.add_parser("metrics", help="write the metrics the renderer lays out text with")
    m.add_argument("--check", action="store_true", help="check that the metrics file is current; write nothing")
    w = sub.add_parser("web", help="write the web fonts of the viewers' page")
    w.add_argument("--out", type=Path, default=WEB, help="write them into OUT instead of apps/viewer-ui/fonts/")
    args = ap.parse_args()
    if args.command == "fetch":
        for path in fetch():
            print(path)
    elif args.command == "metrics":
        metrics(args.check)
    else:
        web(args.out.resolve())


if __name__ == "__main__":
    main()
