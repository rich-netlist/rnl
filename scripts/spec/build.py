"""Build every published form of the RNL standard from its chapters, and check that they agree.

    python scripts/spec/build.py                 check, then build every version into scripts/spec/dist/
    python scripts/spec/build.py --check         the checks only
    python scripts/spec/build.py --version 0.3   only this version, as for the release of a tag spec-0.3-rN
    python scripts/spec/build.py --site DIR      also put the web pages and their downloads into a built site
    python scripts/spec/build.py --no-pdf        skip the PDFs
    python scripts/spec/build.py --stamp         stamp the Chinese parts, once they are in line with the English

Input, the spec/ folder of the repository (or --spec DIR), with a folder for each version V, such as 0.2:
    V/en/, V/zh/            front.md, ch01.md, ch02.md, …, annex-a.md, annex-b.md, …: the source
    rnl-V.md, rnl-V.zh.md   the combined texts, which must be what the chapters give
    rnl-words-V.json        the word list of the version
    rnl-words.schema.json   the schema of the word lists, shared by all versions

Output, for each version, in English and Chinese, all from the same chapters and with the same revision
label: the combined Markdown, the chapters as a zip, one web page, a PDF, and SHA256SUMS. With --site, the
Chinese page of version V goes to DIR/spec/V/ and the English one to DIR/en/spec/V/, each with its
downloads; DIR/spec/ and DIR/en/spec/ lead to the newest version, and each page links to the others.

Before building, it checks for each version that
  1. the Chinese chapters have the sections, list items, table rows and code of the English ones;
  2. each Chinese part is stamped with the hash of the English part it was translated from, and that the
     hash is current;
  3. Annex A in both languages is what the word list gives, and so is the JSON excerpt of 5.3;
  4. every section number cited in the text exists;
  5. the combined texts are what the chapters give.
pandoc and typst must finish without warnings. Any failure stops the build (exit status 1).

Each version has its own revision label, from git (decision 0007): "revision n" at the tag spec-V-rn,
"revision n + later changes" once the chapters or the word list of V have changed since that tag, and
"not released" before its first tag. Commits that change other files do not count.

Needs pandoc 3.x and typst 0.13 or later on PATH, and git for the revision labels.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import spectools  # noqa: E402

READER = "commonmark_x-smart-tex_math_dollars-subscript-superscript-emoji-implicit_header_references"
FONTS = {  # preferences; only the families typst finds are passed on
    "serif": ["Libertinus Serif", "Noto Serif CJK SC", "Source Han Serif SC", "SimSun"],
    "sans": ["Segoe UI", "Noto Sans", "DejaVu Sans", "Noto Sans CJK SC", "Source Han Sans SC", "Microsoft YaHei"],
    "mono": ["DejaVu Sans Mono", "Noto Sans Mono CJK SC", "Noto Sans CJK SC", "Microsoft YaHei"],
}
SITE_URL = "https://rich-netlist.github.io/rnl/"  # the project site; the downloaded page links to it
FONTS_CSS = ("https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600"
             "&family=IBM+Plex+Sans+Condensed:wght@500;600;700&family={body}&display=swap")
LANGS = {
    "en": {"suffix": "", "html_lang": "en", "other": "zh", "other_label": "中文", "contents": "Contents",
           "zip_label": "Markdown, chapters", "md_label": "Markdown", "sums_label": "SHA-256 checksums",
           "intro": "Introduction", "tour": "Language tour", "spec": "Standard", "pages": "Pages",
           "kicker": "Rich Netlist language standard", "theme_dark": "Switch to dark theme",
           "theme_light": "Switch to light theme", "site_home": "en/",
           "body_font": "IBM+Plex+Sans:wght@400;500;600;700",
           "foot": "The text of the standard is licensed under CC-BY-4.0. The English text is normative.",
           "files": "Files", "file": "File", "content": "Content", "versions": "Versions: ",
           "label": "Version {v} · {state} ({date}, {commit})", "released": "revision {n}",
           "after": "revision {n} + later changes", "draft": "not released", "dirty": " + uncommitted changes",
           "site_dir": "en/spec/{v}", "site_other": "../../../spec/{v}/"},
    "zh": {"suffix": ".zh", "html_lang": "zh-CN", "other": "en", "other_label": "EN", "contents": "目录",
           "zip_label": "Markdown 分章包", "md_label": "Markdown 合订本", "sums_label": "SHA-256 校验值",
           "intro": "介绍", "tour": "语言导览", "spec": "标准", "pages": "页面",
           "kicker": "Rich Netlist 语言标准", "theme_dark": "切换到深色", "theme_light": "切换到浅色", "site_home": "",
           "body_font": "Noto+Sans+SC:wght@400;500;700;900",
           "foot": "标准文本按 CC-BY-4.0 许可发布。本文是中文译本，以英文版为准。",
           "files": "文件", "file": "文件", "content": "内容", "versions": "版本：",
           "label": "{v} 版 · {state}（{date}，{commit}）", "released": "修订 {n}",
           "after": "修订 {n} 之后的修改", "draft": "未发布", "dirty": " + 未提交的修改",
           "site_dir": "spec/{v}", "site_other": "../../en/spec/{v}/"},
}
VERSION_DIR = re.compile(r"(0|[1-9]\d*)\.(0|[1-9]\d*)")  # a version folder of spec/, such as 0.2 (16.1)
STAMP = re.compile(r"^<!-- translated from en/(\S+) sha256:([0-9a-f]{12}) -->\n")
FENCE = re.compile(r"^([ \t]*)```([\w-]+)((?:[ \t]+\S+)+)[ \t]*$", re.M)
HEADING_ID = re.compile(r"^#{2,4} (?:Annex |附录 )?(\d+(?:\.\d+)*|[A-Z](?:\.\d+)?)\s", re.M)


class Failed(Exception):
    pass


def read(p: Path) -> str:
    return p.read_text(encoding="utf-8")


def versions(spec: Path) -> list[str]:
    """The versions in spec/, oldest first: the folders named major.minor that hold en/ and zh/."""
    found = [d.name for d in spec.iterdir()
             if d.is_dir() and VERSION_DIR.fullmatch(d.name) and (d / "en").is_dir() and (d / "zh").is_dir()]
    return sorted(found, key=lambda v: tuple(int(x) for x in v.split(".")))


def parts(spec: Path, v: str, lang: str = "en") -> list[str]:
    return spectools.parts([p.name for p in (spec / v / lang).glob("*.md")])


def translated(spec: Path, v: str) -> list[str]:
    return ["front.md"] + [p for p in parts(spec, v) if p != "annex-a.md"]  # Annex A is generated in both


def name(v: str, lang: str, ext: str) -> str:
    return f"rnl-{v}{LANGS[lang]['suffix']}{ext}"


def own_files(out: Path, v: str) -> list[Path]:
    """The files of version v in a folder of downloads: rnl-0.2.md, rnl-0.2-chapters.zip, … (not rnl-0.20…)."""
    return sorted({*out.glob(f"rnl-{v}.*"), *out.glob(f"rnl-{v}-*")})


def git(cwd: Path, *args: str) -> str | None:
    r = subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def revision(spec: Path, v: str) -> dict:
    """The revision of version v: the newest tag spec-v-rN reachable from HEAD, and how many commits changed
    the sources of v (its chapters and its word list) after it. The label names the tagged commit while
    nothing has changed, and otherwise the last commit that changed the sources."""
    fmt = "--format=%h %cs %ct"
    if git(spec, "rev-parse", "--show-toplevel") is None:  # not in a repository, such as a staged copy
        return {"n": None, "after": 0, "dirty": False, "commit": "local", "date": time.strftime("%Y-%m-%d"),
                "epoch": 0}
    sources = [str(spec / v), str(spec / f"rnl-words-{v}.json")]
    tag = git(spec, "describe", "--tags", "--abbrev=0", "--match", f"spec-{v}-r*") or ""
    m = re.fullmatch(rf"spec-{re.escape(v)}-r(\d+)", tag)
    after = int(git(spec, "rev-list", "--count", f"{tag}..HEAD", "--", *sources) or 0) if m else 0
    if m and not after:
        line = git(spec, "log", "-1", fmt, tag)
    else:
        line = git(spec, "log", "-1", fmt, "--", *sources) or git(spec, "log", "-1", fmt)
    commit, date, epoch = (line or f"local {time.strftime('%Y-%m-%d')} 0").split()
    dirty = bool(git(spec, "status", "--porcelain", "--", *sources))
    return {"n": int(m.group(1)) if m else None, "after": after, "dirty": dirty,
            "commit": commit, "date": date, "epoch": int(epoch)}


def revision_label(lang: str, v: str, rev: dict) -> str:
    """At a tag "revision 2", after it "revision 2 + later changes", before the first tag "not released"."""
    L = LANGS[lang]
    if rev["n"] is None:
        state = L["draft"]
    else:
        state = L["released" if rev["after"] == 0 and not rev["dirty"] else "after"].format(n=rev["n"])
    label = L["label"].format(v=v, state=state, date=rev["date"], commit=rev["commit"])
    return label + (L["dirty"] if rev["dirty"] else "")


def en_hash(text: str) -> str:
    return hashlib.sha256(text.replace("\r\n", "\n").encode("utf-8")).hexdigest()[:12]


# --- checks, each for one version -------------------------------------------------------------------

def check_structure(spec: Path, v: str) -> list[str]:
    en, zh = parts(spec, v), parts(spec, v, "zh")
    if en != zh:
        return [f"{v}: the parts differ: en/ has {', '.join(en)}; zh/ has {', '.join(zh)}"]
    msgs = []
    for part in spectools.compared(en):
        msgs += [f"{v} {m}" for m in spectools.compare(read(spec / v / "en" / part), read(spec / v / "zh" / part),
                                                        part.removesuffix(".md"))]
    return msgs


def check_stamps(spec: Path, v: str) -> list[str]:
    msgs = []
    for part in translated(spec, v):
        m = STAMP.match(read(spec / v / "zh" / part))
        want = en_hash(read(spec / v / "en" / part))
        if not m:
            msgs.append(f"{v}/zh/{part}: no stamp of the English part it was translated from")
        elif m.group(1) != part or m.group(2) != want:
            msgs.append(f"{v}/zh/{part}: translated from en/{part} {m.group(2)}, the English is now {want}: "
                        f"bring the Chinese in line, then run --stamp")
    return msgs


def check_word_list(spec: Path, v: str) -> list[str]:
    words = json.loads(read(spec / f"rnl-words-{v}.json"))
    msgs = [] if words.get("version") == v else [f"rnl-words-{v}.json: its version is {words.get('version')}"]
    for lang in LANGS:
        if read(spec / v / lang / "annex-a.md") != spectools.annex_a(words, lang):
            msgs.append(f"{v}/{lang}/annex-a.md: not what the word list gives")
        ch05 = read(spec / v / lang / "ch05.md")
        m = re.search(r"^([ \t]*)```json$", ch05, re.M)
        want = "\n".join(m.group(1) + x if x else x for x in spectools.excerpt(words).split("\n")) if m else None
        if not m or want not in ch05:
            msgs.append(f"{v}/{lang}/ch05.md: the JSON excerpt of 5.3 is not the PLACE entry of the word list")
    return msgs


def prose(text: str) -> str:
    """The text without code blocks and inline code."""
    text = re.sub(r"^[ \t]*```.*?^[ \t]*```[ \t]*$", "", text, flags=re.M | re.S)
    return re.sub(r"`[^`\n]+`", "", text)


REF = re.compile(r"(?<![\w.])(\d{1,2}(?:\.\d{1,2}){1,3})(?![\w])")
# Numbers that name principles ("Principles 2.4, 2.9") or versions; the same rule as rnl.lua.
NOT_A_SECTION = re.compile(r"(?:(?:Principles|原则)\s*(?:\d+(?:\.\d+)*\s*(?:,|、|and|or|和|或)\s*)*"
                           r"|(?:[Vv]ersions?|版本|before|RNL|0\.x)\s*)$")


def check_refs(spec: Path, v: str) -> list[str]:
    msgs = []
    names = parts(spec, v)
    last = max(int(p[2:-3]) for p in names if p.startswith("ch"))  # numbers beyond the chapters are not sections
    for lang in LANGS:
        texts = {p: read(spec / v / lang / p) for p in ["front.md"] + names}
        ids = set(HEADING_ID.findall("\n".join(texts.values())))
        for part, text in texts.items():
            body = prose(text)
            for m in REF.finditer(body):
                num = m.group(1).rstrip(".")
                if num in ids or num.split(".")[0] == "0" or int(num.split(".")[0]) > last:
                    continue
                if NOT_A_SECTION.search(body[max(0, m.start() - 60):m.start()]):
                    continue
                msgs.append(f"{v}/{lang}/{part}: cites {num}, which is not a section")
    return msgs


def combined(spec: Path, v: str, lang: str) -> str:
    src = spec / v / lang
    return spectools.assemble(read(src / "front.md"), [read(src / p) for p in parts(spec, v)], lang)


def check_combined(spec: Path, v: str) -> list[str]:
    return [f"{name(v, lang, '.md')}: not what the chapters give" for lang in LANGS
            if read(spec / name(v, lang, ".md")) != combined(spec, v, lang)]


CHECKS = (("structure", check_structure), ("translation stamps", check_stamps), ("word list", check_word_list),
          ("references", check_refs), ("combined texts", check_combined))


# --- builds ----------------------------------------------------------------------------------------

def fence_attrs(m: re.Match) -> str:
    """```rnl error id=a  ->  ```{.rnl .error data-id="a"}: pandoc keeps only the first word otherwise."""
    indent, lang, rest = m.group(1), m.group(2), m.group(3).split()
    classes = [lang] + [w for w in rest if "=" not in w]
    attrs = [f'data-{k}="{v}"' for k, v in (w.split("=", 1) for w in rest if "=" in w)]
    return indent + "```{" + " ".join([f".{c}" for c in classes] + attrs) + "}"


def pandoc_input(spec: Path, v: str, lang: str, work: Path) -> Path:
    # The translation stamps are for the checks; before the title they would also keep it from being one.
    texts = [STAMP.sub("", read(spec / v / lang / p)).strip() + "\n" for p in ["front.md"] + parts(spec, v)]
    p = work / f"{v}-{lang}.md"
    p.write_text(FENCE.sub(fence_attrs, "\n".join(texts)), encoding="utf-8", newline="\n")
    return p


def run(cmd: list[str], what: str, env: dict | None = None) -> None:
    r = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", env=env)
    noise = (r.stderr or "").strip()
    if r.returncode != 0 or noise:
        raise Failed(f"{what}:\n{noise or r.stdout}")


def typst_fonts() -> dict:
    listed = subprocess.run(["typst", "fonts"], capture_output=True, text=True, encoding="utf-8").stdout
    have = {line.strip().lower() for line in listed.splitlines()}
    return {k: [f for f in v if f.lower() in have] for k, v in FONTS.items()}


def common_args(lang: str, v: str, rev: dict) -> list[str]:
    return ["-f", READER, "--shift-heading-level-by=-1", "--lua-filter", str(HERE / "rnl.lua"),
            "--syntax-highlighting=none", "--fail-if-warnings", "--wrap=none",  # values go into scripts intact
            "-M", f"rnl-lang={lang}", "-M", f"typst-lang={lang}", "-M", f"revision-label={revision_label(lang, v, rev)}",
            "-M", f"revision={rev['commit']}", "-M", f"contents-title={LANGS[lang]['contents']}"]


def size(p: Path) -> str:
    if not p.exists():
        return ""
    n = p.stat().st_size
    return f"{n / 1048576:.1f} MB" if n >= 1048576 else f"{max(1, n // 1024)} KB"


def version_links(lang: str, v: str, all_versions: list[str], href) -> str:
    """The versions, newest first, for the head of the page of v; nothing while there is only one."""
    if len(all_versions) < 2:
        return ""
    items = [f"<b>{x}</b>" if x == v else f'<a href="{href(x)}">{x}</a>' for x in reversed(all_versions)]
    return LANGS[lang]["versions"] + " · ".join(items)


def build_html(src: Path, lang: str, v: str, rev: dict, dest: Path, other_href: str, site_root: str,
               links: str, out: Path, work: Path) -> None:
    """The web page. Its downloads are the files of the same build, next to it; site_root is where the
    links of the top bar lead: the site's pages for this language. links is the line of versions."""
    L, other = LANGS[lang], LANGS[lang]["other"]
    log = work / f"links-{v}-{lang}.tsv"
    log.unlink(missing_ok=True)
    meta = {"html-lang": L["html_lang"], "pagetitle": f"RNL {v} · {L['kicker']}",
            "other-lang": LANGS[other]["html_lang"], "other-label": L["other_label"], "other-href": other_href,
            "pdf-href": name(v, lang, ".pdf"), "md-href": name(v, lang, ".md"),
            "zip-href": name(v, lang, "-chapters.zip"), "pdf-size": size(out / name(v, lang, ".pdf")),
            "md-size": size(out / name(v, lang, ".md")), "zip-size": size(out / name(v, lang, "-chapters.zip")),
            "md-label": L["md_label"], "zip-label": L["zip_label"], "sums-label": L["sums_label"],
            "site-root": site_root, "intro-label": L["intro"], "tour-label": L["tour"], "spec-label": L["spec"],
            "pages-label": L["pages"], "kicker": L["kicker"], "theme-label": L["theme_dark"],
            "theme-dark": L["theme_dark"], "theme-light": L["theme_light"], "foot-note": L["foot"],
            "font-href": FONTS_CSS.format(body=L["body_font"])}
    args = [a for k, val in meta.items() for a in ("-M", f"{k}={val}")]
    if links:  # HTML, so a variable (not escaped) rather than metadata
        args += ["-V", f"versions-html={links}"]
    run(["pandoc", str(src), *common_args(lang, v, rev), "-t", "html5", "--standalone",
         "--template", str(HERE / "page.html"), "--toc", "--toc-depth=2", "--section-divs", *args,
         "-o", str(dest)], f"pandoc html ({v}, {lang})", dict(os.environ, RNL_LINKS_LOG=str(log)))


