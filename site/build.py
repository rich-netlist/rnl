"""Build the static project site in the locales registered in i18n/languages.json.

The pages in src/ are written in English, with each text to translate marked:

    <en>The netlist defines the circuit</en>

The same markup is used in HTML text, in attribute values and in JavaScript strings. The
translations are in the catalogs of i18n/<lang>/site/, one for each file, such as index.html.json,
keyed by the English (decision 0015); timeline.json.json has those of the timeline and of the
releases' summaries, and messages.json the shared messages. A text without a translation is shown
in English; an empty translation remains empty. The shared scripts/site_i18n.py registry generates
menus, fonts and locale output. The build checks that
- groups have an English source, no translation beside it and no duplicate, stray or nested
  language tags;
- the English pages contain no Chinese characters, apart from Chinese content meant for every page:
  elements marked lang="zh-CN", and in scripts the blocks between /*zh-CN*/ and /*/zh-CN*/ (RNL
  written with the Chinese alias package, for example).

The page of the tools links to the files of the desktop viewer's newest release. Its version comes
from the newest tag viewer-vX.Y.Z of the repository (a tag with anything after the version, such as
a release candidate's, does not count), or from --viewer-version. The parts of a page between
<!--release--> and <!--/release--> are kept when there is a release, with {{viewer-version}} (such
as 0.1.0) and {{viewer-tag}} (viewer-v0.1.0) filled in; the parts between <!--no-release--> and
<!--/no-release--> are kept when there is none. The build checks that these blocks close and do not
nest, and that the fields are only used in <!--release--> or <!--linux-release--> blocks.
Linux download blocks are kept only when the selected tag contains tauri.linux.conf.json with both
Linux bundle targets; <!--no-linux-release--> is kept otherwise, including for older Windows-only tags.

Output (site/dist/ by default, ignored by Git), for each page in PAGES:
    zh/index.html, zh/language.html, zh/tools.html, zh/packages.html, zh/assets/, zh/*-standalone.html   Chinese
    en/index.html, en/language.html, en/tools.html, en/packages.html, en/assets/, en/*-standalone.html   English
    index.html                                     leads to zh/ or en/, by the browser's first language
    language.html, tools.html, packages.html, *-standalone.html   lead to the same pages under zh/
The standalone files have the styles and scripts inlined and link the other pages to the published
site; packages.html, the gallery of the standard packages, has none. The pages of the standard (zh/spec/
and en/spec/, one folder for each version), the web viewer (viewer/) and the gallery's data (gallery/),
which the pages link to and load, are added by .github/workflows/pages.yml.

Until 2026-10-18 the Chinese pages were at the root of the site, where their addresses still lead: the
pages at the root that lead elsewhere keep the query and the section of the address, so that an old
link such as tools.html#desktop opens zh/tools.html#desktop. The root itself leads to zh/ when the
browser's first language is Chinese, and to en/ otherwise, or without scripts.
The old addresses of the pages of the standard are kept by scripts/spec/build.py.

Usage, from the repository root (standard library only, and git for the tags):
    python site/build.py              build into site/dist/
    python site/build.py --check      check the sources, write nothing
    python site/build.py --out DIR    build into DIR instead
    python site/build.py --zip FILE   also pack the built site into FILE
    python site/build.py --viewer-version 0.1.0   link the files of this release of the desktop
                                      viewer, whatever the tags say; "none" for no release yet
    python site/build.py --check --rnl PATH       also check and render site/examples/*.sp with rnl
"""
import argparse
from base64 import b64encode
from datetime import date, datetime, timedelta, timezone
from html import escape
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "scripts"))
from site_i18n import LANGUAGES as I18N, SHOWN_IN_ENGLISH_BELOW  # noqa: E402
SRC = HERE / "src"
EXAMPLES = HERE / "examples"
DEFAULT_OUT = HERE / "dist"
SITE_URL = "https://rich-netlist.github.io/rnl/"

# Locale directories come from the shared registry. Legacy root URLs still lead to Chinese pages.
LANGS = [(code, code + "/") for code in I18N.codes]
CODES = I18N.codes
DIRS = dict(LANGS)
HOME = DIRS[I18N.default]
MOVED = DIRS[I18N.legacy]

# Each page and the name of its single-file version; None for a page that has none, such as the
# gallery of the standard packages, which loads its data from gallery/ (site/gallery/build.mjs), and the
# page for agents, whose downloads are written by scripts/skill/build.py.
PAGES = {
    "index.html": "rnl-intro-standalone.html",
    "language.html": "rnl-language-standalone.html",
    "tools.html": "rnl-tools-standalone.html",
    "packages.html": None,
    "gallery.html": None,
    "agents.html": None,
    "timeline.html": None,
}
# Pages added after the Chinese pages moved to zh/ have no old address at the root.
NEW_PAGES = ("agents.html", "gallery.html", "timeline.html")

GROUP = I18N.group
CJK = re.compile("[%s]" % "".join("%s-%s" % (chr(a), chr(b)) for a, b in
                                  [(0x3000, 0x303F), (0x3400, 0x9FFF), (0xF900, 0xFAFF), (0xFF00, 0xFFEF)]))
