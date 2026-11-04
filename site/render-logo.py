"""The logo of the homepage: what the viewer's easter egg shows, as an SVG with its text as outlines.

The homepage's "RNL" is the one the viewer draws when the logo is clicked: the RNL core lays brand/rnl-logo.sp out with the
caption that apps/web/build.mjs gives it (site/render-logo-core.mjs), and the viewer shows the text in Noto Sans SC Bold,
the font of the core's metrics. An <img> cannot load a web font, so here the text becomes paths, drawn from the same font
file, and the logo looks the same on every machine. The text of the SVG that the core writes is placed by its x, its baseline
y and its font size, and stretched to its textLength (lengthAdjust="spacingAndGlyphs"): the outlines are placed and
stretched the same way.

The viewBox is the painted bounds (strokes and the ink of the text) of the viewer's whole drawing, "logo by RNL" included,
which is what the viewer fits into its frame (with a margin of 2px). The homepage has the drawing without that last line, in
the same viewBox, so that the frame of the viewer replaces it in the same place and size. brand/rnl-logo.sp stays as it is.

    python site/render-logo.py [--core bindings/runtime-wasm/dist] [--fonts target/fonts] [--check]

writes site/src/assets/brand/rnl-wordmark.svg and rnl-wordmark-dark.svg (the colours of the dark theme as the viewer's
darkColour gives them). --check writes nothing and fails when the files are not what the source gives. The font is
what `python scripts/fonts.py fetch` writes; the core is built by bindings/runtime-wasm/build.mjs.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from logo_inputs import inputs_hash  # noqa: E402
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "site" / "src" / "assets" / "brand"


def num(x):
    return ("%.2f" % x).rstrip("0").rstrip(".") if abs(x) >= 0.005 else "0"


def dark_colour(colour):
    """apps/viewer-ui/background.js darkColour, as scripts/brand.mjs copies it."""
    if not re.fullmatch(r"#(?:[\da-f]{3,4}|[\da-f]{6}|[\da-f]{8})", colour, re.I):
        return colour
    h = colour[1:]
    full = "".join(c + c for c in h) if len(h) <= 4 else h
    rgb = [int(full[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    lightness = (min(rgb) + max(rgb)) / 2
    reflected = 1 - lightness if lightness < 0.25 else lightness
    if reflected == lightness:
        return colour
    shift = reflected - lightness
    return "#" + "".join("%02x" % round((c + shift) * 255) for c in rgb) + full[6:]


class Face:
    def __init__(self, path):
        self.font = TTFont(path)
        self.glyphs = self.font.getGlyphSet()
        self.cmap = self.font.getBestCmap()
        self.upem = self.font["head"].unitsPerEm
        self.hmtx = self.font["hmtx"]

    def outline(self, text, x, y, size, length):
        """The path of a text placed as the SVG says: baseline at (x, y), stretched to `length` wide."""
        scale = size / self.upem
        natural = sum(self.hmtx[self.cmap[ord(c)]][0] for c in text) * scale
        stretch = length / natural
        pen, bounds = SVGPathPen(self.glyphs, ntos=num), BoundsPen(self.glyphs)
        cursor = 0
        for c in text:
            name = self.cmap[ord(c)]
            matrix = (scale * stretch, 0, 0, -scale, x + cursor * scale * stretch, y)
            self.glyphs[name].draw(TransformPen(pen, matrix))
            self.glyphs[name].draw(TransformPen(bounds, matrix))
            cursor += self.hmtx[name][0]
        return pen.getCommands(), bounds.bounds, stretch


def attr(tag, name):
    m = re.search(r'\b%s="([^"]*)"' % name, tag)
    return m.group(1) if m else None


def convert(svg, faces):
    """The painted elements of a core SVG, with the text as paths, and the bounds of what they paint."""
    parts, boxes = [], []
    for m in re.finditer(r"<(path|circle|text)\b([^>]*?)(?:/>|>(.*?)</text>)", svg, re.S):
        kind, rest, body = m.group(1), m.group(2), m.group(3)
        tag = rest
        if kind == "path":
            nums = [float(v) for v in re.findall(r"-?\d+(?:\.\d+)?", attr(tag, "d"))]
            xs, ys = nums[0::2], nums[1::2]
            half = float(attr(tag, "stroke-width") or 0) / 2 if attr(tag, "stroke") not in (None, "none") else 0
            boxes.append((min(xs) - half, min(ys) - half, max(xs) + half, max(ys) + half))
            parts.append('<path d="%s" fill="%s" stroke="%s"%s/>' % (
                re.sub(r"-?\d+(?:\.\d+)?", lambda v: num(float(v.group(0))), attr(tag, "d")), attr(tag, "fill"), attr(tag, "stroke"),
                ' stroke-width="%s" stroke-linecap="round" stroke-linejoin="round"' % attr(tag, "stroke-width") if attr(tag, "stroke-width") else ""))
        elif kind == "circle":
            cx, cy, r = (float(attr(tag, k)) for k in ("cx", "cy", "r"))
            boxes.append((cx - r, cy - r, cx + r, cy + r))
            parts.append('<circle cx="%s" cy="%s" r="%s" fill="%s"/>' % (num(cx), num(cy), num(r), attr(tag, "fill")))
        else:
            d, box, _ = faces["bold" if 'font-weight="bold"' in tag else "regular"].outline(body, float(attr(tag, "x")), float(attr(tag, "y")), float(attr(tag, "font-size")), float(attr(tag, "textLength")))
            boxes.append(box)
            parts.append('<path d="%s" fill="%s"/>' % (d, attr(tag, "fill")))
    return parts, boxes


def union(boxes):
    return (min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes))


def make(cores, faces, source_hash):
    # the frame of the viewer: the painted bounds of the whole drawing of the egg
    egg_parts, egg_boxes = convert(cores["egg"], faces)
    frame = union(egg_boxes)
    homepage_parts, homepage_boxes = convert(cores["homepage"], faces)
    ink = union(homepage_boxes)
    out = {}
    for name, dark in (("rnl-wordmark.svg", False), ("rnl-wordmark-dark.svg", True)):
        body = "\n".join(homepage_parts)
        if dark:
            body = re.sub(r'(fill|stroke)="(#[0-9a-fA-F]+)"', lambda m: '%s="%s"' % (m.group(1), dark_colour(m.group(2))), body)
        view = (frame[0], frame[1], frame[2] - frame[0], frame[3] - frame[1])
        out[name] = (
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="%s %s %s %s">\n'
            '<!-- Written by site/render-logo.py from brand/rnl-logo.sp and the caption of the viewer\'s interactive logo '
            '(sources sha256 %s), drawn by the RNL core without "logo by RNL"; the text is outlines of Noto Sans SC Bold. Do not edit. -->\n%s\n</svg>\n'
            % (num(view[0]), num(view[1]), num(view[2]), num(view[3]), source_hash, body))
    return out, {"frame": frame, "homepage_ink": ink}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--core", default=str(ROOT / "bindings" / "runtime-wasm" / "dist"))
    ap.add_argument("--fonts", default=str(ROOT / "target" / "fonts"))
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    run = subprocess.run(["node", str(ROOT / "site" / "render-logo-core.mjs"), "--core", args.core], capture_output=True, text=True, encoding="utf-8")
    if run.returncode:
        sys.exit(run.stderr)
    cores = json.loads(run.stdout)
    source_hash = inputs_hash(ROOT)
    fonts = Path(args.fonts)
    faces = {"bold": Face(fonts / "NotoSansSC-Bold.otf"), "regular": Face(fonts / "NotoSansSC-Regular.otf")}
    files, info = make(cores, faces, source_hash)
    ok = True
    for name, text in files.items():
        path = OUT / name
        if args.check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print("%s is not what brand/rnl-logo.sp and the core give: run python site/render-logo.py" % name)
                ok = False
        else:
            path.write_text(text, encoding="utf-8", newline="\n")
            print("wrote", path.relative_to(ROOT))
    f = info["frame"]
    print("frame (the viewer's drawing) %.1f x %.1f, ratio %.4f; the homepage's ink ends at x = %.1f of %.1f" % (
        f[2] - f[0], f[3] - f[1], (f[2] - f[0]) / (f[3] - f[1]), info["homepage_ink"][2] - f[0], f[2] - f[0]))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
