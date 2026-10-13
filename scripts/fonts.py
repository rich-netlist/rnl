"""The fonts shipped with the core (decision 0007): Noto Sans SC Regular and Bold, the Simplified
Chinese subset of Noto Sans CJK, under the SIL Open Font License 1.1.

The fonts are not in the repository. `fetch` downloads them from the noto-cjk repository at a pinned
tag into target/fonts/, which Git ignores, and checks their SHA-256 sums. From them:

- `metrics` writes crates/rnl-render/fonts/noto-sans-sc.metrics, what the renderer lays out text
  with (crates/rnl-render/src/text.rs reads it). The file is committed; `metrics --check` fails
  when it is not what the pinned fonts give.

The metrics file, little-endian:
    "RNLM", u16 version (1), u16 number of faces F (Regular, then Bold)
    per face: u16 units per em, i16 ascender, i16 descender, i16 line gap (hhea),
              i16 top of the em box (OS/2 sTypoAscender), u16 advance of .notdef
    u32 number of runs N
    N runs, by their first code point: u32 first code point, u16 number of code points,
              F × u16 advance in each face
A run is a range of code points that every face maps, each with the same advance in a face as the
others of the range. The faces map the same code points.

Usage, from the repository root (fetch needs only the standard library; metrics needs fontTools:
pip install fonttools):
    python scripts/fonts.py fetch             download the fonts, unless they are there already
    python scripts/fonts.py metrics           write the metrics file
    python scripts/fonts.py metrics --check   check that it is current, write nothing
metrics fetches the fonts first when they are missing.
"""
import argparse
import hashlib
import shutil
import struct
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "target" / "fonts"
METRICS = ROOT / "crates" / "rnl-render" / "fonts" / "noto-sans-sc.metrics"

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


def main():
    ap = argparse.ArgumentParser(description="Fetch the fonts shipped with the core, and write their metrics.")
    sub = ap.add_subparsers(dest="command", required=True)
    sub.add_parser("fetch", help="download the fonts into target/fonts/, unless they are there already")
    m = sub.add_parser("metrics", help="write the metrics the renderer lays out text with")
    m.add_argument("--check", action="store_true", help="check that the metrics file is current; write nothing")
    args = ap.parse_args()
    if args.command == "fetch":
        for path in fetch():
            print(path)
    else:
        metrics(args.check)


if __name__ == "__main__":
    main()
