"""What there is to translate in the RNL project, and how far each language is (decision 0015).

    python scripts/i18n/status.py                      the table: every surface, every language
    python scripts/i18n/status.py --lang zh            one language in detail: what is missing, what is stale
    python scripts/i18n/status.py files ja             the files of one language, whether they exist yet or not
    python scripts/i18n/status.py check                format errors fail; missing and stale translations are reported
    python scripts/i18n/status.py report --markdown    the table and the details as Markdown (--json for data)
    python scripts/i18n/status.py stamp README.zh.md   record that a translated document is in line with its source

The surfaces are listed in i18n/surfaces.json and the languages in i18n/languages.json. English is the
source of every surface; every other language is a translation, which is reported but never required.
--base REF also compares with REF, as a pull request does with its base: an English text that changed
while a translation written beside it did not is reported. --online also reads the catalogs that other
repositories publish. Standard library only; git for the history of documents and for --base.
"""

from __future__ import annotations

import argparse
import difflib
import hashlib
import html.parser
import json
import os
import re
import subprocess
import sys
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "scripts" / "spec"))
import site_i18n  # noqa: E402
import spectools  # noqa: E402

KINDS = ("document", "inline", "dictionary", "messages", "aliases", "external", "untracked", "not-translated")
UNITS = {"document": "file", "inline": "text", "dictionary": "text", "messages": "text", "aliases": "word",
         "external": "text"}
# the first line of a translated document: the source it was made from, by the start of its SHA-256
STAMP = re.compile(r"^<!-- translated from (\S+) sha256:([0-9a-f]{12}) -->\r?\n")
SPEC_STAMP = re.compile(r"^<!-- translated from en/(\S+) sha256:([0-9a-f]{12}) -->\r?\n")  # scripts/spec/build.py
ANY_STAMP = re.compile(r"^<!-- translated from[^\n]*\n?")
SUMMARY = re.compile(r"<!--\s*timeline-summary\s*(\{.*?\})\s*-->", re.S)
SITE_PLACEHOLDER = re.compile(r"\{\{?[a-z][a-z0-9_-]*\}?\}")
TAG = re.compile(r"<(/?[a-zA-Z][a-zA-Z0-9]*)\b")
NUMBERED = re.compile(r"\{\d+\}")
ACCESS_KEY = re.compile(r"&[^\s&)]")
SIMILAR = 0.6  # how alike an old English text and a new one must be to be reported as one changed text
DETAIL = 40  # findings of one kind listed per surface and language; the rest are counted


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def source_hash(text: str) -> str:
    """The hash a stamp records: the source's SHA-256 with line ends as LF, its first 12 hex digits."""
    return hashlib.sha256(text.replace("\r\n", "\n").encode("utf-8")).hexdigest()[:12]


def norm(text: str) -> str:
    return " ".join(text.split())


def short(text: str, n: int = 70) -> str:
    text = norm(text)
    return text if len(text) <= n else text[:n - 1] + "…"


@dataclass
class Finding:
    state: str  # missing, stale, changed, unstamped, unused, differs
    where: str
    text: str = ""


@dataclass
class Lang:
    done: int = 0
    stale: int = 0
    findings: list[Finding] = field(default_factory=list)


@dataclass
class Surface:
    id: str
    title: str
    kind: str
    total: int = 0
    langs: dict[str, Lang] = field(default_factory=dict)
    errors: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    checked: bool = True


