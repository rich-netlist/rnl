"""Build every published form of the RNL standard from its chapters, and check that they agree.

    python scripts/spec/build.py               check, then build into scripts/spec/dist/
    python scripts/spec/build.py --check       the checks only
    python scripts/spec/build.py --site DIR    also put the web pages and their downloads into a built site
    python scripts/spec/build.py --no-pdf      skip the PDFs
    python scripts/spec/build.py --stamp       stamp the Chinese parts, once they are in line with the English

Input, the spec/ folder of the repository (or --spec DIR):
    0.2/en/, 0.2/zh/        front.md, ch01.md ... ch17.md, annex-a.md ... annex-c.md: the source
    rnl-0.2.md, rnl-0.2.zh.md   the combined texts, which must be what the chapters give
    rnl-words-0.2.json, rnl-words.schema.json

Output, for English and Chinese, all from the same chapters and with the same revision label:
the combined Markdown, the chapters as a zip, one web page, a PDF, and SHA256SUMS. With --site, the
Chinese page goes to DIR/spec/0.2/ and the English one to DIR/en/spec/0.2/, each with its downloads.

Before building, it checks that
  1. the Chinese chapters have the sections, list items, table rows and code of the English ones;
  2. each Chinese part is stamped with the hash of the English part it was translated from, and that the
     hash is current;
  3. Annex A in both languages is what the word list gives, and so is the JSON excerpt of 5.3;
  4. every section number cited in the text exists;
  5. the combined texts are what the chapters give.
pandoc and typst must finish without warnings. Any failure stops the build (exit status 1).

Needs pandoc 3.x and typst 0.13 or later on PATH, and git for the revision label (decision 0007).
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

VERSION = "0.2"
PARTS = spectools.PARTS
TRANSLATED = ["front.md"] + [p for p in PARTS if p != "annex-a.md"]  # Annex A is generated in both
READER = "commonmark_x-smart-tex_math_dollars-subscript-superscript-emoji-implicit_header_references"
FONTS = {  # preferences; only the families typst finds are passed on
    "serif": ["Libertinus Serif", "Noto Serif CJK SC", "Source Han Serif SC", "SimSun"],
    "sans": ["Segoe UI", "Noto Sans", "DejaVu Sans", "Noto Sans CJK SC", "Source Han Sans SC", "Microsoft YaHei"],
    "mono": ["DejaVu Sans Mono", "Noto Sans Mono CJK SC", "Noto Sans CJK SC", "Microsoft YaHei"],
}
LANGS = {
    "en": {"suffix": "", "html_lang": "en", "other": "zh", "other_label": "中文", "contents": "Contents",
           "zip_label": "Chapters (zip)", "files": "Files", "file": "File", "content": "Content",
           "label": "Version {v} · {state} ({date}, {commit})", "released": "revision {n}",
           "after": "revision {n} + later changes", "draft": "not released", "dirty": " + uncommitted changes",
           "site_dir": "en/spec/{v}", "site_other": "../../../spec/{v}/"},
    "zh": {"suffix": ".zh", "html_lang": "zh-CN", "other": "en", "other_label": "English", "contents": "目录",
           "zip_label": "分章包", "files": "文件", "file": "文件", "content": "内容",
           "label": "{v} 版 · {state}（{date}，{commit}）", "released": "修订 {n}",
           "after": "修订 {n} 之后的修改", "draft": "未发布", "dirty": " + 未提交的修改",
           "site_dir": "spec/{v}", "site_other": "../../en/spec/{v}/"},
}
TAG = f"spec-{VERSION}-r"  # a revision of the standard is published by the tag spec-0.2-r1, spec-0.2-r2, …
STAMP = re.compile(r"^<!-- translated from en/(\S+) sha256:([0-9a-f]{12}) -->\n")
FENCE = re.compile(r"^([ \t]*)```([\w-]+)((?:[ \t]+\S+)+)[ \t]*$", re.M)
HEADING_ID = re.compile(r"^#{2,4} (?:Annex |附录 )?(\d+(?:\.\d+)*|[A-Z](?:\.\d+)?)\s", re.M)


class Failed(Exception):
    pass


def name(lang: str, ext: str) -> str:
    return f"rnl-{VERSION}{LANGS[lang]['suffix']}{ext}"


def read(p: Path) -> str:
    return p.read_text(encoding="utf-8")


def git(*args: str) -> str:
    return subprocess.run(["git", "-C", str(HERE), *args], capture_output=True, text=True).stdout.strip()


def revision(spec: Path) -> dict:
    dirty = bool(git("status", "--porcelain", "--untracked-files=no", "--", str(spec), str(HERE)))
    # The latest revision tag reachable from HEAD, and how many commits came after it: spec-0.2-r2-3-gabc1234
    m = re.match(rf"{re.escape(TAG)}(\d+)-(\d+)-g", git("describe", "--tags", "--long", "--match", f"{TAG}*"))
    return {"commit": git("rev-parse", "--short", "HEAD"), "date": git("log", "-1", "--format=%cs"),
            "epoch": int(git("log", "-1", "--format=%ct") or 0), "dirty": dirty,
            "n": int(m.group(1)) if m else None, "after": int(m.group(2)) if m else 0}


def revision_label(lang: str, rev: dict) -> str:
    """At a tag "revision 2", after it "revision 2 + later changes", before the first tag "not released"."""
    L = LANGS[lang]
    if rev["n"] is None:
        state = L["draft"]
    else:
        state = L["released" if rev["after"] == 0 and not rev["dirty"] else "after"].format(n=rev["n"])
    label = L["label"].format(v=VERSION, state=state, date=rev["date"], commit=rev["commit"])
    return label + (L["dirty"] if rev["dirty"] else "")


def en_hash(text: str) -> str:
    return hashlib.sha256(text.replace("\r\n", "\n").encode("utf-8")).hexdigest()[:12]


# --- checks ----------------------------------------------------------------------------------------

def check_structure(spec: Path) -> list[str]:
    msgs = []
    for part in spectools.COMPARED:
        msgs += spectools.compare(read(spec / VERSION / "en" / part), read(spec / VERSION / "zh" / part),
                                  part.removesuffix(".md"))
    return msgs


def check_stamps(spec: Path) -> list[str]:
    msgs = []
    for part in TRANSLATED:
        m = STAMP.match(read(spec / VERSION / "zh" / part))
        want = en_hash(read(spec / VERSION / "en" / part))
        if not m:
            msgs.append(f"zh/{part}: no stamp of the English part it was translated from")
        elif m.group(1) != part or m.group(2) != want:
            msgs.append(f"zh/{part}: translated from en/{part} {m.group(2)}, the English is now {want}: "
                        f"bring the Chinese in line, then run --stamp")
    return msgs


def check_word_list(spec: Path) -> list[str]:
    words = json.loads(read(spec / f"rnl-words-{VERSION}.json"))
    msgs = []
    for lang in LANGS:
        if read(spec / VERSION / lang / "annex-a.md") != spectools.annex_a(words, lang):
            msgs.append(f"{lang}/annex-a.md: not what the word list gives")
        ch05 = read(spec / VERSION / lang / "ch05.md")
        m = re.search(r"^([ \t]*)```json$", ch05, re.M)
        want = "\n".join(m.group(1) + x if x else x for x in spectools.excerpt(words).split("\n")) if m else None
        if not m or want not in ch05:
            msgs.append(f"{lang}/ch05.md: the JSON excerpt of 5.3 is not the PLACE entry of the word list")
    return msgs


def prose(text: str) -> str:
    """The text without code blocks and inline code."""
    text = re.sub(r"^[ \t]*```.*?^[ \t]*```[ \t]*$", "", text, flags=re.M | re.S)
    return re.sub(r"`[^`\n]+`", "", text)


REF = re.compile(r"(?<![\w.])(\d{1,2}(?:\.\d{1,2}){1,3})(?![\w])")
# Numbers that name principles ("Principles 2.4, 2.9") or versions; the same rule as rnl.lua.
NOT_A_SECTION = re.compile(r"(?:(?:Principles|原则)\s*(?:\d+(?:\.\d+)*\s*(?:,|、|and|or|和|或)\s*)*"
                           r"|(?:[Vv]ersions?|版本|before|RNL|0\.x)\s*)$")


def check_refs(spec: Path) -> list[str]:
    msgs = []
    for lang in LANGS:
        texts = {p: read(spec / VERSION / lang / p) for p in ["front.md"] + PARTS}
        ids = set(HEADING_ID.findall("\n".join(texts.values())))
        for part, text in texts.items():
            body = prose(text)
            for m in REF.finditer(body):
                num = m.group(1).rstrip(".")
                if num in ids or num.split(".")[0] == "0" or int(num.split(".")[0]) > 17:
                    continue
                if NOT_A_SECTION.search(body[max(0, m.start() - 60):m.start()]):
                    continue
                msgs.append(f"{lang}/{part}: cites {num}, which is not a section")
    return msgs


def combined(spec: Path, lang: str) -> str:
    src = spec / VERSION / lang
    return spectools.assemble(read(src / "front.md"), [read(src / p) for p in PARTS], lang)


def check_combined(spec: Path) -> list[str]:
    return [f"{name(lang, '.md')}: not what the chapters give" for lang in LANGS
            if read(spec / name(lang, ".md")) != combined(spec, lang)]


CHECKS = (("structure", check_structure), ("translation stamps", check_stamps), ("word list", check_word_list),
          ("references", check_refs), ("combined texts", check_combined))


# --- builds ----------------------------------------------------------------------------------------

def fence_attrs(m: re.Match) -> str:
    """```rnl error id=a  ->  ```{.rnl .error data-id="a"}: pandoc keeps only the first word otherwise."""
    indent, lang, rest = m.group(1), m.group(2), m.group(3).split()
    classes = [lang] + [w for w in rest if "=" not in w]
    attrs = [f'data-{k}="{v}"' for k, v in (w.split("=", 1) for w in rest if "=" in w)]
    return indent + "```{" + " ".join([f".{c}" for c in classes] + attrs) + "}"


def pandoc_input(spec: Path, lang: str, work: Path) -> Path:
    # The translation stamps are for the checks; before the title they would also keep it from being one.
    parts = [STAMP.sub("", read(spec / VERSION / lang / p)).strip() + "\n" for p in ["front.md"] + PARTS]
    p = work / f"{lang}.md"
    p.write_text(FENCE.sub(fence_attrs, "\n".join(parts)), encoding="utf-8", newline="\n")
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


def common_args(lang: str, rev: dict) -> list[str]:
    return ["-f", READER, "--shift-heading-level-by=-1", "--lua-filter", str(HERE / "rnl.lua"),
            "--syntax-highlighting=none", "--fail-if-warnings",
            "-M", f"rnl-lang={lang}", "-M", f"typst-lang={lang}", "-M", f"revision-label={revision_label(lang, rev)}",
            "-M", f"revision={rev['commit']}", "-M", f"contents-title={LANGS[lang]['contents']}"]


def build_html(src: Path, lang: str, rev: dict, dest: Path, other_href: str, work: Path) -> None:
    L, other = LANGS[lang], LANGS[lang]["other"]
    log = work / f"links-{lang}.tsv"
    log.unlink(missing_ok=True)
    run(["pandoc", str(src), *common_args(lang, rev), "-t", "html5", "--standalone",
         "--template", str(HERE / "page.html"), "--toc", "--toc-depth=2", "--section-divs",
         "-M", f"html-lang={L['html_lang']}", "-M", f"pagetitle=RNL {VERSION}",
         "-M", f"other-lang={LANGS[other]['html_lang']}", "-M", f"other-label={L['other_label']}",
         "-M", f"other-href={other_href}", "-M", f"pdf-href={name(lang, '.pdf')}",
         "-M", f"md-href={name(lang, '.md')}", "-M", f"zip-href={name(lang, '-chapters.zip')}",
         "-M", f"zip-label={L['zip_label']}", "-o", str(dest)], f"pandoc html ({lang})",
        dict(os.environ, RNL_LINKS_LOG=str(log)))


def build_pdf(src: Path, lang: str, rev: dict, out: Path, work: Path, fonts: dict) -> None:
    typ = work / f"{lang}.typ"
    font_args = [a for k, fams in fonts.items() for f in fams for a in ("-M", f"{k}={f}")]
    run(["pandoc", str(src), *common_args(lang, rev), "-t", "typst", "--standalone",
         "--template", str(HERE / "print.typ"), "-M", f"short-title=RNL {VERSION}", *font_args,
         "-o", str(typ)], f"pandoc typst ({lang})")
    run(["typst", "compile", "--root", str(work), str(typ), str(out / name(lang, ".pdf"))], f"typst ({lang})")


def build_zip(spec: Path, lang: str, rev: dict, out: Path) -> None:
    """The chapters as files, with a README that lists them and a MANIFEST of hashes; reproducible."""
    L = LANGS[lang]
    src = spec / VERSION / lang
    files = {p: read(src / p) for p in PARTS}
    rows = []
    for p, text in files.items():
        m = re.search(r"^## (.+)$", text, re.M)
        rows.append(f"| [{p}]({p}) | {m.group(1) if m else ''} |")
    for f in (f"rnl-words-{VERSION}.json", "rnl-words.schema.json"):
        files[f] = read(spec / f)
        rows.append(f"| [{f}]({f}) | |")
    readme = (STAMP.sub("", read(src / "front.md")).strip() + f"\n\n{revision_label(lang, rev)}\n\n## {L['files']}\n\n"
              f"| {L['file']} | {L['content']} |\n|---|---|\n" + "\n".join(rows) + "\n")
    files = {"README.md": readme, **files}
    manifest = [f"RNL {VERSION} ({lang}), {revision_label(lang, rev)}", ""]
    manifest += [f"{hashlib.sha256(t.encode('utf-8')).hexdigest()}  {f}" for f, t in files.items()]
    files["MANIFEST.txt"] = "\n".join(manifest) + "\n"
    stamp = time.gmtime(rev["epoch"])[:6] if rev["epoch"] else (1980, 1, 1, 0, 0, 0)
    with zipfile.ZipFile(out / name(lang, "-chapters.zip"), "w", zipfile.ZIP_DEFLATED) as z:
        for f, t in files.items():
            info = zipfile.ZipInfo(f"{name(lang, '')}/{f}", date_time=stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, t.encode("utf-8"))


def write_sums(d: Path) -> None:
    lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}"
             for p in sorted(d.iterdir()) if p.is_file() and p.name != "SHA256SUMS"]
    (d / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


# --- main ----------------------------------------------------------------------------------------

def stamp(spec: Path) -> None:
    for part in TRANSLATED:
        p = spec / VERSION / "zh" / part
        text = STAMP.sub("", read(p))
        line = f"<!-- translated from en/{part} sha256:{en_hash(read(spec / VERSION / 'en' / part))} -->\n"
        p.write_text(line + text, encoding="utf-8", newline="\n")
    for lang in LANGS:  # the combined text holds the stamps too
        (spec / name(lang, ".md")).write_text(combined(spec, lang), encoding="utf-8", newline="\n")
    print(f"stamped {len(TRANSLATED)} Chinese parts and rewrote the combined texts")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--spec", type=Path, default=HERE.parents[1] / "spec", help="the spec/ folder (default: the repository's)")
    ap.add_argument("--out", type=Path, default=HERE / "dist", help="where the downloads go (default: scripts/spec/dist/)")
    ap.add_argument("--site", type=Path, help="a built site to add the web pages and their downloads to")
    ap.add_argument("--check", action="store_true", help="run the checks only")
    ap.add_argument("--no-pdf", action="store_true", help="skip the PDFs")
    ap.add_argument("--stamp", action="store_true", help="stamp the Chinese parts with the hashes of the English ones")
    args = ap.parse_args(argv)
    spec = args.spec.resolve()
    if args.stamp:
        stamp(spec)
        return 0

    failures = []
    for what, check in CHECKS:
        msgs = check(spec)
        print(f"check {what}: {'ok' if not msgs else f'{len(msgs)} problem(s)'}")
        failures += msgs
    if failures:
        print("\n".join(failures))
        return 1
    if args.check:
        return 0

    out, work = args.out, args.out / "work"
    work.mkdir(parents=True, exist_ok=True)
    for f in out.glob(f"rnl-{VERSION}*"):
        f.unlink()
    rev = revision(spec)
    fonts = typst_fonts()
    try:
        for lang in LANGS:
            shutil.copyfile(spec / name(lang, ".md"), out / name(lang, ".md"))
            src = pandoc_input(spec, lang, work)
            build_html(src, lang, rev, out / name(lang, ".html"), name(LANGS[lang]["other"], ".html"), work)
            if not args.no_pdf:
                build_pdf(src, lang, rev, out, work, fonts)
            build_zip(spec, lang, rev, out)
            if args.site:
                L = LANGS[lang]
                d = args.site / L["site_dir"].format(v=VERSION)
                d.mkdir(parents=True, exist_ok=True)
                build_html(src, lang, rev, d / "index.html", L["site_other"].format(v=VERSION), work)
                for ext in (".md", "-chapters.zip") + (() if args.no_pdf else (".pdf",)):
                    shutil.copyfile(out / name(lang, ext), d / name(lang, ext))
                write_sums(d)
            print(f"built {lang}: {revision_label(lang, rev)}")
    except Failed as e:
        print(e)
        return 1
    write_sums(out)
    for p in sorted(out.glob(f"rnl-{VERSION}*")):
        print(f"  {p.name:28} {p.stat().st_size // 1024:6} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
