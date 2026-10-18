#!/usr/bin/env python3
"""Builds the RNL skill package, skills/rnl/: the files that are generated, a check that the package is
whole and that every example in it reads, and its published forms on the project site.

The package is a folder in the Agent Skills format (SKILL.md and the files it points to) that teaches an
agent to read and write RNL 0.3. Its short guide to the language is spec/rnl-0.3-guide.md and
spec/rnl-0.3-guide.zh.md, copied; its reference to the standard packages is written from the package files
in libraries/ and from `rnl catalog`; the rest is written by hand. Standard library only; the checks need
the `rnl` command, which is built with `cargo build -p rnl-cli`."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SKILL = ROOT / "skills" / "rnl"
SPEC = ROOT / "spec"
SERIES = "0.3"
SITE_URL = "https://rich-netlist.github.io/rnl/"
GITHUB = "https://github.com/rich-netlist/rnl"

# the short guide to the language: its source, and where the skill keeps its copy. The package is in English
# only; the Chinese guide, spec/rnl-0.3-guide.zh.md, is for the Chinese page of the standard.
GUIDE = (SPEC / "rnl-0.3-guide.md", "references/language.md")
GUIDE_NOTE = (f"> The short guide to the language RNL {SERIES}, for reading and writing RNL files. It is not the standard, "
              f"which prevails where the two differ: {SITE_URL}en/spec/{SERIES}/. Licence: CC-BY-4.0, see `LICENSE.md`.")
LICENSE_TEXTS = ("MIT", "CC-BY-4.0", "CC0-1.0")  # the licences of what the package holds, copied to LICENSES/
GALLERY_JS = ROOT / "site" / "src" / "assets" / "gallery.js"  # what each package is for, in both languages
SITE_SRC = ROOT / "site" / "src"
# the package files the reference is written from: the series, and the packages outside it that the
# catalog lists with it
LIBRARIES = [ROOT / "libraries" / "standard" / SERIES, ROOT / "libraries" / "analog-canvas-extensions"]
ALLOWED_FRONT = {"name", "description", "license", "compatibility", "metadata", "allowed-tools"}
SUMMARY_LINE = re.compile(r"^\s*'([\w-]+(?: [\d.]+)?)':\s*'<zh>.*?</zh><en>(.*?)</en>'", re.M)
FENCE = re.compile(r"^(```+)([^\n]*)\n(.*?)^\1[ \t]*$", re.M | re.S)
RULE = re.compile(r'#\s*@SELECT-INSTANCE\((\w+)="([^"]*)"\)\s*=>\s*@SET-SYMBOL\s+(\$\w+)')
ALIAS = re.compile(r'@ALIAS\("([^"]+)",\s*"([^"]+)"\)')
PACKAGE = re.compile(r'@PACKAGE\(name="([^"]+)",\s*version="([^"]+)"\)')
COUNTS = re.compile(r"(\d+) errors?, (\d+) warnings?, (\d+) hints?")


class Failed(Exception):
    pass


def read(p: Path) -> str:
    return p.read_text(encoding="utf-8").replace("\r\n", "\n")


# --- the rnl command ---------------------------------------------------------------------------------

def find_rnl(given: str | None) -> Path:
    """The rnl command: --rnl, else $RNL, else the one cargo built here, else the one on PATH."""
    exe = ".exe" if os.name == "nt" else ""
    for cand in (given, os.environ.get("RNL"), ROOT / "target" / "debug" / f"rnl{exe}",
                 ROOT / "target" / "release" / f"rnl{exe}", shutil.which("rnl")):
        if cand and Path(cand).is_file():
            return Path(cand)
    raise Failed("no rnl command: build it with `cargo build -p rnl-cli` or give --rnl PATH")


def run_rnl(rnl: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run([str(rnl), *args], capture_output=True, text=True, encoding="utf-8")


# --- reading the package files ----------------------------------------------------------------------

def package_files() -> dict[tuple[str, str], tuple[Path, str]]:
    """{(name, version): (file, text)} of every package file of the series and of the packages listed with it."""
    found = {}
    for d in LIBRARIES:
        for f in sorted(d.glob("*.rnl")):
            text = read(f)
            m = PACKAGE.search(text)
            if not m:
                raise Failed(f"{f.relative_to(ROOT)}: no @PACKAGE in its header record")
            found[m.group(1), m.group(2)] = (f, text)
    return found


def comment_before(text: str, pos: int) -> str:
    """The comment that ends right before pos, on one line; empty when there is none or it is a divider."""
    head = text[:pos].rstrip()
    if not head.endswith("*/"):
        return ""
    start = head.rfind("/*")
    body = " ".join(head[start + 2:-2].split())
    return "" if body.startswith("---") else body


def table_cell(text: str, limit: int = 240) -> str:
    """Text for a cell of a Markdown table: no pipes, and cut at a word when it is long."""
    text = text.replace("|", "/")
    return text if len(text) <= limit else text[:limit].rsplit(" ", 1)[0] + " …"


def definitions_comments(text: str) -> dict[str, str]:
    """{name: the comment right before its definition} for the `$NAME = @DEFINE(` statements of a package file."""
    return {m.group(1): table_cell(comment_before(text, m.start()))
            for m in re.finditer(r"^\s*(\$\w+)\s*=\s*@DEFINE\(", text, re.M)}


def symbols_in(text: str) -> set[str]:
    return set(re.findall(r"^\s*(\$\w+)\s*=\s*@DEFINE-SYMBOL\(", text, re.M))


def summaries() -> dict[str, str]:
    """What each package is for, in English, from the gallery's table: {'rnl-std-fill': …, 'rnl-std-fill 0.3': …}."""
    return {m.group(1): re.sub(r"\s+", " ", m.group(2)).replace("’", "'") for m in SUMMARY_LINE.finditer(read(GALLERY_JS))}


