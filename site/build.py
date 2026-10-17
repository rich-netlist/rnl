"""Build the pages of the project site in every language from the one bilingual source in src/.

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

The page of the tools links to the files of the desktop viewer's newest release. Its version comes
from the newest tag viewer-vX.Y.Z of the repository (a tag with anything after the version, such as
a release candidate's, does not count), or from --viewer-version. The parts of a page between
<!--release--> and <!--/release--> are kept when there is a release, with {{viewer-version}} (such
as 0.1.0) and {{viewer-tag}} (viewer-v0.1.0) filled in; the parts between <!--no-release--> and
<!--/no-release--> are kept when there is none. The build checks that these blocks close and do not
nest, and that the fields are only used in <!--release--> blocks.

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
"""
import argparse
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
SRC = HERE / "src"
DEFAULT_OUT = HERE / "dist"
SITE_URL = "https://rich-netlist.github.io/rnl/"

# (language code, output directory). The root of the site leads to the Chinese pages when the browser's
# first language is Chinese, and otherwise to HOME; the Chinese pages, which were at the root until
# 2026-10-18, keep their old addresses there, which lead to MOVED.
LANGS = [("zh", "zh/"), ("en", "en/")]
CODES = [code for code, _ in LANGS]
DIRS = dict(LANGS)
HOME = DIRS["en"]
MOVED = DIRS["zh"]

# Each page and the name of its single-file version; None for a page that has none, such as the
# gallery of the standard packages, which loads its data from gallery/ (site/gallery/build.mjs).
PAGES = {
    "index.html": "rnl-intro-standalone.html",
    "language.html": "rnl-language-standalone.html",
    "tools.html": "rnl-tools-standalone.html",
    "packages.html": None,
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

# The desktop viewer's releases: a tag viewer-vX.Y.Z and a GitHub release of that name with the files
# the page of the tools links to (.github/workflows/viewer-release.yml makes them).
VIEWER_TAG = re.compile(r"viewer-v(\d+)\.(\d+)\.(\d+)")
VERSION = re.compile(r"\d+\.\d+\.\d+")
# The parts of a page kept with a release of the desktop viewer, or without one, and the fields
# filled in from the release.
RELEASE = re.compile(r"<!--(release|no-release)-->(.*?)<!--/\1-->", re.S)
RELEASE_MARK = re.compile(r"<!--/?(?:no-)?release-->")
FIELD = re.compile(r"\{\{(viewer-[a-z-]+)\}\}")
FIELDS = ("viewer-version", "viewer-tag")


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
    kept = RELEASE.sub(lambda m: blank(m) if m.group(1) == "release" else m.group(0), text)
    for m in FIELD.finditer(kept):
        errors.append("%s:%d: {{%s}} outside a <!--release--> block" % (name, line_of(kept, m.start()), m.group(1)))
    return errors


def release(text, version):
    """Keep the release blocks that apply to `version`, None for no release, with the fields filled in."""
    with_release = version is not None
    text = RELEASE.sub(lambda m: m.group(2) if (m.group(1) == "release") == with_release else "", text)
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


def build_language(files, index, version):
    """Return {relative path: text} for one language, including the standalone pages. Both kinds
    of release block are checked for Chinese before the ones that do not apply are dropped."""
    code, prefix = LANGS[index]
    out = {path: localize(text, index) for path, text in files.items()}
    errors = []
    if code != "zh":
        for path, text in out.items():
            errors += check_leaks(text, "%s (%s)" % (path, code))
    if errors:
        raise BuildError("\n".join(errors))
    out = {path: release(text, version) for path, text in out.items()}
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
    """index.html at the root of the site. It leads to the Chinese pages when the browser's first language
    is Chinese, zh or zh-CN, zh-TW, zh-Hans and so on, and to the English ones otherwise, HOME, keeping the
    query and the section of the address; without scripts, to HOME. It names both languages as its
    alternates, HOME as the default, and links them."""
    zh, en = DIRS["zh"], DIRS["en"]
    return ('<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n'
            '<meta name="viewport" content="width=device-width, initial-scale=1">\n<title>Rich Netlist (RNL)</title>\n'
            '<link rel="alternate" hreflang="zh" href="%s%s">\n<link rel="alternate" hreflang="en" href="%s%s">\n'
            '<link rel="alternate" hreflang="x-default" href="%s%s">\n'
            '<script>var first = (navigator.languages && navigator.languages[0]) || navigator.language || "";\n'
            'var to = /^zh(-|$)/i.test(first) ? "%s" : "%s";\n'
            'if (location.protocol === "file:") to += "index.html";\n'
            'location.replace(to + location.search + location.hash);</script>\n'
            '<noscript><meta http-equiv="refresh" content="0; url=%s"></noscript>\n</head>\n'
            '<body><p><a href="%s" hreflang="zh" lang="zh-CN">中文</a> · <a href="%s" hreflang="en">English</a></p></body>\n'
            '</html>\n') % (SITE_URL, zh, SITE_URL, en, SITE_URL, HOME, zh, HOME, HOME, zh, en)


def redirects():
    """The pages at the root of the site: index.html, which leads to the pages in the browser's language,
    and the old addresses of the Chinese pages, which lead to the same pages under zh/ whatever the
    browser's language."""
    out = {"index.html": home()}
    for page, single in PAGES.items():
        for name in (page, single):
            if name and name != "index.html":
                out[name] = redirect(MOVED + name, "zh-CN", "这一页已移到 " + MOVED + name)
    return out


def build(version=None):
    """Build every page in every language, with the files of the desktop viewer's release `version`,
    or None for no release yet, and the pages at the root that lead to them."""
    files = sources()
    errors = []
    for path, text in files.items():
        errors += check_groups(text, "src/" + path) + check_release(text, "src/" + path)
    if errors:
        raise BuildError("\n".join(errors))
    result = {}
    for index, (code, prefix) in enumerate(LANGS):
        for path, text in build_language(files, index, version).items():
            result[prefix + path] = text
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
        target.write_text(text, encoding="utf-8", newline="\n")


GUIDE = """RNL introduction pages / RNL 介绍页

rnl-intro/ is the complete site: open rnl-intro/zh/index.html (Chinese) or rnl-intro/en/index.html
(English), or upload the folder to any static host, where its root leads to the pages in the
browser's language.
.nojekyll makes GitHub Pages publish it as is.
language.html next to each index.html is the language tour, tools.html the page of the viewers, and
packages.html the gallery of the standard packages; the web viewer itself, the pages of the standard
and the gallery's data are published with the site, not packed here.
The *-standalone.html files in each language folder are single files with everything inlined.
Fonts load from Google Fonts; offline, system fonts are used and the pages still work.

rnl-intro/ 是完整站点：打开 rnl-intro/zh/index.html（中文）或 rnl-intro/en/index.html（英文），
或把整个目录上传到任意静态主机，根目录按浏览器语言跳到中文页或英文页；.nojekyll 让 GitHub Pages 原样发布。每个 index.html 旁边的
language.html 是语言导览，tools.html 是查看器的介绍页，packages.html 是标准包图库；网页版查看器本身、
语言标准的网页和图库的数据随网站发布，不在这个包里。各语言目录里的 *-standalone.html 是内联了全部样式和脚本的单文件版。
字体从 Google Fonts 加载，离线时退回系统字体，页面照常可用。
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
    print("  the root leads to %s in a browser whose first language is Chinese, else to %s; the old addresses"
          " of the Chinese pages lead to %s" % (DIRS["zh"], HOME, MOVED))
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
