"""Compare two versions of the standard, for the pages that show what changed.

compare(old, new) takes the two versions as pandoc reads them (pandoc's JSON) and gives one document: the
text of the new version, in which what only the old one has is marked as removed and what only the new
one has is marked as added. build.py makes the page from it with the same filter and template as the
standard's own page.

- Sections are matched by their numbers, 9.4 with 9.4. A section that only one version has is added or
  removed as a whole, with its subsections.
- Within a section the blocks (paragraphs, lists, tables, code) are matched in order, the most similar
  ones first; lists are compared item by item and tables row by row.
- Text is compared word by word, and Chinese character by character; inline code, emphasis and links
  count as one word. Text of which less than half of the shorter version is kept is shown removed and
  added as a whole.
- Code is compared line by line.

The marks: a Span or a Div of class diff-ins or diff-del, a table row of one of these classes, and on a
code block the attribute data-diff, with one character per line: "=" kept, "-" removed, "+" added.
take() gives either version back from the document; build.py checks that both come back whole.
"""

from __future__ import annotations

import json
import re
from difflib import SequenceMatcher

INS, DEL = "diff-ins", "diff-del"
SIMILAR = 0.5  # blocks at least this similar (0 to 1) are compared inside; others are replaced as a whole
CJK = "".join(f"{chr(a)}-{chr(b)}" for a, b in (  # the characters and signs of Chinese, Japanese and Korean
    (0x2E80, 0x2FFF), (0x3000, 0x303F), (0x3040, 0x30FF), (0x3400, 0x4DBF), (0x4E00, 0x9FFF),
    (0xF900, 0xFAFF), (0xFE30, 0xFE4F), (0xFF00, 0xFFEF)))
WORD = re.compile(rf"[{CJK}]|[^\W{CJK}]+|\s+|.", re.S)  # a Chinese character, a word, spaces, or a sign


def key(x) -> str:
    return json.dumps(x, ensure_ascii=False, sort_keys=True)


def plain(x) -> str:
    """The text of an element or a list of them, roughly as pandoc.utils.stringify gives it."""
    out = []

    def walk(y):
        if isinstance(y, list):
            for z in y:
                walk(z)
        elif isinstance(y, dict):
            t, c = y.get("t"), y.get("c")
            if t == "Str":
                out.append(c)
            elif t in ("Space", "SoftBreak", "LineBreak"):
                out.append(" ")
            elif t in ("Code", "CodeBlock", "Math", "RawInline", "RawBlock"):
                out.append(c[1])
            elif c is not None:
                walk(c)

    walk(x)
    return "".join(out)


def words(x) -> list[str]:
    return [w for w in WORD.findall(plain(x)) if not w.isspace()]


def similarity(x, y) -> float:
    """How alike two blocks, list items or table rows are: the share of words they have in common, in any
    order (which is quick to count); 0 for blocks of different kinds."""
    if isinstance(x, dict) and kind(x) != kind(y):
        return 0.0
    return SequenceMatcher(None, words(x), words(y), autojunk=False).quick_ratio()


def kind(b: dict) -> str:
    return "Para" if b["t"] in ("Para", "Plain") else b["t"]


# --- sequences ---------------------------------------------------------------------------------------

def edits(a: list, b: list, weight) -> list[list]:
    """a turned into b, as runs ["=", items], ["-", items] and ["+", items]; removals come before the
    additions next to them, and short kept runs between changes are folded into the changes."""
    runs = []
    for tag, i1, i2, j1, j2 in SequenceMatcher(None, a, b, autojunk=False).get_opcodes():
        if tag == "equal":
            runs.append(["=", b[j1:j2]])
        else:
            runs += [["-", a[i1:i2]]] * (i2 > i1) + [["+", b[j1:j2]]] * (j2 > j1)
    return fold(runs, weight)


def tidy(runs: list[list]) -> list[list]:
    """Join neighbouring runs: the changes between two kept runs become one removal and one addition."""
    out, gone, new = [], [], []
    for op, items in runs + [["=", []]]:
        if op == "-":
            gone += items
        elif op == "+":
            new += items
        else:
            out += [["-", gone]] * bool(gone) + [["+", new]] * bool(new)
            gone, new = [], []
            if items:
                if out and out[-1][0] == "=":
                    out[-1] = ["=", out[-1][1] + items]
                else:
                    out.append(["=", items])
    return out