# --- the reference to the packages ------------------------------------------------------------------

def role_text(role) -> str:
    """A role as the catalog gives it, a name or [name, kind(s)], as a file writes it: `in_p` (instance)."""
    if isinstance(role, str):
        return f"`{role}`"
    name, kinds = role
    kinds = kinds if isinstance(kinds, str) else "/".join(kinds)
    return f"`{name}` ({kinds})"


def packages_md(rnl: Path) -> str:
    r = run_rnl(rnl, "catalog")
    if r.returncode != 0:
        raise Failed(f"rnl catalog:\n{r.stderr or r.stdout}")
    cat = json.loads(r.stdout)
    files = package_files()
    brief = summaries()

    def wanted(x: dict) -> bool:  # packages name themselves "name", what they hold name them "package"
        return x["version"] == SERIES or x.get("package", x.get("name")) == "analog-canvas-extensions"

    pkgs = sorted((p for p in cat["packages"] if wanted(p)), key=lambda p: p["name"])
    # the catalog is the one the command bundles: it must be the package files of this checkout
    errors = []
    for p in pkgs:
        if (p["name"], p["version"]) not in files:
            errors.append(f"the catalog lists {p['name']} {p['version']}, which has no package file in libraries/")
    for key in files:
        if key[0] not in {p["name"] for p in pkgs}:
            errors.append(f"{key[0]} {key[1]} has a package file, but the catalog does not list it: rebuild rnl")
    syms = [s for s in cat["symbols"] if wanted(s)]
    for (name, version), (_, text) in files.items():
        in_cat = {s["symbol"] for s in syms if s["package"] == name}
        if in_cat != symbols_in(text):
            errors.append(f"{name}: the symbols of the catalog and of the file differ "
                          f"({sorted(in_cat ^ symbols_in(text))[:5]}): rebuild rnl")
    if errors:
        raise Failed("\n".join(errors))

    out = [
        f"# The standard packages, series {SERIES}",
        "",
        f"<!-- Generated by scripts/skill/build.py from libraries/ and `rnl catalog`. Edit those, not this file. -->",
        "",
        f"What a file of RNL {SERIES} can name without writing it: the definitions its statements fill (`@AS $NAME`), "
        "the symbols its instances are drawn with, and the sets it edits. The package `rnl-std` is used at the root "
        "of every file implicitly; the others are used by naming them in the header record. A name that is not in the "
        "packages in use is an error, and `rnl check` suggests the closest one. This reference is written from the "
        f"package files of the series {SERIES} and from the catalog of the `rnl` command, and is in step with them.",
        "",
        "Using a package other than `rnl-std`, in the header record (the standard 16.2, 16.3, 16.5):",
        "",
        "```rnl",
        "@RNL(type=\"header\") {",
        f"  @VERSION(\"{SERIES}\");",
        f"  @USE-PACKAGE(name=\"rnl-std-full\", version=\"{SERIES}\")",
        "}",
        "```",
        "",
        "## Packages",
        "",
        "| Package | References | What it is for |",
        "| --- | --- | --- |",
    ]
    for p in pkgs:
        name = f"`{p['name']}` {p['version']}" + (" (implicit)" if p["implicit"] else "")
        what = brief.get(f"{p['name']} {p['version']}") or brief.get(p["name"]) or ""
        if not what:
            errors.append(f"{p['name']}: no line in SUMMARY of {GALLERY_JS.relative_to(ROOT)}")
        refs = ", ".join(f"`{x}`" for x in p["references"]) or "–"
        out.append(f"| {name} | {refs} | {what} |")
    if errors:
        raise Failed("\n".join(errors))

    out += ["", "## Definitions", "",
            "A definition is used as `#A(role) #B(role) @AS $DEFINITION` (the standard 9.5). A role in braces, such as "
            "`{tail}`, is a set: it takes any number of objects, none included. A role has a kind, "
            "which says what may fill it; parameters have defaults.", ""]
    for p in pkgs:
        defs = [d for d in cat["definitions"] if d["package"] == p["name"] and d["version"] == p["version"]]
        if not defs:
            continue
        comments = definitions_comments(files[p["name"], p["version"]][1])
        out += [f"### {p['name']}", "", "| Definition | Roles | Parameters (defaults) | Symbol | Comment before it in the file |", "| --- | --- | --- | --- | --- |"]
        for d in defs:
            roles = ", ".join(role_text(x) for x in d["roles"]) or "–"
            params = ", ".join(f"`{k}` = `{json.dumps(v)}`" for k, v in d["parameters"]) or "–"
            sym = f"`{d['symbol']}`" if d["symbol"] else "–"
            out.append(f"| `{d['definition']}` | {roles} | {params} | {sym} | {comments.get(d['definition'], '')} |")
        out.append("")

    out += ["## Symbols", "",
            "A symbol is chosen by the rules of the next section, or by `@SET-SYMBOL $NAME` on an object or a set. Pins "
            "are listed by terminal number with the direction a wire leaves in. The standard symbols are drawn in "
            "grid steps, y up.", ""]
    for p in pkgs:
        mine = [s for s in syms if s["package"] == p["name"] and s["version"] == p["version"]]
        if not mine:
            continue
        out += [f"### {p['name']}", "", "| Symbol | Pins | Labels | Definition |", "| --- | --- | --- | --- |"]
        for s in sorted(mine, key=lambda s: s["symbol"]):
            pins = " ".join(f"{x['terminal']}:{x['direction']}" for x in sorted(s["pins"], key=lambda x: x["terminal"])) or "–"
            labels = ", ".join(x["key"] for x in s["labels"]) or "–"
            out.append(f"| `{s['symbol']}` | {pins} | {labels} | {('`' + s['definition'] + '`') if s['definition'] else '–'} |")
        out.append("")

    out += ["## Which symbol an instance gets", "",
            "The rules of `rnl-std-spice` and `rnl-std-spectre` give an instance its symbol; the narrow rules (by model type) "
            "come later than the broad ones and win. A file of SPICE uses the first, a file of Spectre the second (16.3).", ""]
    for pkg, intro in (("rnl-std-spice", "SPICE, by the first letter of the instance's name, then by the type of its `.model`"),
                       ("rnl-std-spectre", "Spectre, by master (primitives), then by the type of the model card")):
        text = files[pkg, SERIES][1]
        rules = RULE.findall(text)
        if len(rules) != text.count("@SELECT-INSTANCE"):
            raise Failed(f"{pkg}: a rule in a shape that the generator does not read")
        out += [f"### {pkg}: {intro}", "", "| Selector | Symbol |", "| --- | --- |"]
        out += [f"| `{k}=\"{v}\"` | `{sym}` |" for k, v, sym in rules]
        out.append("")

    out += ["## Sets", "",
            "The sets that `rnl-std-fill` decides the automatic fill by; a file adds to them or takes from them with "
            "`@ADD-TO` and `@REMOVE-FROM`.", "", "| Set | Package |", "| --- | --- |"]
    out += [f"| `{s['set']}` | `{s['package']}` |" for s in cat["sets"] if s["version"] == SERIES]
    out.append("")

    aliases = len(ALIAS.findall(files["rnl-zh", SERIES][1]))
    out += [f"## Chinese aliases", "",
            f"`rnl-zh` {SERIES} gives Chinese spellings of the keywords, of the names above and of the parameters and roles "
            f"of their definitions: {aliases} aliases, listed in `libraries/standard/0.3/rnl-zh.rnl` of the repository. A file "
            f"loads it with `@USE-ALIASES(name=\"rnl-zh\", version=\"{SERIES}\")` in the header record. Each alias reads as "
            "the standard word it stands for (16.5), so a file may mix both spellings, and the standard spellings keep working.", ""]
    return "\n".join(out).rstrip() + "\n"