class Project:
    def __init__(self, root: Path = ROOT, base: str | None = None, online: bool = False):
        self.root = root
        self.base = base
        self.online = online
        self.errors: list[str] = []
        self.config = json.loads(read(root / "i18n/languages.json"))
        try:
            self.registry = site_i18n.Languages(self.config)
        except (KeyError, ValueError) as e:
            raise SystemExit(f"i18n/languages.json: {e}")
        self.entries = {entry["code"]: entry for entry in self.config["languages"]}
        self.languages = [code for code in self.registry.codes if code != self.registry.default]
        for code in self.languages:
            if self.state(code) not in ("published", "incubating"):
                self.errors.append(f"i18n/languages.json: {code}: status is published or incubating")
        self.surfaces = json.loads(read(root / "i18n/surfaces.json"))["surfaces"]
        ids = [s.get("id") for s in self.surfaces]
        for s in self.surfaces:
            if not s.get("id") or ids.count(s["id"]) > 1:
                self.errors.append(f"i18n/surfaces.json: every surface has an id of its own ({s.get('id')!r})")
            if s.get("kind") not in KINDS:
                self.errors.append(f"i18n/surfaces.json: {s.get('id')}: kind is one of {', '.join(KINDS)}")
            for code in s.get("languages", []) + s.get("inline", []):
                if code not in self.languages:
                    self.errors.append(f"i18n/surfaces.json: {s['id']}: {code} is not a registered translation")
        self._history: dict[str, list[tuple[str, str, str]]] = {}

    def state(self, code: str) -> str:
        """published or incubating; read from the entry itself, as the registry gives other keys English's."""
        return self.entries[code].get("status", "incubating")

    def rel(self, path: Path) -> str:
        return path.relative_to(self.root).as_posix()

    def files(self, patterns) -> list[Path]:
        patterns = [patterns] if isinstance(patterns, str) else patterns
        return sorted({p for pattern in patterns for p in self.root.glob(pattern) if p.is_file()})

    def langs(self, s: dict) -> list[str]:
        return s.get("languages", self.languages)

    def translation(self, s: dict, lang: str, **values) -> str | None:
        t = s.get("translation")
        if isinstance(t, dict):
            t = t.get(lang, t.get("*"))
        return t.format(lang=lang, **values) if t else None

    # --- git ---------------------------------------------------------------------------------------------

    def git(self, *args: str) -> str | None:
        r = subprocess.run(["git", "-C", str(self.root), *args], capture_output=True, text=True, encoding="utf-8")
        return r.stdout if r.returncode == 0 else None

    def shallow(self) -> bool:
        return (self.git("rev-parse", "--is-shallow-repository") or "").strip() != "false"

    def at_base(self, rel: str) -> str | None:
        return self.git("show", f"{self.base}:{rel}") if self.base else None

    def history(self, rel: str) -> list[tuple[str, str, str]]:
        """(commit, date, hash of the file) for each commit that changed the file, newest first."""
        if rel not in self._history:
            found = []
            for line in (self.git("log", "--format=%H %cs", "-n", "300", "--", rel) or "").split("\n"):
                if line:
                    commit, date = line.split()
                    text = self.git("show", f"{commit}:{rel}")
                    found.append((commit, date, source_hash(text) if text is not None else ""))
            self._history[rel] = found
        return self._history[rel]

    def since(self, translation: str, source: str) -> int | None:
        """How many commits changed the source after the last one that changed the translation."""
        if self.shallow():
            return None
        last = (self.git("log", "-1", "--format=%H", "--", translation) or "").strip()
        if not last:
            return None
        count = self.git("rev-list", "--count", f"{last}..HEAD", "--", source)
        return int(count) if count else None


# --- documents: whole translated files, each stamped with the hash of its source --------------------------

def documents(p: Project, s: dict) -> Surface:
    out = Surface(s["id"], s["title"], s["kind"])
    exclude = set(s.get("exclude", []))
    sources = [f for f in p.files(s["source"]) if f.name not in exclude]
    out.total = len(sources)
    if not sources:
        out.errors.append(f"{s['id']}: the source {s['source']} matches no file")
    spec = s.get("stamp") == "spec"
    for lang in p.langs(s):
        st = out.langs[lang] = Lang()
        for src in sources:
            rel = p.translation(s, lang, name=src.name)
            path = p.root / rel
            if not path.is_file():
                st.findings.append(Finding("missing", rel))
                continue
            st.done += 1
            text, source = read(path), read(src)
            m = (SPEC_STAMP if spec else STAMP).match(text)
            if m is None and ANY_STAMP.match(text):
                out.errors.append(f"{rel}: its first line is not a stamp of the form "
                                  f"<!-- translated from {'en/' + src.name if spec else p.rel(src)} sha256:<12 hex digits> -->")
            elif m is None:
                n = p.since(rel, p.rel(src))
                if n:
                    st.stale += 1
                    st.findings.append(Finding("stale", rel, f"not stamped; {p.rel(src)} has changed in {n} "
                                                            f"commit{'s' * (n > 1)} since this file was last changed"))
                else:
                    st.findings.append(Finding("unstamped", rel, "not stamped" + (
                        f"; {p.rel(src)} has not changed since this file was last changed" if n == 0 else
                        "; no history to judge it by")))
            elif m.group(1) != (src.name if spec else p.rel(src)):
                out.errors.append(f"{rel}: stamped as translated from {m.group(1)}, but its source is "
                                  f"{src.name if spec else p.rel(src)}")
            elif m.group(2) != source_hash(source):
                st.stale += 1
                st.findings.append(Finding("stale", rel, stale_text(p, p.rel(src), m.group(2))))
            structure = s.get("structure")
            if structure and (not spec or spectools.compared(spectools.parts([src.name]))):  # as the build compares
                body = (SPEC_STAMP if spec else STAMP).sub("", text)
                diffs = spectools.compare(source, body, src.name, by_position=structure == "position")
                if not spec:  # the words the standard's Chinese does not use are its own concern
                    diffs = [d for d in diffs if ": old word " not in d]
                if diffs:
                    st.findings.append(Finding("differs", rel, f"{len(diffs)} difference{'s' * (len(diffs) > 1)} "
                                                              f"of structure from the source; the first: {short(diffs[0], 110)}"))
    return out


