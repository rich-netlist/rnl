"""Account for every WASM byte; attribute code through an unstripped companion.

LTO inlines across crates, so attribution is by surviving symbol, not perfect
source ownership. gzip is non-additive: zeroing ranges is a marginal diagnostic,
not a deployable transformation or an additive share of compressed size.
"""
import gzip
import hashlib
import json
import re
import subprocess
from collections import defaultdict


def uleb(data, at):
    n, shift = 0, 0
    while True:
        b = data[at]
        at += 1
        n |= (b & 127) << shift
        if b < 128:
            return n, at
        shift += 7
        if shift > 63:
            raise ValueError("Invalid LEB128")


def string(data, at):
    n, at = uleb(data, at)
    return data[at:at+n].decode("utf-8"), at+n


def sections(data):
    if data[:8] != b"\0asm\1\0\0\0":
        raise ValueError("Expected WASM v1")
    result, at = [], 8
    while at < len(data):
        start = at
        kind = data[at]
        size, at = uleb(data, at+1)
        end = at+size
        if end > len(data):
            raise ValueError("Truncated section")
        result.append((kind, start, at, end))
        at = end
    return result


def noncustom(data):
    return data[:8] + b"".join(data[start:end] for kind, start, _, end in sections(data) if kind)


def functions(data):
    result = []
    for kind, _, at, end in sections(data):
        if kind == 2:
            count, _ = uleb(data, at)
            if count:
                raise ValueError("This audit expects the binding's zero-import ABI")
        if kind == 10:
            count, at = uleb(data, at)
            for index in range(count):
                start = at
                size, at = uleb(data, at)
                result.append((index, start, at+size))
                at += size
            if at != end:
                raise ValueError("Code-section accounting mismatch")
    return result


def names(data):
    result = {}
    for kind, _, at, end in sections(data):
        if kind:
            continue
        name, at = string(data, at)
        if name != "name":
            continue
        while at < end:
            sub = data[at]
            size, at = uleb(data, at+1)
            stop = at+size
            if sub == 1:
                count, at = uleb(data, at)
                for _ in range(count):
                    index, at = uleb(data, at)
                    result[index], at = string(data, at)
            at = stop
    return result


def gz(data):
    return len(gzip.compress(data, compresslevel=9, mtime=0))


def canonical_name(name):
    # rustfilt exposes v0 crate disambiguators as crate[0123456789abcdef].
    # They are identity metadata, not a nested module or part of a crate name.
    return re.sub(r"\[[0-9a-f]{16}\]", "", name)


def symbol_crate(name):
    # Only the leading qualified owner is evidence. For <str>::replace::<T>,
    # 'replace' is a method, and a qualified T would be an argument, not its owner.
    # Leave primitive/anonymous owners unattributed rather than inventing crates.
    match = re.match(r"^<*([A-Za-z_][A-Za-z_0-9]*)::", canonical_name(name))
    return match.group(1) if match else "<unattributed>"


def audit(baseline, companion, root):
    release, named = baseline.read_bytes(), companion.read_bytes()
    identical = noncustom(release) == noncustom(named)
    # Changing Rust's strip option can change code layout. Never attach companion
    # names to release indices in that case. Attribute the companion's OWN
    # executable sections and disclose the measured difference to the release.
    data = noncustom(named)
    symbols = names(named)
    indices = sorted(symbols)
    text = "\n".join(symbols[i] for i in indices) + "\n"
    demangled = subprocess.check_output(["rustfilt"], input=text, text=True).splitlines()
    assert len(demangled) == len(indices)
    symbols = dict(zip(indices, demangled))
    groups, entries = defaultdict(list), []
    for index, start, end in functions(data):
        name = canonical_name(symbols.get(index, f"function[{index}]"))
        crate = symbol_crate(name)
        entry = {"index": index, "name": name, "crate": crate, "start": start,
                 "end": end, "raw": end-start, "standalone_gzip": gz(data[start:end])}
        entries.append(entry)
        groups[crate].append(entry)
    total_gzip = gz(data)
    def marginal(rows):
        copy = bytearray(data)
        for row in rows:
            copy[row["start"]:row["end"]] = bytes(row["end"]-row["start"])
        return total_gzip-gz(copy)
    by_crate = []
    for crate, rows in groups.items():
        body = b"".join(data[r["start"]:r["end"]] for r in rows)
        by_crate.append({"crate": crate, "functions": len(rows), "raw": len(body),
                         "raw_percent": 100*len(body)/len(data), "standalone_gzip": gz(body),
                         "zeroed_gzip_delta": marginal(rows)})
    top = sorted(entries, key=lambda r: r["raw"], reverse=True)[:40]
    for row in top:
        row["zeroed_gzip_delta"] = marginal([row])
    section_rows = []
    for kind, start, at, end in sections(release):
        section_rows.append({"id": kind, "name": string(release, at)[0] if kind == 0 else str(kind),
                             "raw": end-start, "standalone_gzip": gz(release[start:end])})
    embedded = []
    paths = list(sorted((root/"libraries/standard").glob("*/*.rnl")))
    paths += [root/"crates/rnl-runtime/data/words-0.2.json", root/"crates/rnl-render/fonts/noto-sans-sc.metrics"]
    for path in paths:
        content = path.read_bytes()
        count = release.count(content)
        embedded.append({"path": str(path.relative_to(root)), "raw": len(content),
                         "standalone_gzip": gz(content), "complete_occurrences": count,
                         "sha256": hashlib.sha256(content).hexdigest()})
    # Strings of fixtures included under #[cfg(test)] must not be bundled in production.
    fixtures = []
    for folder in ("tests/conformance", "tests/viewer/inputs"):
        for path in sorted((root/folder).rglob("*")):
            if path.suffix in (".sp", ".scs"):
                b = path.read_bytes()
                if len(b) > 32 and b in release:
                    fixtures.append(str(path.relative_to(root)))
    semantic = []
    for label, pattern in (("serde / JSON (including derives)", r"serde|Serialize|Deserialize"),
                           ("formatting", r"core::fmt|alloc::fmt|flt2dec|num::flt"),
                           ("Unicode functions", r"unicode"),
                           ("panic / errors", r"panic|backtrace|::error")):
        rows = [r for r in entries if re.search(pattern, r["name"])]
        semantic.append({"label": label, "raw": sum(r["raw"] for r in rows), "functions": len(rows)})
    return {"raw": len(release), "gzip": gz(release), "noncustom_identical": identical,
            "attribution_raw": len(data), "attribution_gzip": total_gzip,
            "attribution_raw_delta": len(data)-len(noncustom(release)),
            "function_count": len(entries), "sections": section_rows,
            "crates": sorted(by_crate, key=lambda r: r["raw"], reverse=True),
            "top_functions": top, "functions": entries, "embedded": embedded,
            "semantic_slices_overlapping": semantic,
            "whole_test_inputs_found": fixtures,
            "caveat": "Crate/function attribution is for the named companion's own non-custom sections; see noncustom_identical and attribution_raw_delta before treating it as the release. Symbols survive fat LTO; generic code belongs to its surviving symbol, not every crate named in type arguments. Semantic slices overlap. zeroed_gzip_delta is non-additive and does not represent safe removal."}
