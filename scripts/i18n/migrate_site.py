"""Move the translations written beside the English in the site's source files into the catalogs of
their languages (decision 0015, step 3b).

    python scripts/i18n/migrate_site.py           move every block but the English of site/src into i18n/
    python scripts/i18n/migrate_site.py --check   say what it would move, and change nothing

Each group of a source file keeps its English only; each other block moves to the catalog of its file
and language, i18n/<lang>/site/<path under site/src>.json, under the key its English gives (the English
with its runs of white space as one space), after the entries the catalog already has.

Where one file says one English text in two ways in a language, the group of each other way names a
context on its English, <en ctx=NAME>, and its key is NAME|English: the way most groups say it, the
first of equals, keeps the plain key, and CONTEXTS gives the names of the others. An English left empty,
for a note that only a translation has, always names a context: the translation's own words when they
are English, as in the module notes of tools.html, else a name of CONTEXTS. A file whose catalog already
has a key with another translation, or a way that CONTEXTS does not name, stops the run, which then
changes nothing.

Running it again moves what was written beside the English since; it changes nothing that has moved.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from site_i18n import LANGUAGES as I18N  # noqa: E402

SOURCES = ("*.html", "assets/*.js")
# (file, language, translation) -> the context of the group that says its English that way
CONTEXTS = {
    ("index.html", "zh", "看得懂"): "readable",  # "Yes" in the comparison: can a person read the circuit
    ("index.html", "zh", "只有网表"): "step-0",  # "Netlist only", the first step of the demo
    ("index.html", "zh", "场景"): "tabs",  # "Scenarios", the label of the scenarios' tabs
    ("index.html", "zh", "许可证"): "chip",  # "License", a chip of a scenario
    ("tools.html", "zh", "打开网页版查看器"): "cta",  # "Open the web viewer", the call to action
    ("tools.html", "zh", "层次"): "sample",  # "Hierarchy", a sample of the viewer
}


def slug(text: str) -> str | None:
    """A context made of a translation's own words, when they are English: "Host adapter" -> host-adapter."""
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-") if re.fullmatch(r"[A-Za-z0-9 .\-]+", text) else None


class Stop(Exception):
    pass


def plan(path: str, text: str, catalogs: dict[str, dict]) -> tuple[str, dict[str, dict]]:
    """The file's text with its groups' English only, and what each language's catalog of it gains."""
    found = []  # (match, blocks, context) of the groups that have blocks to move
    for m in I18N.group.finditer(text):
        blocks, context = I18N.parts(m.group())
        if len(blocks) > 1:
            found.append((m, blocks, context))
    ways = Counter()  # (language, English, translation) -> how many groups say it so
    first = {}  # (language, English) -> the first translation, for equals
    for _, blocks, context in found:
        for lang, value in blocks.items():
            if lang != I18N.default and not context:
                ways[lang, I18N.key(blocks), value] += 1
                first.setdefault((lang, I18N.key(blocks)), value)

    def usual(lang, english):
        counts = {value: n for (l, e, value), n in ways.items() if (l, e) == (lang, english)}
        return max(counts, key=lambda v: (counts[v], v == first[lang, english]))

    added = {lang: {} for lang in I18N.codes if lang != I18N.default}
    out, at = [], 0
    for m, blocks, context in found:
        english = I18N.key(blocks)
        if not context:
            others = [(lang, v) for lang, v in blocks.items()
                      if lang != I18N.default and (not english or v != usual(lang, english))]
            names = {CONTEXTS.get((path, lang, v)) or (slug(v) if not english else None) for lang, v in others}
            if None in names or len(names) > 1:
                raise Stop(f"{path}:{text.count(chr(10), 0, m.start()) + 1}: {english!r} is said as "
                           f"{[v for _, v in others]!r} too: name its context in CONTEXTS")
            context = names.pop() if names else ""
        key = I18N.key(blocks, context)
        for lang, value in blocks.items():
            if lang == I18N.default:
                continue
            had = catalogs.get(lang, {}).get(key, added[lang].get(key))
            if had is not None and had != value:
                raise Stop(f"{path}: the {lang} catalog has {key[:60]!r} as {had!r}, the source as {value!r}")
            added[lang][key] = value
        out += [text[at:m.start()], f"<{I18N.default}{f' ctx={context}' if context else ''}>{blocks[I18N.default]}</{I18N.default}>"]
        at = m.end()
    return "".join(out + [text[at:]]), {lang: entries for lang, entries in added.items() if entries}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="say what it would move, and change nothing")
    args = ap.parse_args(argv)
    src = ROOT / "site" / "src"
    files = sorted({p for pattern in SOURCES for p in src.glob(pattern)})
    writes, moved = {}, Counter()
    try:
        for file in files:
            path = file.relative_to(src).as_posix()
            catalogs = {lang: I18N.catalog(lang, path) for lang in I18N.codes if lang != I18N.default}
            text = file.read_text(encoding="utf-8")
            new, added = plan(path, text, catalogs)
            if new != text:
                writes[file] = new
            for lang, entries in added.items():
                target = ROOT / "i18n" / lang / "site" / f"{path}.json"
                writes[target] = json.dumps({**catalogs[lang], **entries}, ensure_ascii=False, indent=2) + "\n"
                moved[lang] += len(entries)
    except Stop as e:
        print(e, file=sys.stderr)
        return 1
    sources = [f for f in writes if f.suffix != ".json"]
    for lang, n in moved.items():
        print(f"{lang}: {n} translations moved out of {len(sources)} source files")
    if args.check:
        print("\n".join(f"would write {f.relative_to(ROOT).as_posix()}" for f in writes))
        return 0
    for file, text in writes.items():
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {len(writes)} files" if writes else "nothing to move")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