def stale_text(p: Project, source: str, stamped: str) -> str:
    """Where the source was when the document was translated, as far as the history shows."""
    for i, (commit, date, digest) in enumerate(p.history(source)):
        if digest == stamped:
            return (f"translated from {source} as of {commit[:8]} ({date}); it has changed in {i} "
                    f"commit{'s' * (i != 1)} since: git diff {commit[:8]} -- {source}")
    return f"translated from a version of {source} ({stamped}) that its history does not show"


# --- inline: translations written beside the English in the site's source files ----------------------------

def groups(p: Project, text: str) -> list[tuple[int, dict[str, str]]]:
    """(line, {language: text}) of each translation group, as scripts/site_i18n.py reads them."""
    return [(text.count("\n", 0, m.start()) + 1, dict(p.registry.block.findall(m.group())))
            for m in p.registry.group.finditer(text)]


def catalog(p: Project, rel: str, errors: list[str]) -> dict:
    """A catalog keyed by English text: {English: translation, or null where there is none yet}."""
    path = p.root / rel
    if not path.is_file():
        return {}
    try:
        data = json.loads(read(path))
    except json.JSONDecodeError as e:
        errors.append(f"{rel}: not valid JSON: {e}")
        return {}
    if not isinstance(data, dict) or not all(isinstance(v, (str, type(None))) for v in data.values()):
        errors.append(f"{rel}: a catalog is an object whose values are texts, or null where there is none yet")
        return {}
    return data


def inline(p: Project, s: dict) -> Surface:
    out = Surface(s["id"], s["title"], s["kind"])
    files = p.files(s["source"])
    units = []
    for path in files:
        rel, text = p.rel(path), read(path)
        out.errors += p.registry.errors(text, rel)
        units += [(rel, line, blocks) for line, blocks in groups(p, text)]
    out.total = len(units)
    beside = s.get("inline", [])
    for lang in p.langs(s):
        st = out.langs[lang] = Lang()
        catalogs = {}
        for rel, line, blocks in units:
            if lang not in beside and rel not in catalogs:
                catalogs[rel] = catalog(p, p.translation(s, lang, path=rel.removeprefix("site/src/")), out.errors)
            text = blocks.get(lang) if lang in beside else catalogs[rel].get(norm(blocks["en"]))
            if text is None:
                st.findings.append(Finding("missing", f"{rel}:{line}", short(blocks["en"])))
                continue
            st.done += 1
            en = blocks["en"]
            if not set(SITE_PLACEHOLDER.findall(text)) <= set(SITE_PLACEHOLDER.findall(en)):
                st.findings.append(Finding("differs", f"{rel}:{line}", "placeholders differ from the English: "
                                           f"{sorted(set(SITE_PLACEHOLDER.findall(en)))} and {sorted(set(SITE_PLACEHOLDER.findall(text)))}"))
        if p.base and lang in beside:
            for path in files:
                old = p.at_base(p.rel(path))
                if old is not None:
                    for (_, before), (line, after) in changed_pairs(groups(p, old), groups(p, read(path))):
                        if lang in after and norm(before.get(lang, "")) == norm(after[lang]):
                            st.stale += 1
                            st.findings.append(Finding("changed", f"{p.rel(path)}:{line}",
                                                       f"the English changed from {short(before['en'], 50)!r} to "
                                                       f"{short(after['en'], 50)!r}; the {lang} beside it did not"))
    if beside:
        out.notes.append(f"Written beside the English in the source files: {', '.join(beside)}. "
                         "Whether it is stale shows only against a base (--base).")
    return out