def fold(runs: list[list], weight) -> list[list]:
    """A kept run between two changes, and no longer than the larger side of either, joins the changes
    (as diff-match-patch's semantic cleanup does), so that a rewritten sentence does not read as a
    patchwork of single words or characters."""
    runs = tidy(runs)
    while True:
        for k in range(1, len(runs) - 1):
            if runs[k][0] != "=":
                continue
            size = sum(map(weight, runs[k][1]))
            sides = []
            for step in (-1, 1):
                j, gone, new = k + step, 0, 0
                while 0 <= j < len(runs) and runs[j][0] != "=":
                    w = sum(map(weight, runs[j][1]))
                    gone, new = (gone + w, new) if runs[j][0] == "-" else (gone, new + w)
                    j += step
                sides.append(max(gone, new))
            if all(sides) and size <= min(sides):
                runs = tidy(runs[:k] + [["-", runs[k][1]], ["+", runs[k][1]]] + runs[k + 1:])
                break
        else:
            return runs


def pairs(a: list, b: list, alike) -> list[tuple]:
    """Items of a matched with items of b, in order, so that the matched ones are as alike as possible:
    [(i, j)], with None on the side that has no match. Unmatched items of a come first."""
    n, m = len(a), len(b)
    s = [[alike(x, y) for y in b] for x in a]
    best = [[0.0] * (m + 1) for _ in range(n + 1)]
    for i in range(n - 1, -1, -1):
        for j in range(m - 1, -1, -1):
            both = best[i + 1][j + 1] + s[i][j] if s[i][j] >= SIMILAR else 0.0
            best[i][j] = max(best[i + 1][j], best[i][j + 1], both)
    out, i, j = [], 0, 0
    while i < n or j < m:
        if i < n and j < m and s[i][j] >= SIMILAR and best[i][j] == best[i + 1][j + 1] + s[i][j]:
            out.append((i, j))
            i, j = i + 1, j + 1
        elif i < n and (j == m or best[i][j] == best[i + 1][j]):
            out.append((i, None))
            i += 1
        else:
            out.append((None, j))
            j += 1
    return out


def matched(a: list, b: list, alike):
    """Walk two lists: ("=", None, y) for what both have, (x, y) for a pair alike enough to compare
    inside, (x, None) and (None, y) for the rest."""
    for tag, i1, i2, j1, j2 in SequenceMatcher(None, [key(x) for x in a], [key(y) for y in b],
                                               autojunk=False).get_opcodes():
        if tag == "equal":
            for y in b[j1:j2]:
                yield "=", y
        else:
            xs, ys = a[i1:i2], b[j1:j2]
            for i, j in pairs(xs, ys, alike):
                yield (xs[i] if i is not None else None), (ys[j] if j is not None else None)


# --- inlines -----------------------------------------------------------------------------------------

def tokens(inlines: list) -> list[tuple]:
    """Words, Chinese characters, signs and spaces of the text; other inlines (code, emphasis, links) whole."""
    out = []
    for el in inlines:
        if el["t"] == "Str":
            out += [("w", w, len(w)) for w in WORD.findall(el["c"])]
        elif el["t"] in ("Space", "SoftBreak"):
            out.append(("w", " ", 1))
        else:
            out.append(("x", key(el), max(1, len(plain(el)))))
    return out


def inlines(toks: list[tuple]) -> list:
    out = []
    for t, v, _ in toks:
        if t == "x":
            out.append(json.loads(v))
        elif v == " ":
            out.append({"t": "Space"})
        elif out and out[-1]["t"] == "Str":
            out[-1] = {"t": "Str", "c": out[-1]["c"] + v}
        else:
            out.append({"t": "Str", "c": v})
    return out


def span(cls: str, content: list) -> dict:
    return {"t": "Span", "c": [["", [cls], []], content]}