# --- the files of the package ------------------------------------------------------------------------

def generated(rnl: Path) -> dict[str, str]:
    """{path in the skill: text} of the files that are generated."""
    out = {"references/packages.md": packages_md(rnl)}
    src, dest = GUIDE
    title, _, rest = read(src).partition("\n")
    if not title.startswith("# "):
        raise Failed(f"{src.relative_to(ROOT)}: the first line is not a title, `# …`")
    out[dest] = f"{title}\n\n{GUIDE_NOTE}\n{rest}"
    for name in LICENSE_TEXTS:
        out[f"LICENSES/{name}.txt"] = read(ROOT / "LICENSES" / f"{name}.txt")
    return out


def skill_files() -> list[str]:
    return sorted(p.relative_to(SKILL).as_posix() for p in SKILL.rglob("*") if p.is_file())


def write_generated(rnl: Path) -> None:
    for path, text in generated(rnl).items():
        target = SKILL / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8", newline="\n")
        print(f"wrote skills/rnl/{path}")


# --- checks ------------------------------------------------------------------------------------------

def front_matter(text: str) -> tuple[dict, str]:
    m = re.match(r"---\n(.*?)\n---\n(.*)", text, re.S)
    if not m:
        raise Failed("SKILL.md: no front matter")
    fields, key = {}, None
    for line in m.group(1).split("\n"):
        if re.match(r"\S", line):
            key, _, value = line.partition(":")
            fields[key.strip()] = value.strip()
        elif key:  # a folded or nested value
            fields[key] += " " + line.strip()
    return fields, m.group(2)