def changed_pairs(old: list, new: list) -> list[tuple]:
    """The groups of the new text whose English differs from that of the group they replace in the old."""
    a, b = [norm(g[1]["en"]) for g in old], [norm(g[1]["en"]) for g in new]
    pairs = []
    for op, i1, i2, j1, j2 in difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes():
        if op != "replace":
            continue
        if i2 - i1 == j2 - j1:
            pairs += list(zip(old[i1:i2], new[j1:j2]))
            continue
        for j in range(j1, j2):
            best = max(range(i1, i2), key=lambda i: difflib.SequenceMatcher(None, a[i], b[j]).ratio())
            if similar(a[best], b[j]):
                pairs.append((old[best], new[j]))
    return pairs


def similar(old: str, new: str) -> bool:
    """Whether a new English text is likely an old one changed: alike enough, and not too short to tell."""
    return min(len(old), len(new)) >= 12 and difflib.SequenceMatcher(None, old, new).ratio() >= SIMILAR


# --- dictionaries: {"en": …, "zh": …} in data ------------------------------------------------------------

def dictionaries(p: Project, s: dict, text: str | None, rel: str) -> dict[str, dict]:
    """The dictionaries of one file, by a name that stays the same when their texts change."""
    if text is None:
        return {}
    form = s["format"]
    if form == "release-summary":
        m = SUMMARY.search(text)
        return {rel: json.loads(m.group(1))} if m else {}
    data = json.loads(text)
    if form == "registry-messages":
        by = {entry["code"]: entry.get("messages", {}) for entry in data["languages"]}
        return {key: {code: by[code][key] for code in by if key in by[code]} for key in by.get("en", {})}
    found = {}

    def walk(value, path):
        if isinstance(value, dict):
            if isinstance(value.get("en"), str) and all(isinstance(v, str) for v in value.values()):
                found[path] = value
                return
            for k, v in value.items():
                walk(v, f"{path}.{k}")
        elif isinstance(value, list):
            for i, v in enumerate(value):
                key = v.get("id", i) if isinstance(v, dict) else i
                walk(v, f"{path}[{key}]")
    walk(data, rel)
    return found


def dictionary(p: Project, s: dict) -> Surface:
    out = Surface(s["id"], s["title"], s["kind"])
    units, before = {}, {}
    for path in p.files(s["source"]):
        rel = p.rel(path)
        try:
            units |= dictionaries(p, s, read(path), rel)
            before |= dictionaries(p, s, p.at_base(rel), rel)
        except json.JSONDecodeError as e:
            out.errors.append(f"{rel}: not valid JSON: {e}")
    out.total = len(units)
    for lang in p.langs(s):
        st = out.langs[lang] = Lang()
        for name, texts in units.items():
            if lang not in texts:
                st.findings.append(Finding("missing", name, short(texts["en"])))
                continue
            st.done += 1
            old = before.get(name)
            if old and norm(old["en"]) != norm(texts["en"]) and norm(old.get(lang, "")) == norm(texts[lang]):
                st.stale += 1
                st.findings.append(Finding("changed", name, f"the English changed from {short(old['en'], 50)!r} to "
                                                            f"{short(texts['en'], 50)!r}; the {lang} did not"))
    return out


# --- messages: the viewers' catalogs, keyed by the English text ----------------------------------------------

class StaticText(html.parser.HTMLParser):
    """The texts of the body of a page that i18n.js translates when it starts: its text nodes outside scripts,
    styles and drawings, and the attributes title, placeholder and aria-label."""

    def __init__(self):
        super().__init__()
        self.found, self.skip, self.body = [], 0, False

    def handle_starttag(self, tag, attrs):
        self.body |= tag == "body"
        if tag in ("script", "style", "svg"):
            self.skip += 1
        if self.body:
            self.found += [v for k, v in attrs if k in ("title", "placeholder", "aria-label") and v and v.strip()]

    def handle_endtag(self, tag):
        if tag in ("script", "style", "svg") and self.skip:
            self.skip -= 1

    def handle_data(self, data):
        if self.body and not self.skip and data.strip():
            self.found.append(data.strip())