ZH_ELEMENT = re.compile(r"<(\w+)\b[^>]*\blang=\"(?!en(?:-|\"))[^\"]*\"[^>]*>.*?</\1>", re.S)
ZH_BLOCK = re.compile(r"/\*zh-CN\*/.*?/\*/zh-CN\*/", re.S)

STYLE_TAG = re.compile(r'<link rel="stylesheet" href="assets/([\w.-]+\.css)">')
SCRIPT_TAG = re.compile(r'<script src="assets/([\w.-]+\.js)"></script>')
NOTICE = "<!-- Built by site/build.py from site/src/. Edit the source, not this file. -->\n"

# The desktop viewer's releases: a tag viewer-vX.Y.Z and a GitHub release of that name with the files
# the page of the tools links to (.github/workflows/viewer-release.yml makes them).
VIEWER_TAG = re.compile(r"viewer-v(\d+)\.(\d+)\.(\d+)")
VERSION = re.compile(r"\d+\.\d+\.\d+")
# The parts of a page kept with a release of the desktop viewer, or without one, and the fields
# filled in from the release.
RELEASE = re.compile(r"<!--((?:no-)?(?:(?:linux|macos)-)?release)-->(.*?)<!--/\1-->", re.S)
RELEASE_MARK = re.compile(r"<!--/?(?:no-)?(?:(?:linux|macos)-)?release-->")
FIELD = re.compile(r"\{\{(viewer-[a-z-]+)\}\}")
FIELDS = ("viewer-version", "viewer-tag")
TIMELINE = "{{timeline}}"
TIMELINE_FULL = "{{timeline-full}}"
# The homepage shows this many of the latest published entries; timeline.html has all of them.
HOME_ENTRIES = 3
# Release dates are modern Asia/Shanghai dates (UTC+08:00, without DST).
SHANGHAI = timezone(timedelta(hours=8))
RELEASE_VERSION = r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?"
TIMELINE_TAG = re.compile(rf"(?:(viewer|core)-v({RELEASE_VERSION})|spec-(\d+\.\d+)-r(\d+))")
SUMMARY = re.compile(r"<!--\s*timeline-summary\s*\n(.*?)-->", re.S)


class BuildError(Exception):
    pass


def git_text(*args):
    try:
        return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True,
                              encoding="utf-8", check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None


def timeline_tags():
    """Annotated tag creation times, or commit times for lightweight tags; no network access."""
    listed = git_text("for-each-ref", "--format=%(refname:short)%00%(creatordate:iso8601-strict)", "refs/tags")
    return [{"tag": tag, "date": stamp} for tag, stamp in
            (line.split("\0", 1) for line in (listed or "").splitlines()) if TIMELINE_TAG.fullmatch(tag)]


def timeline_date(value):
    try:
        if len(value) == 10:
            return datetime.combine(date.fromisoformat(value), datetime.min.time(), SHANGHAI)
        stamp = datetime.fromisoformat(value.replace("Z", "+00:00"))
        if stamp.tzinfo is None:
            raise ValueError("timestamp needs a timezone")
        return stamp.astimezone(SHANGHAI)
    except (ValueError, TypeError) as error:
        raise BuildError(f"Invalid timeline date {value!r}: {error}") from error


def localized_values(value, label):
    try:
        english = I18N.text(value, I18N.default)
    except ValueError as error:
        raise BuildError(f"{label}: {error}") from error
    if not english.strip():
        raise BuildError(f"{label} needs a nonempty English source")
    return value


def release_summary(tag):
    """Prefer the tagged notes; old tags can be backfilled in the checkout without moving tags."""
    match = TIMELINE_TAG.fullmatch(tag)
    line, version, _, _ = match.groups()
    candidates = []
    if line:
        path = (f"apps/desktop/release-notes/{version}.md" if line == "viewer"
                else f"release-notes/core/{version}.md")
        candidates.append(git_text("show", f"{tag}:{path}") or "")
        if (ROOT / path).is_file():
            candidates.append((ROOT / path).read_text(encoding="utf-8"))
    # Standard releases already take their notes from the annotated tag's message.
    candidates.append(git_text("for-each-ref", "--format=%(contents)", f"refs/tags/{tag}") or "")
    for notes in candidates:
        blocks = SUMMARY.findall(notes)
        if len(blocks) > 1:
            raise BuildError(f"{tag}: more than one timeline-summary block")
        if not blocks:
            continue
        try:
            summary = localized_values(json.loads(blocks[0]), f"{tag} timeline-summary")
        except ValueError as error:
            raise BuildError(f"{tag}: invalid timeline-summary JSON: {error}") from error
        if any("\n" in text or "\r" in text for text in summary.values()):
            raise BuildError(f"{tag}: timeline-summary must be one paragraph per language")
        return summary
    raise BuildError(f"{tag}: missing English timeline-summary in release notes; see release-notes/README.md")