def build_pdf(src: Path, lang: str, v: str, rev: dict, out: Path, work: Path, fonts: dict) -> None:
    typ = work / f"{v}-{lang}.typ"
    font_args = [a for k, fams in fonts.items() for f in fams for a in ("-M", f"{k}={f}")]
    run(["pandoc", str(src), *common_args(lang, v, rev), "-t", "typst", "--standalone",
         "--template", str(HERE / "print.typ"), "-M", f"short-title=RNL {v}", *font_args,
         "-o", str(typ)], f"pandoc typst ({v}, {lang})")
    run(["typst", "compile", "--root", str(work), str(typ), str(out / name(v, lang, ".pdf"))], f"typst ({v}, {lang})")


def build_zip(spec: Path, lang: str, v: str, rev: dict, out: Path) -> None:
    """The chapters as files, with a README that lists them and a MANIFEST of hashes; reproducible."""
    L = LANGS[lang]
    src = spec / v / lang
    files = {p: read(src / p) for p in parts(spec, v)}
    rows = []
    for p, text in files.items():
        m = re.search(r"^## (.+)$", text, re.M)
        rows.append(f"| [{p}]({p}) | {m.group(1) if m else ''} |")
    for f in (f"rnl-words-{v}.json", "rnl-words.schema.json"):
        files[f] = read(spec / f)
        rows.append(f"| [{f}]({f}) | |")
    label = revision_label(lang, v, rev)
    readme = (STAMP.sub("", read(src / "front.md")).strip() + f"\n\n{label}\n\n## {L['files']}\n\n"
              f"| {L['file']} | {L['content']} |\n|---|---|\n" + "\n".join(rows) + "\n")
    files = {"README.md": readme, **files}
    manifest = [f"RNL {v} ({lang}), {label}", ""]
    manifest += [f"{hashlib.sha256(t.encode('utf-8')).hexdigest()}  {f}" for f, t in files.items()]
    files["MANIFEST.txt"] = "\n".join(manifest) + "\n"
    stamp = time.gmtime(rev["epoch"])[:6] if rev["epoch"] else (1980, 1, 1, 0, 0, 0)
    with zipfile.ZipFile(out / name(v, lang, "-chapters.zip"), "w", zipfile.ZIP_DEFLATED) as z:
        for f, t in files.items():
            info = zipfile.ZipInfo(f"{name(v, lang, '')}/{f}", date_time=stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, t.encode("utf-8"))