def scan_viewer(p: Project, source: dict) -> dict[str, str]:
    """Every interface text the scan finds, with the first file it is in."""
    found = {}

    def add(text, rel):
        if re.search(r"[A-Za-z]", NUMBERED.sub("", text)):  # not a template of placeholders alone
            found.setdefault(text, rel)
    for path in p.files(source.get("js", [])):
        rel, text = p.rel(path), read(path)
        for m in re.finditer(r"\bm`", text):  # each m`…`, the ones nested in another's ${…} too
            add(template(text, m.end())[0], rel)
        for m in re.finditer(r"\bm\(\s*(\"(?:[^\"\\]|\\.)*\"|'(?:[^'\\]|\\.)*')\s*[,)]", text):
            add(js_string(m.group(1)), rel)
    if source.get("commands") and (p.root / source["commands"]).is_file():
        rel = source["commands"]

        def walk(value):
            if isinstance(value, dict):
                if str(value.get("id", "")).startswith("language-"):
                    return  # a language is named in its own language, as in every menu
                for k, v in value.items():
                    if k == "label" and isinstance(v, str):
                        add(v, rel)
                    walk(v)
            elif isinstance(value, list):
                for v in value:
                    walk(v)
        walk(json.loads(read(p.root / rel)))
    if source.get("html") and (p.root / source["html"]).is_file():
        parser = StaticText()
        parser.feed(read(p.root / source["html"]))
        for text in parser.found:
            add(text, source["html"])
    for path in p.files(source.get("rust", [])):
        for m in re.finditer(r"\btext\([^,()]*(?:\([^()]*\))?[^,()]*,\s*\"((?:[^\"\\]|\\.)*)\"\)", read(path)):
            add(json.loads(f'"{m.group(1)}"'), p.rel(path))
    return found


def template(text: str, i: int) -> tuple[str, int]:
    """The text of the template literal that starts at text[i], after its backquote, as m() gets it: its
    strings, cooked, with each ${…} as {0}, {1}…; and the index after its closing backquote."""
    parts, piece, n = [], [], 0
    while i < len(text):
        c = text[i]
        if c == "\\":
            piece.append(text[i:i + 2])
            i += 2
        elif c == "`":
            parts.append(cooked("".join(piece)))
            return "".join(parts), i + 1
        elif text.startswith("${", i):
            parts.append(cooked("".join(piece)) + "{%d}" % n)
            piece, n = [], n + 1
            i = expression_end(text, i + 2)
        else:
            piece.append(c)
            i += 1
    return "".join(parts + piece), i


def expression_end(text: str, i: int) -> int:
    """The index after the } that closes the ${ before text[i], past the strings and templates inside."""
    depth = 1
    while i < len(text):
        c = text[i]
        if c in "\"'":
            m = re.compile(rf"{c}(?:[^{c}\\\n]|\\.)*{c}").match(text, i)
            i = m.end() if m else i + 1
        elif c == "`":
            i = template(text, i + 1)[1]
        elif c == "{":
            depth, i = depth + 1, i + 1
        elif c == "}":
            depth, i = depth - 1, i + 1
            if not depth:
                return i
        else:
            i += 1
    return i


ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "b": "\b", "f": "\f", "v": "\v", "0": "\0"}


def cooked(raw: str) -> str:
    """A string of a script as JavaScript reads it, its escapes such as \\n, \\u2026 and \\` unescaped."""
    def one(m):
        e = m.group(1)
        if e.startswith("u{"):
            return chr(int(e[2:-1], 16))
        if e[0] in "ux" and len(e) > 1:
            return chr(int(e[1:], 16))
        return ESCAPES.get(e, "" if e == "\n" else e)
    return re.sub(r"\\(u\{[0-9a-fA-F]+\}|u[0-9a-fA-F]{4}|x[0-9a-fA-F]{2}|.)", one, raw, flags=re.S)


def js_string(literal: str) -> str:
    return cooked(literal[1:-1])


def message_errors(rel: str, data: dict) -> list[str]:
    errors = []
    for key, value in data.items():
        if not isinstance(value, str):
            continue
        if not set(NUMBERED.findall(value)) <= set(NUMBERED.findall(key)):  # one may be left out, as a plural's "s"
            errors.append(f"{rel}: {short(key, 50)!r}: a placeholder {{0}}, {{1}}… that the English does not have")
        if "\t" in key and value.rpartition("\t")[2] != key.rpartition("\t")[2]:
            errors.append(f"{rel}: {short(key, 50)!r}: the shortcut after the tab is not the English one")
        if len(ACCESS_KEY.findall(value)) > 1:
            errors.append(f"{rel}: {short(key, 50)!r}: more than one access key (&)")
    return errors


