"""Shared website locale registry, English fallback, markup and language navigation.

Language groups end with their required <en> block; translations before it are optional.
An explicit empty block is intentional, while an absent block falls back to English.
A translation may also come from the catalog of its source file, <registry folder>/<code>/site/<path
under site/src>.json, keyed by the English with its runs of white space as one space; the block
beside the English comes first (decision 0015, i18n/README.md).
"""
from collections import Counter
from html import escape
import json
import os
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "i18n/languages.json"
# A page less translated than this is shown in English, with a notice; one more translated, but not
# wholly, is shown translated, with the English where its translation is missing (decision 0015).
SHOWN_IN_ENGLISH_BELOW = 0.8
HELP_URL = "https://github.com/rich-netlist/rnl/blob/main/i18n/README.md"
PLACEHOLDER = re.compile(r"\{\{?[a-z][a-z0-9_-]*\}?\}")
MARKUP_TAG = re.compile(r"<(/?[a-zA-Z][a-zA-Z0-9]*)\b")
HREF = re.compile(r"\bhref=\"([^\"]*)\"")


def norm(text):
    """The key of an English text in a catalog: its runs of white space as one space."""
    return " ".join(text.split())


class Languages:
    def __init__(self, config, catalogs=None):
        """catalogs: the folder of the languages' folders of catalogs, that of the registry; None for none."""
        self.catalogs = Path(catalogs) if catalogs else None
        self._catalogs = {}
        self.default = config["default"]
        self.legacy = config["legacy"]
        entries = config["languages"]
        self.codes = [entry["code"] for entry in entries]
        if (self.default != "en" or self.legacy not in self.codes or "en" not in self.codes
                or len(set(self.codes)) != len(self.codes)
                or any(not re.fullmatch(r"[a-z]{2,3}(?:-[a-z0-9]+)*", code) for code in self.codes)):
            raise ValueError("languages.json needs unique language codes, English default and a registered legacy language")
        fallback = next(entry for entry in entries if entry["code"] == self.default)
        self.locales = {}
        for entry in entries:
            code = entry["code"]
            locale = {**fallback, **entry}
            locale["html_lang"] = entry.get("html_lang", code)
            locale["matches"] = entry.get("matches", [code])
            locale["short"] = entry.get("short", entry["name"])
            locale["standard"] = entry.get("standard", self.default)
            locale["fonts"] = {**fallback["fonts"], **entry.get("fonts", {})}
            locale["messages"] = {**fallback["messages"], **entry.get("messages", {})}
            if (not locale["name"] or not isinstance(locale["nav_breakpoint"], int)
                    or locale["nav_breakpoint"] < 320
                    or not re.fullmatch(r"[A-Za-z0-9-]+", locale["html_lang"])
                    or any(not re.fullmatch(r"[a-z0-9-]+", match) for match in locale["matches"])
                    or any(not isinstance(value, str) for value in locale["messages"].values())):
                raise ValueError(f"Invalid locale metadata: {code}")
            self.locales[code] = locale
        if any(locale["standard"] not in self.locales for locale in self.locales.values()):
            raise ValueError("A standard language must be registered")
        if any(self.locales[locale["standard"]]["standard"] != locale["standard"] for locale in self.locales.values()):
            raise ValueError("Standard language fallbacks must point directly to a published language")
        tags = "|".join(map(re.escape, self.codes))
        self.tag = re.compile(rf"</?(?:{tags})>")
        body = rf"(?:(?!{self.tag.pattern}).)*?"
        block = lambda code: rf"<{re.escape(code)}>{body}</{re.escape(code)}>"
        translated = "|".join(block(code) for code in self.codes if code != self.default)
        self.group = re.compile(rf"(?:(?:{translated})\s*)*{block(self.default)}", re.S)
        self.block = re.compile(rf"<({tags})>({body})</\1>", re.S)

    def text(self, values, code):
        """A missing dictionary entry falls back; an explicit empty string stays empty."""
        if not isinstance(values, dict) or self.default not in values:
            raise ValueError("Localized text needs an English source")
        if any(key not in self.codes or not isinstance(value, str) for key, value in values.items()):
            raise ValueError("Localized text contains an unknown language or a non-string value")
        return values.get(code, values[self.default])

    def message(self, code, key, **values):
        return self.locales[code]["messages"][key].format(**values)

    def errors(self, text, name):
        errors = []
        for match in self.group.finditer(text):
            codes = [block[0] for block in self.block.findall(match.group())]
            if len(set(codes)) != len(codes):
                errors.append(f"{name}:{text.count(chr(10), 0, match.start()) + 1}: duplicate language in translation group")
        rest = self.group.sub(lambda m: re.sub(r"[^\n]", " ", m.group()), text)
        for match in self.tag.finditer(rest):
            errors.append(f"{name}:{rest.count(chr(10), 0, match.start()) + 1}: {match.group()} outside a translation group ending in <en>…</en>")
        return errors

    def catalog_path(self, code, path):
        return self.catalogs / code / "site" / (path + ".json") if self.catalogs else None

    def catalog(self, code, path):
        """{English: translation, or None where there is none yet} for one source file, such as
        "index.html" or "assets/intro.js", in one language; {} when it has none."""
        if code == self.default or path is None or self.catalogs is None:
            return {}
        if (code, path) not in self._catalogs:
            file = self.catalog_path(code, path)
            try:
                data = json.loads(file.read_text(encoding="utf-8")) if file.is_file() else {}
            except ValueError:
                data = None
            self._catalogs[code, path] = data if isinstance(data, dict) else {}
        return self._catalogs[code, path]

    def translation(self, blocks, code, path=None):
        """The text of one group in one language, or None where it has none: its block beside the English,
        else its catalog's entry. An explicit empty text is a translation."""
        if code in blocks:
            return blocks[code]
        value = self.catalog(code, path).get(norm(blocks[self.default]))
        return value if isinstance(value, str) else None

    def coverage(self, text, code, path=None):
        """(translated, all) of the groups of a text in one language."""
        found = [dict(self.block.findall(m.group())) for m in self.group.finditer(text)]
        if code == self.default:
            return len(found), len(found)
        return sum(self.translation(blocks, code, path) is not None for blocks in found), len(found)

    def catalog_errors(self, code, path):
        """What is malformed in the catalog of one source file: not an object of texts or null; a
        placeholder, a tag or a link that its English does not have; more straight quotes or backslashes
        than its English, which a script's string could not hold."""
        file = self.catalog_path(code, path)
        if file is None or code == self.default or not file.is_file():
            return []
        name = file.relative_to(self.catalogs.parent).as_posix() if self.catalogs.parent in file.parents else str(file)
        try:
            data = json.loads(file.read_text(encoding="utf-8"))
        except ValueError as e:
            return [f"{name}: not valid JSON: {e}"]
        if not isinstance(data, dict) or not all(isinstance(v, (str, type(None))) for v in data.values()):
            return [f"{name}: a catalog is an object whose values are texts, or null where there is none yet"]
        errors = []
        for key, value in data.items():
            if not isinstance(value, str):
                continue
            where = f"{name}: {key[:60]!r}"
            if not set(PLACEHOLDER.findall(value)) <= set(PLACEHOLDER.findall(key)):
                errors.append(f"{where}: a placeholder that the English does not have")
            if Counter(MARKUP_TAG.findall(value)) != Counter(MARKUP_TAG.findall(key)):
                errors.append(f"{where}: its tags are not those of the English")
            if Counter(HREF.findall(value)) != Counter(HREF.findall(key)):
                errors.append(f"{where}: its links are not those of the English")
            if any(value.count(c) > key.count(c) for c in "'\"\\"):
                errors.append(f"{where}: more straight quotes or backslashes than the English; use ’ “ ”")
        return errors

    def localize(self, text, code, path=None, english=False):
        """The text in one language: each group as its translation, or as its English where it has none,
        or everywhere when english is set, as for a page shown in English."""
        def one(m):
            blocks = dict(self.block.findall(m.group()))
            self.text(blocks, code)  # checks the group
            value = None if english else self.translation(blocks, code, path)
            return blocks[self.default] if value is None else value
        return self.group.sub(one, text)

    def notice(self, code, translated, total):
        """The notice of a page not wholly translated, in its language, or "" for one that is."""
        if code == self.default or translated >= total:
            return ""
        partial = translated >= SHOWN_IN_ENGLISH_BELOW * total
        locale = self.locales[code]
        text = self.message(code, "partly_translated" if partial else "not_translated", language=locale["name"])
        return (f'\n<div class="wrap translation-notice{" is-partial" if partial else ""}" lang="{locale["html_lang"]}">'
                f'<p class="note"><svg class="ico"><use href="#i-info"/></svg><span>{escape(text)} '
                f'<a href="{HELP_URL}">{escape(self.message(code, "help_translate"))}</a></span></p></div>')

    def markup(self, values, html=False):
        self.text(values, self.default)
        # English closes each group; adding a locale never changes existing source groups.
        codes = [code for code in self.codes if code != self.default] + [self.default]
        return "".join(f"<{code}>{value if html else escape(value)}</{code}>"
                       for code in codes if code in values for value in [values[code]])

    def font_url(self, code):
        return ("https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600"
                "&family=IBM+Plex+Sans+Condensed:wght@500;600;700&family="
                + self.locales[code]["fonts"]["google"] + "&display=swap")

    def css(self, code):
        locale = self.locales[code]
        fonts = locale["fonts"]
        return (f':root {{ --f-body: {fonts["body"]}; --f-disp: {fonts["display"]}; --f-mono: {fonts["mono"]}; }}\n'
                f'@media (max-width: {locale["nav_breakpoint"]}px) {{\n'
                '  .brand-sub { display: none; }\n'
                '  .topbar-in { gap: 10px; }\n'
                '  .language-menu { margin-left: auto; }\n'
                '  .menu-btn { display: inline-grid; }\n'
                # the links of the pages become a menu: a panel below the top bar, one link to a line
                '  .pages {\n'
                '    display: none; position: fixed; z-index: 45; top: 66px; left: 12px; right: 12px; margin: 0; padding: 8px;\n'
                '    flex-direction: column; gap: 2px; max-height: calc(100dvh - 78px); overflow-y: auto;\n'
                '    border: 1px solid var(--line); border-radius: var(--radius); background: var(--panel); box-shadow: var(--shadow);\n  }\n'
                '  .pages.is-open { display: flex; }\n'
                '  .pages a { padding: 12px 14px; border-radius: 8px; font-size: 16px; }\n'
                '  .pages a.is-on { color: var(--markup); background: var(--markup-2); box-shadow: none; border-radius: 8px; font-weight: 600; }\n}\n')

    def menu(self, code, href):
        locale = self.locales[code]
        title = self.message(code, "language_menu")
        links = []
        for target in self.codes:
            item = self.locales[target]
            current = ' aria-current="true"' if target == code else ""
            links.append(f'<a href="{escape(href(target), quote=True)}" hreflang="{item["html_lang"]}" '
                         f'lang="{item["html_lang"]}" data-lang-link data-language="{target}"{current}>'
                         f'{escape(item["name"])}<span aria-hidden="true">{"✓" if target == code else ""}</span></a>')
        return ('<details class="language-menu">'
                f'<summary class="lang-link" aria-label="{escape(title, quote=True)}: {escape(locale["name"], quote=True)}">'
                f'{escape(locale["short"])}<svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 6 4 4 4-4"/></svg></summary>'
                f'<nav class="language-options" aria-label="{escape(title, quote=True)}">'
                + "".join(links) + '</nav></details>')

    def standard(self, code):
        return self.locales[code]["standard"]


def load(path=None):
    path = Path(path or os.environ.get("RNL_SITE_LANGUAGES", CONFIG))
    return Languages(json.loads(path.read_text(encoding="utf-8")), catalogs=path.parent)


def asset(name):
    return (ROOT / "site/src/assets" / name).read_text(encoding="utf-8")


LANGUAGES = load()