def timeline_events(data, tags, summary=release_summary):
    """Manual milestones override matching tags; sort by Shanghai time, with one latest item."""
    events = []
    claimed = set()
    ids = set()
    for original in data["milestones"]:
        event = dict(original)
        key = event["id"]
        if key in ids:
            raise BuildError(f"Duplicate timeline id: {key}")
        ids.add(key)
        for tag in event.get("tags", []):
            if tag in claimed:
                raise BuildError(f"Timeline tag appears in more than one manual milestone: {tag}")
            claimed.add(tag)
        events.append(event)
    for release_tag in tags:
        tag = release_tag["tag"]
        if tag in claimed:
            continue
        match = TIMELINE_TAG.fullmatch(tag)
        if not match:
            continue
        line, version, series, revision = match.groups()
        if line == "viewer":
            title = {code: I18N.message(code, "timeline_viewer", version=version) for code in CODES}
        elif line == "core":
            title = {code: I18N.message(code, "timeline_core", version=version) for code in CODES}
        else:
            title = {code: I18N.message(code, "timeline_spec", series=series, revision=revision) for code in CODES}
        events.append({"id": tag, "date": release_tag["date"], "title": title,
                       "summary": summary(tag), "url": f"https://github.com/rich-netlist/rnl/releases/tag/{tag}"})
    for event in events:
        event["stamp"] = timeline_date(event["date"])
        localized_values(event["title"], f"{event['id']} title")
        localized_values(event.get("summary", event.get("description")), f"{event['id']} summary/description")
        if event.get("start_date") and timeline_date(event["start_date"]) > event["stamp"]:
            raise BuildError(f"{event['id']}: timeline date range is reversed")
    return sorted(events, key=lambda event: (event["stamp"], event["id"]))


def translations_beside(value, path):
    """The places in `value`, the data of a timeline, of the dictionaries with a translation beside their English."""
    if isinstance(value, dict):
        if isinstance(value.get(I18N.default), str):
            return [path] if set(value) - {I18N.default} else []
        return [found for key, v in value.items() for found in translations_beside(v, f"{path}.{key}")]
    if isinstance(value, list):
        return [found for i, v in enumerate(value)
                for found in translations_beside(v, f"{path}[{v.get('id', i) if isinstance(v, dict) else i}]")]
    return []


def timeline_html(data=None, tags=None, full=False):
    """The rows of the timeline: for the homepage, its latest published entries, a link to the full timeline and
    what comes next; for timeline.html (`full`), every published entry, the latest first."""
    if data is None:
        try:
            data = json.loads((SRC / "timeline.json").read_text(encoding="utf-8"))
        except (OSError, ValueError) as error:
            raise BuildError(f"src/timeline.json: {error}") from error
        beside = translations_beside(data, "src/timeline.json")
        if beside:
            raise BuildError("\n".join(f"{where}: a translation written beside the English: translations go into "
                                       f"i18n/<lang>/site/timeline.json.json, keyed by the English" for where in beside))
    events = timeline_events(data, timeline_tags() if tags is None else tags)
    if not events:
        raise BuildError("The timeline has no published milestones")

    def localized(values, html=False):
        # translations of the timeline's texts, its milestones' and the releases' summaries, are in the
        # catalog of timeline.json (decision 0015); released notes may still carry their own
        return I18N.markup(I18N.with_catalog(values, "timeline.json"), html)

    def link(event):
        url = event.get("url")
        if not url:
            return ""
        if re.search(r'[\s<>"\']', url) or re.match(r"(?:[a-z][a-z0-9+.-]*:|//)", url, re.I) and not url.startswith("https://"):
            raise BuildError(f"{event['id']}: unsafe timeline URL {url!r}")
        labels = localized_values(event.get("link_label", {code: I18N.message(code, "timeline_notes") for code in CODES}), "timeline link label")
        return f' <a href="{escape(url, quote=True)}">{localized(labels)}</a>'

    rows = []
    shown = events if full else events[-HOME_ENTRIES:]
    for event in shown:
        day = event["stamp"].date().isoformat()
        if event.get("start_date"):
            start = timeline_date(event["start_date"]).date().isoformat()
            day = localized({code: I18N.message(code, "timeline_range", start=start, end=day) for code in CODES})
        current = ' class="now"' if event is events[-1] else ""
        # Handwritten descriptions retain their reviewed inline links/code. Release summaries are plain text.
        description = localized(event.get("summary", event.get("description")), html="summary" not in event)
        rows.append(f'        <li{current}><time>{day}</time><b>{localized(event["title"])}</b>'
                    f'<p>{description}{link(event)}</p></li>')
    if full:
        return "\n".join(reversed(rows))
    if len(shown) < len(events):
        rows.append('        <li class="tl-more"><a href="timeline.html">'
                    + localized({code: I18N.message(code, "timeline_all") for code in CODES}) + '</a></li>')
    rows.append('        <li class="tl-sep">' + localized({code: I18N.message(code, "timeline_next") for code in CODES}) + '</li>')
    for event in data.get("next", []):
        title = localized(localized_values(event["title"], "next milestone title"))
        description = localized(localized_values(event["description"], "next milestone description"), html=True)
        rows.append(f'        <li class="next"><b>{title}</b><p>{description}</p></li>')
    return "\n".join(rows)


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def blank(m):
    return re.sub(r"[^\n]", " ", m.group(0))