def messages(p: Project, s: dict) -> Surface:
    out = Surface(s["id"], s["title"], s["kind"])
    found = {text: rel for text, rel in scan_viewer(p, s["source"]).items() if text not in s.get("keep", [])}
    out.total = len(found)
    named = [s["source"].get("commands"), s["source"].get("html")]
    out.errors += [f"{s['id']}: {rel} does not exist" for rel in named if rel and not (p.root / rel).is_file()]
    for lang in p.langs(s):
        st = out.langs[lang] = Lang()
        rel = p.translation(s, lang)
        data = catalog(p, rel, out.errors)
        out.errors += message_errors(rel, data)
        missing = [text for text in found if not isinstance(data.get(text), str)]
        unused = [key for key in data if key not in found]
        st.done = len(found) - len(missing)
        paired = set()
        for text in missing:
            old = max(unused, key=lambda k: difflib.SequenceMatcher(None, k, text).ratio(), default=None)
            if old is not None and old not in paired and similar(old, text):
                paired.add(old)
                st.stale += 1
                st.findings.append(Finding("changed", found[text], f"{short(text, 50)!r} has no {lang}; it may be "
                                           f"the English of {short(old, 50)!r}, translated {short(data[old] or '', 30)!r}"))
            else:
                st.findings.append(Finding("missing", found[text], short(text)))
        st.findings += [Finding("unused", rel, f"{short(key)!r} is not found by the scan") for key in unused if key not in paired]
    if s.get("keep"):
        out.notes.append("Kept in English on purpose: " + ", ".join(s["keep"]) + ".")
    return out


# --- aliases: the alias package of a standard series ------------------------------------------------------

def aliases(p: Project, s: dict) -> Surface:
    out = Surface(s["id"], s["title"], s["kind"])
    words = json.loads(read(p.root / s["words"]))
    # the words of header records only are read before aliases apply (16.5)
    body = [k for k in words["keywords"] if not set(k.get("where") or []) <= {"header", "package-header"}]
    wanted = {"@" + k["keyword"] for k in body} | {q["name"] for k in body for q in k.get("params", [])}
    for group in ("kinds", "attributes", "style_properties", "subobjects"):
        wanted |= {item["name"] for item in words.get(group, [])}
    for path in p.files(s["packages"]):
        if not re.fullmatch(r"rnl-[a-z]{2,3}(-[A-Za-z0-9]+)?\.rnl", path.name) or path.name.startswith("rnl-std"):
            wanted |= set(re.findall(r"(\$[A-Za-z_][A-Za-z0-9_]*) *= *@DEFINE", read(path)))
    wanted -= set(s.get("no_alias", []))
    out.total = len(wanted)
    for lang in p.langs(s):
        st = out.langs[lang] = Lang()
        path = p.root / p.translation(s, lang)
        if not path.is_file():
            continue  # optional: a language need not have one
        given = {target for _, target in re.findall(r'@ALIAS\("([^"]*)",\s*"([^"]*)"\)', read(path))}
        st.done = len(wanted & given)
        st.findings += [Finding("missing", p.rel(path), word) for word in sorted(wanted - given)]
    if s.get("no_alias"):
        out.notes.append("Kept in English on purpose: " + ", ".join(s["no_alias"]) + ".")
    return out


# --- external: what other repositories publish ------------------------------------------------------------

def external(p: Project, s: dict) -> Surface:
    out = Surface(s["id"], s["title"], s["kind"])
    if not p.online:
        out.checked = False
        out.notes.append(f"In {s['repository']}; not read without --online.")
        return out
    try:
        with urllib.request.urlopen(s["url"], timeout=20) as r:
            data = json.loads(r.read().decode("utf-8"))
    except (OSError, ValueError) as e:
        out.checked = False
        out.notes.append(f"{s['url']} could not be read: {e}")
        return out
    if s["format"] == "gallery-index":
        units = {f"{e.get('id')}.{f}": e.get(f) or {} for e in data.get("entries", []) for f in ("title", "description")}
    else:
        units = {f"{r.get('name')} {r.get('version')}": r.get("summary") or {} for r in data.get("packages", [])}
    out.total = len(units)
    for lang in p.langs(s):
        st = out.langs[lang] = Lang()
        for name, texts in units.items():
            if lang in texts:
                st.done += 1
            else:
                st.findings.append(Finding("missing", name, short(texts.get("en", ""))))
    return out


ADAPTERS = {"document": documents, "inline": inline, "dictionary": dictionary, "messages": messages,
            "aliases": aliases, "external": external}


def survey(p: Project) -> list[Surface]:
    out = []
    for s in p.surfaces:
        if s.get("kind") in ADAPTERS:
            surface = ADAPTERS[s["kind"]](p, s)
        else:
            surface = Surface(s["id"], s["title"], s.get("kind", "?"), checked=False)
            surface.notes.append(s.get("reason") or s.get("where", ""))
        if s.get("notes"):
            surface.notes.append(s["notes"])
        out.append(surface)
    return out


# --- output -----------------------------------------------------------------------------------------------