def diff_inlines(a: list, b: list, whole: bool = True) -> list | None:
    """b with the words that changed from a marked; None, when whole is allowed, if less than half of the
    shorter of the two is kept: added text or removed text alone is always marked within."""
    ta, tb = tokens(a), tokens(b)
    runs = edits(ta, tb, lambda t: t[2])
    kept = sum(t[2] for op, items in runs if op == "=" for t in items)
    if whole and kept < SIMILAR * min(sum(t[2] for t in ta), sum(t[2] for t in tb)):
        return None
    out = []
    for op, items in runs:
        out += inlines(items) if op == "=" else [span(INS if op == "+" else DEL, inlines(items))]
    return out


# --- blocks ------------------------------------------------------------------------------------------

def div(cls: str, blocks: list) -> dict:
    return {"t": "Div", "c": [["", [cls], []], blocks]}


def wrapped(gone: list, new: list) -> list:
    return [div(DEL, gone)] * bool(gone) + [div(INS, new)] * bool(new)


def diff_blocks(a: list, b: list) -> list:
    out, gone, new = [], [], []
    for x, y in matched(a, b, similarity):
        if x is None:
            new.append(y)
        elif y is None:
            gone.append(x)
        else:
            out += wrapped(gone, new) + ([y] if x == "=" else diff_block(x, y))
            gone, new = [], []
    return out + wrapped(gone, new)


def diff_block(x: dict, y: dict) -> list:
    t = x["t"]
    if kind(x) == "Para":
        merged = diff_inlines(x["c"], y["c"])
        return [{"t": y["t"], "c": merged}] if merged is not None else wrapped([x], [y])
    if t == "CodeBlock":
        return [diff_code(x, y)]
    if t in ("BulletList", "OrderedList"):
        return [diff_list(x, y)]
    if t == "Table":
        merged = diff_table(x, y)
        return [merged] if merged else wrapped([x], [y])
    if t == "BlockQuote":
        return [{"t": t, "c": diff_blocks(x["c"], y["c"])}]
    if t == "Div" and x["c"][0] == y["c"][0]:
        return [{"t": t, "c": [y["c"][0], diff_blocks(x["c"][1], y["c"][1])]}]
    return wrapped([x], [y])


def diff_code(x: dict, y: dict) -> dict:
    lines, marks = [], ""
    for op, items in edits(x["c"][1].split("\n"), y["c"][1].split("\n"), lambda s: len(s) + 1):
        lines += items
        marks += op * len(items)
    ident, classes, attrs = y["c"][0]
    return {"t": "CodeBlock", "c": [[ident, classes, attrs + [["data-diff", marks]]], "\n".join(lines)]}


def diff_list(x: dict, y: dict) -> dict:
    ordered = x["t"] == "OrderedList"
    items_a, items_b = (x["c"][1], y["c"][1]) if ordered else (x["c"], y["c"])
    items = []
    for p, q in matched(items_a, items_b, similarity):
        if p == "=":
            items.append(q)
        else:
            items.append([div(DEL, p)] if q is None else [div(INS, q)] if p is None else diff_blocks(p, q))
    return {"t": y["t"], "c": [y["c"][0], items] if ordered else items}


def mark_row(row: list, cls: str) -> list:
    (ident, classes, attrs), cells = row
    return [[ident, classes + [cls], attrs], cells]


def diff_rows(a: list, b: list) -> list:
    out = []
    for p, q in matched(a, b, similarity):
        if p == "=":
            out.append(q)
        elif q is None or p is None or len(p[1]) != len(q[1]):
            out += ([mark_row(p, DEL)] if p is not None else []) + ([mark_row(q, INS)] if q is not None else [])
        else:
            out.append([q[0], [c[:4] + [diff_blocks(d[4], c[4])] for d, c in zip(p[1], q[1])]])
    return out


def diff_table(x: dict, y: dict) -> dict | None:
    """A table compared row by row, when both have the same columns and one body; otherwise None."""
    _, _, cols_a, head_a, bodies_a, _ = x["c"]
    attr, caption, cols, head, bodies, foot = y["c"]
    if len(cols_a) != len(cols) or len(bodies_a) != 1 or len(bodies) != 1:
        return None
    (_, _, heads_a, rows_a), (battr, n, heads, rows) = bodies_a[0], bodies[0]
    return {"t": "Table", "c": [attr, caption, cols, [head[0], diff_rows(head_a[1], head[1])],
                                [[battr, n, diff_rows(heads_a, heads), diff_rows(rows_a, rows)]], foot]}