def check_groups(text, name):
    return I18N.errors(text, name)


def check_release(text, name):
    """Report the marks of release blocks that are not part of a block, blocks inside blocks, and
    fields that are unknown or outside a <!--release--> block, where there is no release to give them."""
    errors = []
    for m in RELEASE.finditer(text):
        inner = RELEASE_MARK.search(m.group(2))
        if inner:
            errors.append("%s:%d: %s inside a release block" % (name, line_of(text, m.start(2) + inner.start()), inner.group(0)))
    outside = RELEASE.sub(blank, text)
    for m in RELEASE_MARK.finditer(outside):
        errors.append("%s:%d: %s is not part of a release block" % (name, line_of(outside, m.start()), m.group(0)))
    for m in FIELD.finditer(text):
        if m.group(1) not in FIELDS:
            errors.append("%s:%d: {{%s}} is not a field; the fields are %s"
                          % (name, line_of(text, m.start()), m.group(1), ", ".join("{{%s}}" % f for f in FIELDS)))
    kept = RELEASE.sub(lambda m: blank(m) if m.group(1) in ("release", "linux-release", "macos-release") else m.group(0), text)
    for m in FIELD.finditer(kept):
        errors.append("%s:%d: {{%s}} outside a <!--release--> block" % (name, line_of(kept, m.start()), m.group(1)))
    return errors


def release(text, version, linux=False, macos=False):
    """Keep the release blocks that apply to `version`, None for no release, with the fields filled in."""
    with_release = version is not None
    enabled = {"release": with_release, "no-release": not with_release,
               "linux-release": with_release and linux, "no-linux-release": not (with_release and linux),
               "macos-release": with_release and macos, "no-macos-release": not (with_release and macos)}
    text = RELEASE.sub(lambda m: m.group(2) if enabled[m.group(1)] else "", text)
    if not with_release:
        return text
    values = {"viewer-version": version, "viewer-tag": "viewer-v" + version}
    return FIELD.sub(lambda m: values[m.group(1)], text)


def newest_viewer_release():
    """The version of the desktop viewer's newest release, such as "0.1.0", from the tags of the
    repository; None when it has no tag viewer-vX.Y.Z, or git cannot tell."""
    try:
        listed = subprocess.run(["git", "tag", "--list", "viewer-v*"], cwd=ROOT, capture_output=True, text=True, check=True)
    except (OSError, subprocess.CalledProcessError):
        return None
    versions = [tuple(int(n) for n in m.groups()) for m in map(VIEWER_TAG.fullmatch, listed.stdout.split()) if m]
    return "%d.%d.%d" % max(versions) if versions else None


def has_linux_release(version):
    """Only link Linux files when the selected tag includes the Linux bundle configuration.
    Earlier Windows-only tags must keep working even after this site change is deployed."""
    if version is None:
        return False
    try:
        config = subprocess.run(
            ["git", "show", f"viewer-v{version}:apps/desktop/tauri.linux.conf.json"],
            cwd=ROOT, capture_output=True, text=True, check=True)
        return set(json.loads(config.stdout)["bundle"]["targets"]) >= {"appimage", "deb"}
    except (OSError, subprocess.CalledProcessError, ValueError, KeyError, TypeError):
        return False


def has_macos_release(version):
    """Older tags must not acquire links to Mac artifacts that were never built."""
    if version is None:
        return False
    try:
        config = subprocess.run(
            ["git", "show", f"viewer-v{version}:apps/desktop/tauri.macos.conf.json"],
            cwd=ROOT, capture_output=True, text=True, check=True)
        return set(json.loads(config.stdout)["bundle"]["targets"]) >= {"app", "dmg"}
    except (OSError, subprocess.CalledProcessError, ValueError, KeyError, TypeError):
        return False


def localize(text, index):
    return I18N.localize(text, CODES[index])


def check_leaks(text, name):
    """The English source must mark intentionally quoted non-English content."""
    masked = ZH_BLOCK.sub(blank, ZH_ELEMENT.sub(blank, text))
    lines = masked.split("\n")
    bad = sorted({line_of(masked, m.start()) for m in CJK.finditer(masked)})
    return ["%s:%d: Chinese text on a non-Chinese page: %s" % (name, n, lines[n - 1].strip()[:100]) for n in bad]


def sources():
    files = {page: (SRC / page).read_text(encoding="utf-8") for page in PAGES}
    for path in sorted((SRC / "assets").iterdir()):
        if path.suffix in (".css", ".js"):
            files["assets/" + path.name] = path.read_text(encoding="utf-8")
    return files


def examples():
    if not EXAMPLES.is_dir():
        return {}
    return {path.name: path.read_text(encoding="utf-8")
            for path in sorted(EXAMPLES.iterdir()) if path.suffix == ".sp"}


SAMPLES_SITE = re.compile(r"(?:samples/site/|')(ota5t[\w.-]*\.sp)")
COUNTS = re.compile(r"(\d+) errors?, (\d+) warnings?, (\d+) hints?")