def cell(surface: Surface, lang: str) -> str:
    if not surface.checked:
        return "—"
    st = surface.langs.get(lang)
    if st is None:
        return ""
    text = f"{st.done}/{surface.total}"
    return text + (f", {st.stale} stale" if st.stale else "")


def table(p: Project, surfaces: list[Surface], langs: list[str], markdown: bool = False) -> str:
    rows = [(s.id, UNITS.get(s.kind, s.kind), *(cell(s, lang) for lang in langs)) for s in surfaces
            if s.kind in UNITS]
    head = ("surface", "unit", *(f"{lang} ({p.state(lang)})" for lang in langs))
    if markdown:
        lines = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
        return "\n".join(lines + ["| " + " | ".join(r) + " |" for r in rows])
    widths = [max(len(str(r[i])) for r in [head, *rows]) for i in range(len(head))]
    return "\n".join("  ".join(str(v).ljust(w) for v, w in zip(r, widths)).rstrip() for r in [head, *rows])


def details(surface: Surface, lang: str, markdown: bool = False) -> list[str]:
    st = surface.langs.get(lang)
    if st is None:
        return []
    out = []
    for state in ("stale", "changed", "missing", "differs", "unstamped", "unused"):
        found = [f for f in st.findings if f.state == state]
        if not found:
            continue
        out.append(f"{state} ({len(found)}):")
        for f in found[:DETAIL]:
            line = f"{f.where}: {f.text}" if f.text else f.where
            out.append(f"- `{f.where}`" + (f": {f.text}" if f.text else "") if markdown else f"  {line}")
        if len(found) > DETAIL:
            out.append(f"{'- ' if markdown else '  '}… and {len(found) - DETAIL} more")
    return out


def report(p: Project, surfaces: list[Surface], langs: list[str], markdown: bool) -> str:
    out = [table(p, surfaces, langs, markdown), ""]
    errors = p.errors + [e for s in surfaces for e in s.errors]
    if errors:
        out += ["Format errors:" if not markdown else "**Format errors**", ""] + [f"- {e}" for e in errors] + [""]
    for lang in langs:
        for s in surfaces:
            lines = details(s, lang, markdown)
            if lines:
                if markdown:
                    out += [f"<details><summary>{s.id}, {lang}: {cell(s, lang)}</summary>", "", *lines, "", "</details>", ""]
                else:
                    out += [f"{s.id} ({s.title}), {lang}: {cell(s, lang)}", *lines, ""]
    others = [s for s in surfaces if s.kind not in UNITS or not s.checked]
    if others:
        out.append("**Not counted**" if markdown else "Not counted:")
        out += [f"- {s.id} ({s.kind}): {' '.join(s.notes)}" for s in others]
    return "\n".join(out).rstrip() + "\n"


def as_json(p: Project, surfaces: list[Surface]) -> str:
    return json.dumps({
        "languages": {code: p.state(code) for code in p.languages},
        "errors": p.errors + [e for s in surfaces for e in s.errors],
        "surfaces": [{"id": s.id, "title": s.title, "kind": s.kind, "total": s.total, "checked": s.checked,
                      "notes": s.notes, "languages": {lang: {"done": st.done, "stale": st.stale,
                                                             "findings": [f.__dict__ for f in st.findings]}
                                                      for lang, st in s.langs.items()}} for s in surfaces],
    }, ensure_ascii=False, indent=1) + "\n"


def files(p: Project, lang: str) -> list[str]:
    """The files that hold the translations of one language, by surface, whether they exist or not."""
    out = []
    for s in p.surfaces:
        if lang not in s.get("languages", [lang]) or s.get("kind") == "not-translated":
            continue
        if s["kind"] == "document":
            exclude = set(s.get("exclude", []))
            paths = [p.translation(s, lang, name=f.name) for f in p.files(s["source"]) if f.name not in exclude]
        elif s["kind"] == "inline":
            if lang in s.get("inline", []):
                out.append(f"{s['id']}: written beside the English in {', '.join(s['source'])}")
                continue
            paths = [p.translation(s, lang, path=p.rel(f).removeprefix("site/src/")) for f in p.files(s["source"])]
        elif s["kind"] == "dictionary":
            out.append(f"{s['id']}: the \"{lang}\" of the dictionaries in {', '.join([s['source']] if isinstance(s['source'], str) else s['source'])}")
            continue
        elif s["kind"] in ("messages", "aliases"):
            paths = [p.translation(s, lang)]
        elif s["kind"] == "external":
            out.append(f"{s['id']}: in {s['repository']}")
            continue
        else:
            out.append(f"{s['id']}: {s.get('where', '')}" + (f" (later {p.translation(s, lang)})" if s.get("translation") else ""))
            continue
        out += [f"{s['id']}: {path}{'' if (p.root / path).is_file() else '  (not yet)'}" for path in paths]
    return out