def check_manifest() -> list[str]:
    errors = []
    fields, body = front_matter(read(SKILL / "SKILL.md"))
    if fields.get("name") != SKILL.name:
        errors.append(f"SKILL.md: name is {fields.get('name')!r}, the folder is {SKILL.name!r}")
    if not re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*", fields.get("name", "")) or len(fields.get("name", "")) > 64:
        errors.append("SKILL.md: name must be lowercase letters, digits and hyphens, at most 64 characters")
    desc = fields.get("description", "")
    if not desc or len(desc) > 1024:
        errors.append(f"SKILL.md: description is {len(desc)} characters; it needs 1 to 1024")
    if len(fields.get("compatibility", "x")) > 500:
        errors.append("SKILL.md: compatibility is longer than 500 characters")
    for k in fields.keys() - ALLOWED_FRONT:
        errors.append(f"SKILL.md: front matter key {k!r} is not one of {sorted(ALLOWED_FRONT)}")
    if body.count("\n") > 500:
        errors.append(f"SKILL.md: {body.count(chr(10))} lines in the body; keep it under 500 and move detail to references/")
    return errors


def check_links() -> list[str]:
    """Every relative link of the package's Markdown files leads to a file of the package, and SKILL.md links
    every file of references/ and examples/."""
    errors, linked = [], set()
    for md in sorted(SKILL.rglob("*.md")):
        for target in re.findall(r"\]\(([^)\s]+)\)", re.sub(r"```.*?```", "", read(md), flags=re.S)):
            if re.match(r"([a-z][a-z0-9+.-]*:|#|//)", target):
                continue
            path = (md.parent / target.split("#")[0]).resolve()
            if not path.exists():
                errors.append(f"{md.relative_to(SKILL).as_posix()}: the link {target} leads nowhere")
            elif md.name == "SKILL.md":
                linked.add(path.relative_to(SKILL.resolve()).as_posix())
    for f in skill_files():
        if f.startswith(("references/", "examples/")) and f not in linked:
            errors.append(f"SKILL.md does not link {f}")
    return errors