def check_examples(files, ex):
    """Check that every samples/site/<name>.sp referenced in src/ exists in site/examples/, and that
    the 5T OTA netlist excerpts and coordinates in src/assets/scenes.js stay in step with them."""
    errors = []
    linked = {m.group(1) for text in files.values() for m in SAMPLES_SITE.finditer(text)}
    have = set(ex)
    for name in sorted(linked - have):
        errors.append("src/: references samples/site/%s, which is not in site/examples/" % name)
    for name in sorted(have - linked):
        errors.append("examples/%s: not referenced in src/" % name)
    scenes = files.get("assets/scenes.js", "")
    if not scenes or not ex:
        return errors

    def block(pattern):
        m = re.search(pattern, scenes, re.S)
        if not m:
            errors.append("src/assets/scenes.js: missing %s" % pattern)
            return ""
        return m.group(1)

    for bare, en in re.findall(r"u\d+:\s*\['([^']+)',\s*'([^']+)'\]", block(r"var U = \{(.*?)\n  \};")):
        if "ota5t-edits.sp" in ex and bare not in ex["ota5t-edits.sp"]:
            errors.append("examples/ota5t-edits.sp: missing %r from U in src/assets/scenes.js" % bare)
        for target in ("ota5t.sp", "ota5t-resolved.sp"):
            if target in ex and en not in ex[target]:
                errors.append("examples/%s: missing %r from U in src/assets/scenes.js" % (target, en))
    for zh in re.findall(r"u\d+:\s*'([^']+)'", block(r"var UZ = \{(.*?)\n  \};")):
        if "ota5t-zh.sp" in ex and zh not in ex["ota5t-zh.sp"]:
            errors.append("examples/ota5t-zh.sp: missing %r from UZ in src/assets/scenes.js" % zh)
    for line in re.findall(r"add\('(?:[hm]\d+|end)',\s*'([^']+)'", scenes):
        for target in ("ota5t.sp", "ota5t-zh.sp", "ota5t-edits.sp", "ota5t-resolved.sp", "ota5t_tb.sp"):
            if target in ex and line not in ex[target]:
                errors.append("examples/%s: missing %r from otaCode in src/assets/scenes.js" % (target, line))
    for tbl_name, targets in (("L_SYM", ("ota5t.sp", "ota5t-zh.sp", "ota5t-edits.sp")),
                              ("L_RESOLVED", ("ota5t-resolved.sp",))):
        for dev, x, y, o in re.findall(r"(M\d):\s*\[(-?\d+),\s*(-?\d+),\s*'(\w+)'\]",
                                       block(r"var %s = \{(.*?)\};" % tbl_name)):
            stmt = ('#%s @PLACE(%s, %s);' % (dev, x, y) if dev == "M5" and o == "R0"
                    else '#%s @PLACE(%s, %s) => @ORIENT("%s")' % (dev, x, y, o))
            for target in targets:
                if target in ex and stmt not in ex[target]:
                    errors.append("examples/%s: missing %r from %s in src/assets/scenes.js" % (target, stmt, tbl_name))
    for line, rest in re.findall(r"\{\s*t:\s*'([^']+)'([^}]*)\}", block(r"var E5_LINES = \[(.*?)\n  \];")):
        if "sig1:" in line:
            continue
        needle = line.lstrip("* ").strip() if "is-err" in rest else line.rstrip(";")
        if "ota5t-edits.sp" in ex and needle not in ex["ota5t-edits.sp"]:
            errors.append("examples/ota5t-edits.sp: missing %r from E5_LINES in src/assets/scenes.js" % needle)
    for line in re.findall(r"\{\s*t:\s*'([^']+)'", block(r"var E2_LINES = \[(.*?)\n  \];")):
        if I18N.tag.search(line):
            continue
        if "ota5t_tb.sp" in ex and line not in ex["ota5t_tb.sp"]:
            errors.append("examples/ota5t_tb.sp: missing %r from E2_LINES in src/assets/scenes.js" % line)
    return errors


def find_rnl(given):
    """The rnl command: --rnl, else $RNL, else the one cargo built here, else the one on PATH."""
    exe = ".exe" if os.name == "nt" else ""
    for cand in (given, os.environ.get("RNL"), ROOT / "target" / "debug" / ("rnl" + exe),
                 ROOT / "target" / "release" / ("rnl" + exe), shutil.which("rnl")):
        if cand and Path(cand).is_file():
            return Path(cand)
    raise BuildError("no rnl command: build it with `cargo build -p rnl-cli` or give --rnl PATH")