def stamp(p: Project, target: str, as_of: str | None = None) -> str:
    """Write the stamp of the source at the head of a translated document: of the source as it is now, or as
    it is in the commit as_of."""
    path = (Path.cwd() / target).resolve()
    rel = p.rel(path)
    for s in p.surfaces:
        if s.get("kind") != "document":
            continue
        for lang in p.langs(s):
            for src in p.files(s["source"]):
                if p.translation(s, lang, name=src.name) == rel:
                    if s.get("stamp") == "spec":
                        raise SystemExit(f"{rel}: the standard's chapters are stamped by python scripts/spec/build.py --stamp")
                    text = ANY_STAMP.sub("", read(path))  # a stamp, or a line meant as one
                    source = read(src) if as_of is None else p.git("show", f"{as_of}:{p.rel(src)}")
                    if source is None:
                        raise SystemExit(f"{p.rel(src)} is not in {as_of}")
                    line = f"<!-- translated from {p.rel(src)} sha256:{source_hash(source)} -->\n"
                    path.write_text(line + text, encoding="utf-8", newline="\n")
                    return f"{rel}: stamped as translated from {p.rel(src)} as it is " + (f"in {as_of}" if as_of else "now")
    raise SystemExit(f"{rel} is not a translated document listed in i18n/surfaces.json")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("command", nargs="?", default="table", choices=["table", "files", "check", "report", "stamp"])
    ap.add_argument("target", nargs="?", help="the language of files, the document of stamp")
    ap.add_argument("--lang", help="only this language, in detail")
    ap.add_argument("--base", help="also compare with this commit, as a pull request does with its base")
    ap.add_argument("--online", action="store_true", help="also read what other repositories publish")
    ap.add_argument("--markdown", action="store_true", help="report as Markdown")
    ap.add_argument("--json", action="store_true", help="report as JSON")
    ap.add_argument("--github", action="store_true", help="check: also write GitHub annotations")
    ap.add_argument("--summary", type=Path, help="check: also append the Markdown report to this file")
    ap.add_argument("--as-of", help="stamp: the source as it is in this commit, such as the one that last "
                                    "changed the translation, not as it is now")
    ap.add_argument("--root", type=Path, default=ROOT, help=argparse.SUPPRESS)
    args = ap.parse_args(argv)
    sys.stdout.reconfigure(encoding="utf-8")
    p = Project(args.root.resolve(), args.base, args.online)
    if args.command == "files":
        if not re.fullmatch(r"[a-z]{2,3}(?:-[A-Za-z0-9]+)*", args.target or "") or args.target == p.registry.default:
            raise SystemExit("files: the code of a language, such as zh or ja")
        if args.target not in p.languages:
            print(f"{args.target} is not in i18n/languages.json yet; these are the files it would have:")
        print("\n".join(files(p, args.target)))
        return 0
    if args.command == "stamp":
        if not args.target:
            raise SystemExit("stamp: the translated document to stamp")
        print(stamp(p, args.target, args.as_of))
        return 0
    if args.lang and args.lang not in p.languages:
        raise SystemExit(f"--lang: one of {', '.join(p.languages)}")
    surfaces = survey(p)
    langs = [args.lang] if args.lang else p.languages
    errors = p.errors + [e for s in surfaces for e in s.errors]
    if args.json:
        print(as_json(p, surfaces), end="")
    elif args.command == "table" and not args.lang:
        print(table(p, surfaces, langs))
        if errors:
            print("\n" + "\n".join(f"error: {e}" for e in errors))
    else:
        print(report(p, surfaces, langs, args.markdown), end="")
    if args.command == "check":
        if args.summary:
            with args.summary.open("a", encoding="utf-8") as f:
                f.write("## Translations\n\n" + report(p, surfaces, langs, True))
        if args.github:
            for e in errors:
                print(f"::error::{e}")
            for s in surfaces:
                for lang, st in s.langs.items():
                    for f in [f for f in st.findings if f.state in ("stale", "changed")][:DETAIL]:
                        where = f.where.split(":")
                        loc = f"file={where[0]}" + (f",line={where[1]}" if len(where) > 1 and where[1].isdigit() else "")
                        print(f"::warning {loc}::{s.id}, {lang}: {f.text}")
        return 1 if errors else 0
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