def check_site_texts() -> list[str]:
    """The pages and the llms.txt of the site name every file of the package, so they cannot fall behind it."""
    errors = []
    for name in ("agents.html", "llms.txt", "llms.zh.txt"):
        text = read(SITE_SRC / name)
        errors += [f"site/src/{name} does not name {f}" for f in skill_files()
                   if not f.startswith("LICENSES/") and f != "LICENSE.md" and f not in text]
    return errors


def netlists() -> list[tuple[str, str, str, bool]]:
    """(where, file name, text, errors expected) of every netlist in the package: the code blocks marked `spice`
    or `spectre` in its Markdown files, and the files of examples/. A block marked `error` is expected to fail."""
    found = []
    for md in sorted(SKILL.rglob("*.md")):
        for i, m in enumerate(FENCE.finditer(read(md)), 1):
            words = m.group(2).split()
            if words and words[0] in ("spice", "spectre"):
                ext = ".scs" if words[0] == "spectre" else ".sp"
                line = read(md)[:m.start()].count("\n") + 2
                found.append((f"{md.relative_to(SKILL).as_posix()}:{line}", f"block{i}{ext}", m.group(3), "error" in words))
    for f in sorted((SKILL / "examples").glob("*")):
        if f.suffix in (".sp", ".scs", ".cir"):
            found.append((f"examples/{f.name}", f.name, read(f), False))
    return found


def check_netlists(rnl: Path) -> list[str]:
    errors = []
    with tempfile.TemporaryDirectory() as tmp:
        for where, name, text, expect_error in netlists():
            d = Path(tmp) / hashlib.sha256(where.encode()).hexdigest()[:8]
            d.mkdir()
            (d / name).write_text(text, encoding="utf-8", newline="\n")
            r = run_rnl(rnl, "check", str(d / name))
            m = COUNTS.search(r.stdout)
            if not m:
                errors.append(f"{where}: rnl check printed no summary:\n{r.stdout}{r.stderr}")
                continue
            n_err, n_warn = int(m.group(1)), int(m.group(2))
            if expect_error and n_err == 0:
                errors.append(f"{where}: marked `error`, but rnl check finds no error")
            elif not expect_error and (n_err or n_warn):
                errors.append(f"{where}: rnl check finds {n_err} errors and {n_warn} warnings:\n"
                              + "\n".join(l for l in r.stdout.splitlines() if l.startswith(("error", "warning"))))
        print(f"checked {len(netlists())} netlists with rnl check")
    return errors


