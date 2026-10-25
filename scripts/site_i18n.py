"""Shared website locale registry, English fallback, markup and language navigation.

Language groups end with their required <en> block; translations before it are optional.
An explicit empty block is intentional, while an absent block falls back to English.
A translation may also come from the catalog of its source file, <registry folder>/<code>/site/<path
under site/src>.json, keyed by the English with its runs of white space as one space; the block
beside the English comes first (decision 0015, i18n/README.md). Where one English text is said in two
ways, its block names a context, <en ctx=NAME>, and the catalog's key is NAME|English.
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
CONTEXT = r"(?: ctx=([a-z0-9-]+))?"  # the context an English block may name: <en ctx=readable>
CONTEXT_KEY = re.compile(r"^[a-z0-9-]+\|")
ELEMENT = re.compile(r"<(/?)([a-zA-Z][a-zA-Z0-9]*)\b[^<>]*?(/?)>")
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}


def norm(text):
    """The key of an English text in a catalog: its runs of white space as one space."""
    return " ".join(text.split())


def unclosed(text):
    """The elements a text closes without opening them, and those it opens without closing them, in
    order: what a translation must leave as its English does, so that the page around it holds."""
    opened, closed = [], []
    for close, name, alone in ELEMENT.findall(text):
        name = name.lower()
        if alone or name in VOID:
            continue
        if not close:
            opened.append(name)
        elif opened and opened[-1] == name:
            opened.pop()
        else:
            closed.append(name)
    return closed, opened


class Languages:
    def __init__(self, config, catalogs=None):
        """catalogs: the folders of the languages' folders of catalogs, the first that has a catalog giving
        it, such as the registry's own and then the repository's i18n/; one folder, or None for none."""
        self.catalogs = [Path(c) for c in ([catalogs] if isinstance(catalogs, (str, Path)) else catalogs or [])]
        self._catalogs = {}
        self.default = config["default"]
        self.legacy = config["legacy"]
        entries = config["languages"]
        self.entries = {entry["code"]: entry for entry in entries}  # as written, without English's keys
        self.codes = [entry["code"] for entry in entries]
        # the languages shown in the menus and chosen for a browser: English and those `published`; one that
        # is `incubating`, or has no status yet, is built for previews and tests only (decision 0015)
        self.published = [code for code in self.codes
                          if code == config["default"] or self.entries[code].get("status") == "published"]
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
            # the shared messages: English's in the registry, a translation's in its catalog messages.json
            messages = {key: text for key, text in self.catalog(code, "messages").items() if isinstance(text, str)}
            locale["messages"] = {**fallback["messages"], **entry.get("messages", {}), **messages}
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
        self.tag = re.compile(rf"</?(?:{tags})(?: ctx=[a-z0-9-]+)?>")
        body = rf"(?:(?!{self.tag.pattern}).)*?"
        block = lambda code: rf"<{re.escape(code)}(?: ctx=[a-z0-9-]+)?>{body}</{re.escape(code)}>"
        translated = "|".join(block(code) for code in self.codes if code != self.default)
        self.group = re.compile(rf"(?:(?:{translated})\s*)*{block(self.default)}", re.S)
        self.block = re.compile(rf"<({tags}){CONTEXT}>({body})</\1>", re.S)

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
        # a language takes its catalogs from the first folder that has a folder of its own, else from the
        # first that has the file: a test's language beside its registry is not mixed with the repository's
        # catalogs of the same code
        own = [folder for folder in self.catalogs if (folder / code).is_dir()][:1]
        found = [folder / code / "site" / (path + ".json") for folder in own or self.catalogs]
        return next((file for file in found if file.is_file()), found[0] if found else None)

    def catalog(self, code, path):
        """{English: translation, or None where there is none yet} for one source file, such as
        "index.html" or "assets/intro.js", in one language; {} when it has none."""
        if code == self.default or path is None or not self.catalogs:
            return {}
        if (code, path) not in self._catalogs:
            file = self.catalog_path(code, path)
            try:
                data = json.loads(file.read_text(encoding="utf-8")) if file.is_file() else {}
            except ValueError:
                data = None
            self._catalogs[code, path] = data if isinstance(data, dict) else {}
        return self._catalogs[code, path]

    def with_catalog(self, values, path):
        """A dictionary {language: text} with the translations that the catalog of `path` has for its English,
        where it has none of its own."""
        found = dict(values)
        for code in self.codes:
            if code != self.default and code not in found:
                text = self.catalog(code, path).get(norm(values[self.default]))
                if isinstance(text, str):
                    found[code] = text
        return found

    def parts(self, group):
        """{language: text} of a translation group, and the context its English names, or ""."""
        found = self.block.findall(group)
        return {code: text for code, _, text in found}, next((ctx for code, ctx, _ in found if code == self.default), "")

    def key(self, blocks, context=""):
        """The key of a group in a catalog: its English, with its context before it when it names one."""
        english = norm(blocks[self.default])
        return f"{context}|{english}" if context else english

    def translation(self, blocks, code, path=None, context=""):
        """The text of one group in one language, or None where it has none: its block beside the English,
        else its catalog's entry. An explicit empty text is a translation."""
        if code in blocks:
            return blocks[code]
        value = self.catalog(code, path).get(self.key(blocks, context))
        return value if isinstance(value, str) else None

    def coverage(self, text, code, path=None):
        """(translated, all) of the groups of a text in one language."""
        found = [self.parts(m.group()) for m in self.group.finditer(text)]
        if code == self.default:
            return len(found), len(found)
        return sum(self.translation(blocks, code, path, context) is not None for blocks, context in found), len(found)

    def catalog_errors(self, code, path):
        """What is malformed in the catalog of one source file, as would break the page: not an object of
        texts or null; a placeholder that its English does not have; elements it does not open and close
        as its English does; in a script, more straight quotes or backslashes than its English, which the
        script's string could not hold. Other tags and links than the English's are the translator's
        choice, such as a link to the page in the same language, and are only reported
        (scripts/i18n/status.py)."""
        file = self.catalog_path(code, path)
        if file is None or code == self.default or not file.is_file():
            return []
        name = next((file.relative_to(folder.parent).as_posix() for folder in self.catalogs if folder in file.parents), str(file))
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
            english = CONTEXT_KEY.sub("", key)
            if not set(PLACEHOLDER.findall(value)) <= set(PLACEHOLDER.findall(english)):
                errors.append(f"{where}: a placeholder that the English does not have")
            if unclosed(value) != unclosed(english):
                errors.append(f"{where}: its elements do not open and close as those of the English do")
            if path.endswith(".js") and any(value.count(c) > english.count(c) for c in "'\"\\"):
                errors.append(f"{where}: more straight quotes or backslashes than the English; use ’ “ ”")
        return errors

    def differences(self, english, value):
        """What a translation has other than its English, which is not an error: its tags and its links."""
        found = []
        if Counter(MARKUP_TAG.findall(value)) != Counter(MARKUP_TAG.findall(english)):
            found.append("its tags are not those of the English")
        if Counter(HREF.findall(value)) != Counter(HREF.findall(english)):
            found.append("its links are not those of the English")
        return found

    def localize(self, text, code, path=None, english=False):
        """The text in one language: each group as its translation, or as its English where it has none,
        or everywhere when english is set, as for a page shown in English."""
        def one(m):
            blocks, context = self.parts(m.group())
            self.text(blocks, code)  # checks the group
            value = None if english else self.translation(blocks, code, path, context)
            return blocks[self.default] if value is None else value
        return self.group.sub(one, text)

    def english_name(self, code):
        """A language's name in English, `names.en` of its entry, else its name in itself."""
        entry = self.entries[code]
        return (entry.get("names") or {}).get("en") or entry["name"]

    def notice(self, code, translated, total):
        """The notice of a page not wholly translated, or "" for one that is. Like every notice of the
        system it is in English, in every language (decision 0015): its messages are English's only."""
        if code == self.default or translated >= total:
            return ""
        partial = translated >= SHOWN_IN_ENGLISH_BELOW * total
        text = self.message(self.default, "partly_translated" if partial else "not_translated", language=self.english_name(code))
        return (f'\n<div class="wrap translation-notice{" is-partial" if partial else ""}" lang="{self.locales[self.default]["html_lang"]}">'
                f'<p class="note"><svg class="ico"><use href="#i-info"/></svg><span>{escape(text)} '
                f'<a href="{HELP_URL}">{escape(self.message(self.default, "help_translate"))}</a></span></p></div>')

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
        # the published languages, and the page's own when it is incubating
        for target in [c for c in self.codes if c in self.published or c == code]:
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
    # a registry elsewhere, as tests make, reads its own catalogs and then the repository's
    return Languages(json.loads(path.read_text(encoding="utf-8")), catalogs=dict.fromkeys([path.parent, ROOT / "i18n"]))


def asset(name):
    return (ROOT / "site/src/assets" / name).read_text(encoding="utf-8")


LANGUAGES = load()
