"""Build the introduction pages in every language from the one bilingual source in src/.

In src/, every piece of text a reader sees is written once per language, side by side:

    <zh>网表定义电路</zh><en>The netlist defines the circuit</en>

The same markup is used in HTML text, in attribute values and in JavaScript strings. Only
whitespace may separate the blocks of a group, and every group lists the languages in the order of
LANGS; a block may be empty where a language needs no text. The build keeps one block of each
group per language and checks that
- every <zh> block is followed by an <en> block, with no stray or nested language tags;
- the English pages contain no Chinese characters, apart from Chinese content meant for every page:
  elements marked lang="zh-CN", and in scripts the blocks between /*zh-CN*/ and /*/zh-CN*/ (RNL
  written with the Chinese alias package, for example).

Output (site/dist/ by default, ignored by Git), for each page in PAGES:
    index.html, language.html, assets/, *-standalone.html       Chinese, at the site root
    en/index.html, en/language.html, en/assets/, en/*-standalone.html   English
The standalone files have the styles and scripts inlined and link the other pages to the published
site.

Usage, from the repository root (standard library only):
    python site/build.py              build into site/dist/
    python site/build.py --check      check the sources, write nothing
    python site/build.py --out DIR    build into DIR instead
    python site/build.py --zip FILE   also pack the built site into FILE
"""
import argparse
import re
import shutil
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SRC = HERE / "src"
DEFAULT_OUT = HERE / "dist"
SITE_URL = "https://rich-netlist.github.io/rnl/"

# (language code, output directory); the first language sits at the site root.
LANGS = [("zh", ""), ("en", "en/")]
CODES = [code for code, _ in LANGS]

# Each page and the name of its single-file version.
PAGES = {
    "index.html": "rnl-intro-standalone.html",
    "language.html": "rnl-language-standalone.html",
}

LANG_TAG = re.compile(r"</?(?:%s)>" % "|".join(CODES))
# One block per language, in LANGS order, separated only by whitespace; a block holds no language tags.
GROUP = re.compile(r"\s*".join(r"<%s>((?:(?!%s).)*)</%s>" % (c, LANG_TAG.pattern, c) for c in CODES), re.S)
CJK = re.compile("[%s]" % "".join("%s-%s" % (chr(a), chr(b)) for a, b in
                                  [(0x3000, 0x303F), (0x3400, 0x9FFF), (0xF900, 0xFAFF), (0xFF00, 0xFFEF)]))
ZH_ELEMENT = re.compile(r"<(\w+)\b[^>]*\blang=\"zh[^\"]*\"[^>]*>.*?</\1>", re.S)
ZH_BLOCK = re.compile(r"/\*zh-CN\*/.*?/\*/zh-CN\*/", re.S)

STYLE_TAG = re.compile(r'<link rel="stylesheet" href="assets/([\w.-]+\.css)">')
SCRIPT_TAG = re.compile(r'<script src="assets/([\w.-]+\.js)"></script>')
NOTICE = "<!-- Built by site/build.py from site/src/. Edit the source, not this file. -->\n"


class BuildError(Exception):
    pass


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def blank(m):
    return re.sub(r"[^\n]", " ", m.group(0))


def check_groups(text, name):
    """Report every language tag that is not part of a complete group."""
    rest = GROUP.sub(blank, text)
    stray = {}
    for m in LANG_TAG.finditer(rest):
        stray.setdefault(line_of(rest, m.start()), []).append(m.group(0))
    shape = "".join("<%s>…</%s>" % (c, c) for c in CODES)
    return ["%s:%d: %s not part of a complete %s group" % (name, n, " ".join(tags), shape)
            for n, tags in sorted(stray.items())]


def localize(text, index):
    return GROUP.sub(lambda m: m.group(index + 1), text)


def check_leaks(text, name):
    """A non-Chinese page must not contain Chinese outside the content marked as Chinese."""
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


def build_language(files, index):
    """Return {relative path: text} for one language, including the standalone pages."""
    code, prefix = LANGS[index]
    out = {path: localize(text, index) for path, text in files.items()}
    errors = []
    if code != "zh":
        for path, text in out.items():
            errors += check_leaks(text, "%s (%s)" % (path, code))
    if errors:
        raise BuildError("\n".join(errors))
    for page, single in PAGES.items():
        out[page] = out[page].replace("<!doctype html>\n", "<!doctype html>\n" + NOTICE, 1)
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
    return re.sub(r'(\shref=")([^"]*)"', absolute, html)


def build():
    files = sources()
    errors = []
    for path, text in files.items():
        errors += check_groups(text, "src/" + path)
    if errors:
        raise BuildError("\n".join(errors))
    result = {}
    for index, (code, prefix) in enumerate(LANGS):
        for path, text in build_language(files, index).items():
            result[prefix + path] = text
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
        target.write_text(text, encoding="utf-8", newline="\n")


GUIDE = """RNL introduction pages / RNL 介绍页

rnl-intro/ is the complete site: open rnl-intro/index.html (Chinese) or rnl-intro/en/index.html
(English), or upload the folder to any static host. .nojekyll makes GitHub Pages publish it as is.
language.html next to each index.html is the language tour. The *-standalone.html files in each
language folder are single files with everything inlined. Fonts load from Google Fonts; offline,
system fonts are used and the pages still work.

rnl-intro/ 是完整站点：打开 rnl-intro/index.html（中文）或 rnl-intro/en/index.html（英文），
或把整个目录上传到任意静态主机；.nojekyll 让 GitHub Pages 原样发布。每个 index.html 旁边的
language.html 是语言导览。各语言目录里的 *-standalone.html 是内联了全部样式和脚本的单文件版。
字体从 Google Fonts 加载，离线时退回系统字体，页面照常可用。
"""


def pack(result, zip_path):
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for path, text in sorted(result.items()):
            zf.writestr("rnl-intro/" + path, text)
        zf.writestr("rnl-intro/.nojekyll", "")
        zf.writestr("README.txt", GUIDE)


def main():
    ap = argparse.ArgumentParser(description="Build the RNL introduction pages in every language.")
    ap.add_argument("--check", action="store_true", help="check the sources and write nothing")
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT, help="output directory (default: site/dist)")
    ap.add_argument("--zip", type=Path, help="also pack the built site into this zip file")
    args = ap.parse_args()
    for stream in (sys.stdout, sys.stderr):
        stream.reconfigure(errors="backslashreplace")
    try:
        result = build()
    except BuildError as e:
        print(e, file=sys.stderr)
        print("build failed", file=sys.stderr)
        return 1
    groups = len(GROUP.findall("\n".join(sources().values())))
    if args.check:
        print("ok: %d text groups, each in %s" % (groups, " and ".join(CODES)))
        return 0
    out = args.out.resolve()
    write(result, out)
    print("built %d files into %s (%d text groups)" % (len(result), out, groups))
    for code, prefix in LANGS:
        for page in PAGES:
            print("  %s: %s" % (code, out / prefix / page))
    if args.zip:
        pack(result, args.zip)
        print("packed %s" % args.zip.resolve())
    return 0


if __name__ == "__main__":
    sys.exit(main())