def check_examples_rnl(rnl):
    """Check that every file in site/examples/*.sp passes `rnl check` with 0 errors and 0 warnings
    and renders with `rnl render`."""
    errors = []
    with tempfile.TemporaryDirectory() as tmp:
        for idx, path in enumerate(sorted(EXAMPLES.glob("*.sp"))):
            rel = "examples/" + path.name
            chk = subprocess.run([str(rnl), "check", str(path)], capture_output=True, text=True, encoding="utf-8")
            m = COUNTS.search(chk.stdout)
            if not m:
                errors.append("%s: rnl check printed no summary:\n%s%s" % (rel, chk.stdout, chk.stderr))
                continue
            n_err, n_warn = int(m.group(1)), int(m.group(2))
            if n_err or n_warn:
                errors.append("%s: rnl check finds %d errors and %d warnings:\n%s"
                              % (rel, n_err, n_warn, "\n".join(
                                  l for l in chk.stdout.splitlines() if l.startswith(("error", "warning")))))
            out_dir = Path(tmp) / str(idx)
            rnd = subprocess.run([str(rnl), "render", str(path), "--out", str(out_dir)],
                                 capture_output=True, text=True, encoding="utf-8")
            if rnd.returncode != 0 or not list(out_dir.glob("*.svg")):
                errors.append("%s: rnl render failed:\n%s%s" % (rel, rnd.stdout, rnd.stderr))
    return errors


def page_scripts(files):
    """{page: the paths of the scripts it loads}, and {script: its page} for each script only one page loads."""
    loads = {page: ["assets/" + name for name in SCRIPT_TAG.findall(files[page]) if "assets/" + name in files]
             for page in PAGES}
    users = {}
    for page, scripts in loads.items():
        for script in scripts:
            users.setdefault(script, []).append(page)
    return loads, {script: pages[0] for script, pages in users.items() if len(pages) == 1}


def coverage(files, code):
    """{page: (translated, all)} of the text groups a page shows in one language, its scripts' with its own."""
    loads, _ = page_scripts(files)
    found = {}
    for page, scripts in loads.items():
        counts = [I18N.coverage(files[path], code, path) for path in [page, *scripts]]
        found[page] = (sum(c[0] for c in counts), sum(c[1] for c in counts))
    return found


def build_language(files, index, version, linux=False, macos=False):
    """Return {relative path: text} for one language, including the standalone pages. Both kinds
    of release block are checked for Chinese before the ones that do not apply are dropped. A page
    less translated than site_i18n.SHOWN_IN_ENGLISH_BELOW is shown in English, with the scripts only
    it loads, and every page not wholly translated says so (decision 0015)."""
    code, prefix = LANGS[index]
    counts = coverage(files, code)
    english = {page for page, (done, total) in counts.items() if done < SHOWN_IN_ENGLISH_BELOW * total}
    _, own = page_scripts(files)
    out = {path: I18N.localize(text, code, path, english=(path if path in PAGES else own.get(path)) in english)
           for path, text in files.items()}
    out = {path: text.replace("{{locale-css}}", I18N.css(code)) for path, text in out.items()}
    for page in PAGES:
        tokens = {"html-lang": I18N.locales[code]["html_lang"], "locale-code": code, "font-href": escape(I18N.font_url(code), quote=True),
                  "language-menu": I18N.menu(code, lambda target: "../" + target + "/" + ("" if page == "index.html" else page)),
                  "spec-root": "spec/" if I18N.standard(code) == code else "../" + I18N.standard(code) + "/spec/",
                  "translation-notice": I18N.notice(code, *counts[page])}
        tokens.update({"nav-" + key: escape(I18N.message(code, key)) for key in ("intro", "tour", "spec", "packages", "gallery", "tools", "agents", "pages")})
        for key, value in tokens.items():
            out[page] = out[page].replace("{{" + key + "}}", value)
        if I18N.standard(code) != code:
            out[page] = re.sub(r'(\shref=")spec/', r'\1../' + I18N.standard(code) + '/spec/', out[page])
    errors = []
    if code == I18N.default:
        for path, text in out.items():
            errors += check_leaks(text, "%s (%s)" % (path, code))
    if errors:
        raise BuildError("\n".join(errors))
    out = {path: release(text, version, linux, macos) for path, text in out.items()}
    for page, single in PAGES.items():
        out[page] = out[page].replace("<!doctype html>\n", "<!doctype html>\n" + NOTICE, 1)
        if single:
            out[single] = standalone(page, out, SITE_URL + prefix)
    return out


def standalone(page, out, page_url):
    """Inline the page's styles and scripts, and point relative links at the published site."""
    html = out[page]

    def inline(tag, m):
        name = "assets/" + m.group(1)
        if name not in out:
            raise BuildError("%s references %s, which does not exist" % (page, name))
        body = out[name]
        if tag == "script" and "</script" in body.lower():
            raise BuildError(name + " contains </script and cannot be inlined")
        return "<%s>\n%s\n</%s>" % (tag, body, tag)
    html = STYLE_TAG.sub(lambda m: inline("style", m), html)
    html = SCRIPT_TAG.sub(lambda m: inline("script", m), html)

    def inline_image(m):
        path = SRC / m.group(2)
        mime = "image/svg+xml" if path.suffix == ".svg" else "image/png"
        return m.group(1) + "data:" + mime + ";base64," + b64encode(path.read_bytes()).decode("ascii") + '"'
    html = re.sub(r'(\s(?:src|href)=")(assets/[^"<>]+\.(?:svg|png))"', inline_image, html)
    if 'src="assets/' in html or 'href="assets/' in html:
        raise BuildError("the standalone version of %s still references assets/" % page)

    def absolute(m):
        url = m.group(2)
        if re.match(r"(#|[a-z][a-z0-9+.-]*:|//)", url):
            return m.group(0)
        base = page_url
        if url.startswith("./"):
            url = url[2:]
        while url.startswith("../"):
            url = url[3:]
            base = base[:base.rstrip("/").rfind("/") + 1]
        return m.group(1) + base + url + '"'
    return re.sub(r'(\s(?:href|data-viewer-url)=")([^"]*)"', absolute, html)