def check(rnl: Path) -> list[str]:
    errors = check_manifest() + check_links() + check_site_texts()
    for path, text in generated(rnl).items():
        target = SKILL / path
        if not target.exists() or read(target) != text:
            errors.append(f"skills/rnl/{path}: not what its sources give; run python scripts/skill/build.py")
    extra = set(skill_files()) - {"SKILL.md", "LICENSE.md"} - set(generated(rnl)) - {
        f for f in skill_files() if f.startswith(("references/", "examples/"))}
    errors += [f"skills/rnl/{f}: not a file the package knows" for f in sorted(extra)]
    return errors + check_netlists(rnl)


# --- the published forms ----------------------------------------------------------------------------

def git_commit() -> str:
    r = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "HEAD"], capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else "unknown"


def git_time() -> int:
    r = subprocess.run(["git", "-C", str(ROOT), "log", "-1", "--format=%ct", "--", "skills/rnl", "scripts/skill",
                        "spec/rnl-0.3-guide.md", "spec/rnl-0.3-guide.zh.md"], capture_output=True, text=True)
    return int(r.stdout.strip()) if r.returncode == 0 and r.stdout.strip() else 1767225600  # 2026-01-01


def build_zip(dest: Path) -> None:
    """The package as a zip with the folder rnl/ in it, so that unpacking it where skills are kept leaves rnl/SKILL.md;
    the same bytes for the same files."""
    stamp = time.gmtime(git_time())[:6]
    with zipfile.ZipFile(dest, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for f in skill_files():
            info = zipfile.ZipInfo(f"{SKILL.name}/{f}", date_time=stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            zf.writestr(info, (SKILL / f).read_bytes())


def sha256(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def build_site(site: Path) -> None:
    """Add to a built site: skill/rnl-skill.zip, the files of the package as plain text under skill/rnl/, a manifest and
    the hashes, and the pointer for agents, llms.txt (English) and zh/llms.txt (Chinese)."""
    skill = site / "skill"
    shutil.rmtree(skill, ignore_errors=True)
    for f in skill_files():
        (skill / SKILL.name / f).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(SKILL / f, skill / SKILL.name / f)
    build_zip(skill / "rnl-skill.zip")
    manifest = {
        "name": SKILL.name, "standard": SERIES, "commit": git_commit(), "zip": SITE_URL + "skill/rnl-skill.zip",
        "files": [{"path": f, "url": f"{SITE_URL}skill/{SKILL.name}/{f}", "bytes": (SKILL / f).stat().st_size,
                   "sha256": sha256(SKILL / f)} for f in skill_files()],
    }
    (skill / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n", encoding="utf-8", newline="\n")
    sums = [f"{sha256(skill / 'rnl-skill.zip')}  rnl-skill.zip"] + [f"{e['sha256']}  {SKILL.name}/{e['path']}" for e in manifest["files"]]
    (skill / "SHA256SUMS.txt").write_text("\n".join(sums) + "\n", encoding="utf-8", newline="\n")
    for src, dest in (("llms.txt", site / "llms.txt"), ("llms.zh.txt", site / "zh" / "llms.txt")):
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(read(SITE_SRC / src).replace("{{site}}", SITE_URL).replace("{{github}}", GITHUB),
                        encoding="utf-8", newline="\n")
    print(f"wrote {skill} ({len(manifest['files'])} files and rnl-skill.zip), {site / 'llms.txt'} and {site / 'zh' / 'llms.txt'}")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--rnl", help="the rnl command (default: $RNL, target/debug/rnl, target/release/rnl, PATH)")
    ap.add_argument("--check", action="store_true", help="check the package and write nothing")
    ap.add_argument("--site", type=Path, help="a built site to add the package's downloads and llms.txt to")
    args = ap.parse_args(argv)
    for stream in (sys.stdout, sys.stderr):
        stream.reconfigure(errors="backslashreplace")
    try:
        if args.site and not args.check:
            build_site(args.site)
            return 0
        rnl = find_rnl(args.rnl)
        if args.check:
            errors = check(rnl)
            print("\n".join(errors) if errors else "ok: the package is whole and every netlist in it reads")
            return 1 if errors else 0
        write_generated(rnl)
    except Failed as e:
        print(e, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