# --- sections ----------------------------------------------------------------------------------------

def number(header: dict) -> str | None:
    """The section number of a heading, as rnl.lua gives it: "9.4", "A", "A.1"."""
    text = plain(header["c"][2])
    m = re.match(r"(\d+[.\d]*)\s", text) or re.fullmatch(r"(\d+)", text)
    if m:
        return m.group(1).removesuffix(".")
    m = re.match(r"(?:Annex|附录) ([A-Z])\s", text) or re.match(r"([A-Z]\.\d+)\s", text)
    return m.group(1) if m else None


def sections(blocks: list) -> list[list]:
    """The blocks cut before each heading, as [key, blocks]: the number of the section, or for a heading
    without one its text; the blocks before the first heading come first, with the key ""."""
    out, seen = [["", []]], {}
    for b in blocks:
        if b["t"] == "Header":
            k = number(b) or "~" + plain(b["c"][2])
            seen[k] = seen.get(k, 0) + 1
            out.append([k if seen[k] == 1 else f"{k}~{seen[k]}", [b]])
        else:
            out[-1][1].append(b)
    return out


def marked(x) -> bool:
    if isinstance(x, list):
        return any(marked(y) for y in x)
    if not isinstance(x, dict):
        return False
    if x.get("t") in ("Span", "Div") and {INS, DEL} & set(x["c"][0][1]):
        return True
    if x.get("t") == "CodeBlock":
        return any(k == "data-diff" and set(v) != {"="} for k, v in x["c"][0][2])
    if x.get("t") == "Table":
        rows = [r for part in (x["c"][3], x["c"][5]) for r in part[1]]
        rows += [r for body in x["c"][4] for r in body[2] + body[3]]
        if any({INS, DEL} & set(r[0][1]) for r in rows):
            return True
    return marked(x.get("c"))


def whole(secs: list[list], cls: str) -> list:
    """Sections that only one version has, each in one Div with its subsections, so that pandoc makes the
    Div the section itself."""
    out, level = [], None
    for _, blocks in secs:
        head = blocks[0]["c"][0] if blocks and blocks[0]["t"] == "Header" else None
        if out and head and level and head > level:
            out[-1]["c"][1] += blocks
        else:
            out.append(div(cls, blocks))
            level = head
    return [d for d in out if d["c"][1]]


def compare(old: dict, new: dict) -> tuple[dict, dict]:
    """The document that shows what changed from old to new, and which sections changed: a dict of lists of
    (key, heading text) under "changed", "added" and "removed"."""
    sa, sb = sections(old["blocks"]), sections(new["blocks"])
    out, summary = [], {"changed": [], "added": [], "removed": []}

    def title(blocks):
        return plain(blocks[0]["c"][2]) if blocks and blocks[0]["t"] == "Header" else ""

    for tag, i1, i2, j1, j2 in SequenceMatcher(None, [k for k, _ in sa], [k for k, _ in sb],
                                               autojunk=False).get_opcodes():
        if tag == "equal":
            for (k, a), (_, b) in zip(sa[i1:i2], sb[j1:j2]):
                if a and b and a[0]["t"] == "Header":
                    head = b[0] if key(a[0]) == key(b[0]) else {
                        "t": "Header", "c": b[0]["c"][:2] + [diff_inlines(a[0]["c"][2], b[0]["c"][2], whole=False)]}
                    merged = [head] + diff_blocks(a[1:], b[1:])
                else:
                    merged = diff_blocks(a, b)
                if marked(merged):
                    summary["changed"].append((k, title(b)))
                out += merged
        else:
            summary["removed"] += [(k, title(a)) for k, a in sa[i1:i2] if a]
            summary["added"] += [(k, title(b)) for k, b in sb[j1:j2] if b]
            out += whole(sa[i1:i2], DEL) + whole(sb[j1:j2], INS)
    return {"pandoc-api-version": new["pandoc-api-version"], "meta": new["meta"], "blocks": out}, summary