def write_sums(d: Path) -> None:
    lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}"
             for p in sorted(d.iterdir()) if p.is_file() and p.name != "SHA256SUMS"]
    (d / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def write_redirect(d: Path, lang: str, newest: str) -> None:
    """d/index.html, which leads to the page of the newest version and keeps the section of the address."""
    L = LANGS[lang]
    html = (f'<!doctype html>\n<html lang="{L["html_lang"]}">\n<head>\n<meta charset="utf-8">\n'
            f'<title>RNL {newest} · {L["kicker"]}</title>\n<link rel="canonical" href="{newest}/">\n'
            f'<meta http-equiv="refresh" content="0; url={newest}/">\n'
            f'<script>location.replace("{newest}/" + location.hash)</script>\n</head>\n'
            f'<body><p><a href="{newest}/">RNL {newest}</a></p></body>\n</html>\n')
    d.mkdir(parents=True, exist_ok=True)
    (d / "index.html").write_text(html, encoding="utf-8", newline="\n")


# --- main ----------------------------------------------------------------------------------------

def stamp(spec: Path, v: str) -> None:
    for part in translated(spec, v):
        p = spec / v / "zh" / part
        text = STAMP.sub("", read(p))
        line = f"<!-- translated from en/{part} sha256:{en_hash(read(spec / v / 'en' / part))} -->\n"
        p.write_text(line + text, encoding="utf-8", newline="\n")
    for lang in LANGS:  # the combined text holds the stamps too
        (spec / name(v, lang, ".md")).write_text(combined(spec, v, lang), encoding="utf-8", newline="\n")
    print(f"{v}: stamped {len(translated(spec, v))} Chinese parts and rewrote the combined texts")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--spec", type=Path, default=HERE.parents[1] / "spec", help="the spec/ folder (default: the repository's)")
    ap.add_argument("--version", help="only this version, such as 0.2 (default: every version in spec/)")
    ap.add_argument("--out", type=Path, default=HERE / "dist", help="where the downloads go (default: scripts/spec/dist/)")
    ap.add_argument("--site", type=Path, help="a built site to add the web pages and their downloads to")
    ap.add_argument("--check", action="store_true", help="run the checks only")
    ap.add_argument("--no-pdf", action="store_true", help="skip the PDFs")
    ap.add_argument("--stamp", action="store_true", help="stamp the Chinese parts with the hashes of the English ones")
    args = ap.parse_args(argv)
    spec = args.spec.resolve()
    all_versions = versions(spec)
    if not all_versions:
        print(f"{spec}: no version folders, such as 0.2/ with en/ and zh/")
        return 1
    if args.version and args.version not in all_versions:
        print(f"{spec}: no version {args.version}; there are {', '.join(all_versions)}")
        return 1
    chosen = [args.version] if args.version else all_versions
    if args.stamp:
        for v in chosen:
            stamp(spec, v)
        return 0

    failures = []
    for what, check in CHECKS:
        msgs = [m for v in chosen for m in check(spec, v)]
        print(f"check {what}: {'ok' if not msgs else f'{len(msgs)} problem(s)'}")
        failures += msgs
    if failures:
        print("\n".join(failures))
        return 1
    if args.check:
        return 0

    out, work = args.out, args.out / "work"
    work.mkdir(parents=True, exist_ok=True)
    fonts = typst_fonts()
    try:
        for v in chosen:
            for f in own_files(out, v):
                f.unlink()
            rev = revision(spec, v)
            for lang in LANGS:
                L = LANGS[lang]
                shutil.copyfile(spec / name(v, lang, ".md"), out / name(v, lang, ".md"))
                src = pandoc_input(spec, v, lang, work)
                build_zip(spec, lang, v, rev, out)
                if not args.no_pdf:
                    build_pdf(src, lang, v, rev, out, work, fonts)
                published = version_links(lang, v, all_versions, lambda x: SITE_URL + L["site_dir"].format(v=x) + "/")
                build_html(src, lang, v, rev, out / name(v, lang, ".html"), name(v, L["other"], ".html"),
                           SITE_URL + L["site_home"], published, out, work)
                if args.site:
                    d = args.site / L["site_dir"].format(v=v)
                    d.mkdir(parents=True, exist_ok=True)
                    for ext in (".md", "-chapters.zip") + (() if args.no_pdf else (".pdf",)):
                        shutil.copyfile(out / name(v, lang, ext), d / name(v, lang, ext))
                    build_html(src, lang, v, rev, d / "index.html", L["site_other"].format(v=v), "../../",
                               version_links(lang, v, all_versions, lambda x: f"../{x}/"), out, work)
                    write_sums(d)
                print(f"built {v} {lang}: {revision_label(lang, v, rev)}")
        if args.site:
            for lang in LANGS:
                write_redirect(args.site / Path(LANGS[lang]["site_dir"].format(v="")), lang, all_versions[-1])
    except Failed as e:
        print(e)
        return 1
    write_sums(out)
    for v in chosen:
        for p in own_files(out, v):
            print(f"  {p.name:28} {p.stat().st_size // 1024:6} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