def redirect(target, lang, text):
    """A page that leads to `target`, an address relative to the root of the site, where the page is,
    with the query and the section of its own address. GitHub Pages has no redirects of its own: the
    script leads there at once, and the refresh where scripts do not run."""
    return ('<!doctype html>\n<html lang="%s">\n<head>\n<meta charset="utf-8">\n<title>Rich Netlist (RNL)</title>\n'
            '<link rel="canonical" href="%s%s">\n<meta http-equiv="refresh" content="0; url=%s">\n'
            '<script>var to = "%s"; if (location.protocol === "file:" && /\\/$/.test(to)) to += "index.html";\n'
            'location.replace(to + location.search + location.hash);</script>\n</head>\n'
            '<body><p><a href="%s">%s</a></p></body>\n</html>\n') % (lang, SITE_URL, target, target, target, target, text)


def home():
    """Choose a saved locale, then the browser's supported preferences, then English."""
    choices = sorted([(match, code) for code in CODES for match in I18N.locales[code]["matches"]],
                     key=lambda item: len(item[0]), reverse=True)
    alternates = "\n".join(f'<link rel="alternate" hreflang="{I18N.locales[code]["html_lang"]}" href="{SITE_URL}{DIRS[code]}">'
                           for code in CODES)
    links = " · ".join(f'<a href="{DIRS[code]}" hreflang="{I18N.locales[code]["html_lang"]}" lang="{I18N.locales[code]["html_lang"]}">{escape(I18N.locales[code]["name"])}</a>'
                       for code in CODES)
    return f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"><title>Rich Netlist (RNL)</title>
{alternates}
<link rel="alternate" hreflang="x-default" href="{SITE_URL}{HOME}">
<script>
var codes = {json.dumps(CODES)}, choices = {json.dumps(choices)}, selected;
try {{ selected = localStorage.getItem('rnl-site-language'); }} catch (error) {{}}
if (codes.indexOf(selected) === -1) {{
  var preferences = navigator.languages || [navigator.language || ''];
  selected = {json.dumps(I18N.default)};
  outer: for (var i = 0; i < preferences.length; i++) {{
    var language = preferences[i].toLowerCase();
    for (var j = 0; j < choices.length; j++) {{
      var match = choices[j][0];
      if (language === match || language.indexOf(match + '-') === 0) {{ selected = choices[j][1]; break outer; }}
    }}
  }}
}}
var to = selected + '/';
if (location.protocol === 'file:') to += 'index.html';
location.replace(to + location.search + location.hash);
</script>
<noscript><meta http-equiv="refresh" content="0; url={HOME}"></noscript></head>
<body><p>{links}</p></body></html>
'''


def redirects():
    """The pages at the root of the site: index.html, which leads to the pages in the browser's language,
    and the old addresses of the Chinese pages, which lead to the same pages under zh/ whatever the
    browser's language."""
    out = {"index.html": home()}
    for page, single in PAGES.items():
        if page in NEW_PAGES:
            continue
        for name in (page, single):
            if name and name != "index.html":
                out[name] = redirect(MOVED + name, "en", "This page has moved to " + MOVED + name)
    return out


def build(version=None):
    """Build every page in every language, with the files of the desktop viewer's release `version`,
    or None for no release yet, and the pages at the root that lead to them."""
    files = sources()
    errors = [f"src/{path}: a translation written beside the English: translations go into i18n/<lang>/site/{path}.json; "
              f"python scripts/i18n/migrate_site.py moves them there" for path, text in files.items()
              if any(len(I18N.parts(m.group())[0]) > 1 for m in GROUP.finditer(text))]
    errors += [f"i18n/languages.json: the shared messages of {code} written in the registry: translations go into "
               f"i18n/{code}/site/messages.json" for code, entry in I18N.entries.items()
               if code != I18N.default and entry.get("messages")]
    if errors:
        raise BuildError("\n".join(errors))
    if files["index.html"].count(TIMELINE) != 1:
        raise BuildError("src/index.html needs exactly one {{timeline}} marker")
    files["index.html"] = files["index.html"].replace(TIMELINE, timeline_html())
    if files["timeline.html"].count(TIMELINE_FULL) != 1:
        raise BuildError("src/timeline.html needs exactly one {{timeline-full}} marker")
    files["timeline.html"] = files["timeline.html"].replace(TIMELINE_FULL, timeline_html(full=True))
    ex = examples()
    errors = []
    for path, text in files.items():
        errors += check_groups(text, "src/" + path) + check_release(text, "src/" + path)
        errors += [e for code in CODES for e in I18N.catalog_errors(code, path)]
    for page in PAGES:
        if files[page].count("{{translation-notice}}") != 1:
            errors.append("src/%s needs exactly one {{translation-notice}} marker, after its hero" % page)
    errors += check_examples(files, ex)
    if errors:
        raise BuildError("\n".join(errors))
    result = {}
    linux = has_linux_release(version)
    macos = has_macos_release(version)
    for index, (code, prefix) in enumerate(LANGS):
        for path, text in build_language(files, index, version, linux, macos).items():
            result[prefix + path] = text
        # Brand images are copied verbatim; only prose and scripts need localization.
        for path in sorted((SRC / "assets").rglob("*")):
            if path.suffix in (".svg", ".png"):
                result[prefix + path.relative_to(SRC).as_posix()] = path.read_bytes()
    for name, text in ex.items():
        result["examples/" + name] = text
    result.update(redirects())
    return result


def write(result, out):
    # Start the default output directory from empty; keep the directory itself, since a local
    # preview server may be serving it.
    if out == DEFAULT_OUT and out.exists():
        for child in out.iterdir():
            if child.is_dir():
                shutil.rmtree(child)
            else:
                child.unlink()
    for path, text in result.items():
        target = out / path
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(text, bytes):
            target.write_bytes(text)
        else:
            target.write_text(text, encoding="utf-8", newline="\n")


GUIDE = """RNL introduction pages

rnl-intro/ is the complete site: open rnl-intro/en/index.html, or the index.html in the folder of
another language, such as rnl-intro/zh/index.html, or upload the folder to any static host, where
its root leads to the pages in the browser's language.
.nojekyll makes GitHub Pages publish it as is.
language.html next to each index.html is the language tour, tools.html the page of the viewers, and
packages.html the gallery of the standard packages; the web viewer itself, the pages of the standard
and the gallery's data are published with the site, not packed here.
The *-standalone.html files in each language folder are single files with everything inlined.
Fonts load from Google Fonts; offline, system fonts are used and the pages still work.
"""


def pack(result, zip_path):
    """The site in a zip, without the old addresses of the Chinese pages, which only the published site has."""
    old = set(redirects()) - {"index.html"}
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for path, text in sorted(result.items()):
            if path in old:
                continue
            zf.writestr("rnl-intro/" + path, text)
        zf.writestr("rnl-intro/.nojekyll", "")
        zf.writestr("README.txt", GUIDE)


def main():
    ap = argparse.ArgumentParser(description="Build the pages of the RNL site in every language.")
    ap.add_argument("--check", action="store_true", help="check the sources and write nothing")
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT, help="output directory (default: site/dist)")
    ap.add_argument("--zip", type=Path, help="also pack the built site into this zip file")
    ap.add_argument("--viewer-version", metavar="VERSION",
                    help="link the files of this release of the desktop viewer, such as 0.1.0, instead of the "
                         "newest tag viewer-vX.Y.Z; none for no release yet")
    ap.add_argument("--rnl", metavar="PATH",
                    help="also check and render site/examples/*.sp with this rnl command")
    args = ap.parse_args()
    for stream in (sys.stdout, sys.stderr):
        stream.reconfigure(errors="backslashreplace")
    if args.viewer_version is None:
        version = newest_viewer_release()
    elif args.viewer_version == "none":
        version = None
    elif VERSION.fullmatch(args.viewer_version):
        version = args.viewer_version
    else:
        ap.error("--viewer-version takes a version such as 0.1.0, or none")
    try:
        result = build(version)
        if args.rnl is not None:
            rnl = find_rnl(args.rnl)
            rnl_errors = check_examples_rnl(rnl)
            if rnl_errors:
                raise BuildError("\n".join(rnl_errors))
    except BuildError as e:
        print(e, file=sys.stderr)
        print("build failed", file=sys.stderr)
        return 1
    groups = len(GROUP.findall("\n".join(sources().values())))
    if args.check:
        msg = "ok: %d text groups, each in %s, and %d example files" % (groups, " and ".join(CODES), len(examples()))
        if args.rnl is not None:
            msg += " (checked and rendered with rnl)"
        print(msg)
        return 0
    out = args.out.resolve()
    write(result, out)
    print("built %d files into %s (%d text groups)" % (len(result), out, groups))
    for code, prefix in LANGS:
        for page in PAGES:
            print("  %s: %s" % (code, out / prefix / page))
    print("  the root uses a saved language or supported browser preferences, falling back to %s; "
          "legacy root addresses lead to %s" % (HOME, MOVED))
    if version:
        print("the page of the tools links to the desktop viewer %s (tag viewer-v%s)" % (version, version))
    else:
        print("the page of the tools says the desktop viewer's first release is coming (no tag viewer-vX.Y.Z)")
    if args.zip:
        pack(result, args.zip)
        print("packed %s" % args.zip.resolve())
    return 0


if __name__ == "__main__":
    sys.exit(main())