# --- the versions back, for the check ----------------------------------------------------------------

def take(blocks: list, which: str) -> list:
    """The blocks of one version, "old" or "new", from a document made by compare()."""
    drop, keep = (INS, DEL) if which == "old" else (DEL, INS)
    skip = "+" if which == "old" else "-"

    def bl(bs):
        return [y for b in bs for y in block(b)]

    def block(b):
        t, c = b["t"], b.get("c")
        if t == "Div":
            if drop in c[0][1]:
                return []
            return bl(c[1]) if keep in c[0][1] else [{"t": t, "c": [c[0], bl(c[1])]}]
        if t in ("Para", "Plain"):
            return [{"t": t, "c": inl(c)}]
        if t == "Header":
            return [{"t": t, "c": c[:2] + [inl(c[2])]}]
        if t == "CodeBlock":
            marks = dict(c[0][2]).get("data-diff")
            if marks is None:
                return [b]
            lines = [s for s, m in zip(c[1].split("\n"), marks) if m != skip]
            return [{"t": t, "c": [[c[0][0], c[0][1], [kv for kv in c[0][2] if kv[0] != "data-diff"]],
                                   "\n".join(lines)]}]
        if t == "BulletList":
            return [{"t": t, "c": [i for i in map(bl, c) if i]}]
        if t == "OrderedList":
            return [{"t": t, "c": [c[0], [i for i in map(bl, c[1]) if i]]}]
        if t == "BlockQuote":
            return [{"t": t, "c": bl(c)}]
        if t == "Table":
            attr, caption, cols, head, bodies, foot = c
            return [{"t": t, "c": [attr, caption, cols, [head[0], rows(head[1])],
                                   [[x[0], x[1], rows(x[2]), rows(x[3])] for x in bodies], [foot[0], rows(foot[1])]]}]
        return [b]

    def rows(rs):
        return [[[r[0][0], [x for x in r[0][1] if x != keep], r[0][2]], [cell[:4] + [bl(cell[4])] for cell in r[1]]]
                for r in rs if drop not in r[0][1]]

    def inl(xs):
        out = []
        for x in xs:
            if x["t"] == "Span" and drop in x["c"][0][1]:
                continue
            out += inl(x["c"][1]) if x["t"] == "Span" and keep in x["c"][0][1] else [x]
        return out

    return bl(blocks)


def outline(blocks: list) -> list[tuple]:
    """What a reader sees of the blocks, in order: the text of each paragraph, heading and code block,
    and where list items and table rows begin; used to check that take() gives a version back whole."""
    out = []

    def walk(bs):
        for b in bs:
            t, c = b["t"], b.get("c")
            if t in ("Para", "Plain"):
                out.append(("p", plain(c)))
            elif t == "Header":
                out.append(("h", plain(c[2])))
            elif t == "CodeBlock":
                out.append(("code", c[1]))
            elif t in ("BulletList", "OrderedList"):
                for item in (c if t == "BulletList" else c[1]):
                    out.append(("item",))
                    walk(item)
            elif t == "Table":
                _, _, _, head, bodies, foot = c
                for r in head[1] + [r for x in bodies for r in x[2] + x[3]] + foot[1]:
                    out.append(("row",))
                    for cell in r[1]:
                        walk(cell[4])
            elif t in ("Div", "BlockQuote"):
                walk(c[1] if t == "Div" else c)
            else:
                out.append((t, key(c)))

    walk(blocks)
    return out


def check(old: dict, new: dict, merged: dict) -> list[str]:
    """Both versions come back whole from the merged document; the first difference otherwise."""
    msgs = []
    for which, doc in (("old", old), ("new", new)):
        got, want = outline(take(merged["blocks"], which)), outline(doc["blocks"])
        if got != want:
            k = next((i for i, (g, w) in enumerate(zip(got, want)) if g != w), min(len(got), len(want)))
            msgs.append(f"the {which} version does not come back whole: at {k}, "
                        f"{got[k] if k < len(got) else 'nothing'} instead of {want[k] if k < len(want) else 'nothing'}")
    return msgs
