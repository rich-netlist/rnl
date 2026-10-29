"""Disposable prototypes, applied only to the CI source checkout, never the product tree.

Every edit is recorded as a patch in the artifact. Ablations deliberately remove
API capabilities and are upper bounds, not deployable recommendations.
"""
import json
from pathlib import Path
import zlib


PROFILES = [
    ("baseline", {}),
    ("opt-z", {"OPT_LEVEL": "z"}),
    ("panic-abort", {"PANIC": "abort"}),
    ("lto-thin", {"LTO": "thin"}),
    ("lto-off", {"LTO": "off"}),
    ("cgu-16", {"CODEGEN_UNITS": "16"}),
    ("symbols", {"STRIP": "none"}),
    ("debug-stripped", {"DEBUG": "2"}),
    ("debug-unstripped", {"DEBUG": "2", "STRIP": "none"}),
]
CODE = [
    "baseline", "compact-words", "compressed-packages", "json-value-output",
    "direct-json-input", "no-write", "no-snapshot", "no-catalog", "read-only",
    "combined", "combined-value",
]
ABLATIONS = {"no-write", "no-snapshot", "no-catalog", "read-only"}


def replace(path, old, new):
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise ValueError(f"Expected exactly one prototype anchor in {path}: {old[:70]}")
    path.write_text(text.replace(old, new), encoding="utf-8", newline="\n")


def apply(root, name):
    """Return profile overrides. The caller restores the disposable checkout afterwards."""
    binding = root / "bindings/runtime-wasm/src/lib.rs"
    if name in ("compact-words", "combined", "combined-value"):
        words = root / "crates/rnl-runtime/data/words-0.2.json"
        value = json.loads(words.read_text(encoding="utf-8"))
        words.write_text(json.dumps(value, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    if name in ("compressed-packages", "combined", "combined-value"):
        # A measured extra runtime dependency, not proposed as a production change.
        manifest = root / "crates/rnl-view/Cargo.toml"
        replace(manifest, "[dependencies]\n", '[dependencies]\nminiz_oxide = "=0.8.9"\n')
        payload, ranges = bytearray(), []
        directory = root / "libraries/standard"
        for path in sorted(directory.glob("*/*.rnl")):
            content = path.read_bytes()
            ranges.append((path.relative_to(directory).as_posix(), len(payload), len(content)))
            payload.extend(content)
        (root / "crates/rnl-view/src/size-bundle.zlib").write_bytes(zlib.compress(payload, 9))
        declarations = 'const BUNDLED_DATA: &[u8] = include_bytes!("size-bundle.zlib");\n'
        declarations += 'const BUNDLED_RANGES: &[(&str, usize, usize)] = &[\n'
        declarations += ''.join(f'    ({json.dumps(n)}, {start}, {length}),\n' for n, start, length in ranges)
        declarations += '];\n'
        (root / "crates/rnl-view/src/size-bundle.rs").write_text(declarations, encoding="utf-8")
        source = root / "crates/rnl-view/src/lib.rs"
        replace(source, 'include!(concat!(env!("OUT_DIR"), "/bundled.rs"));',
                '#[cfg(test)]\ninclude!(concat!(env!("OUT_DIR"), "/bundled.rs"));\ninclude!("size-bundle.rs");')
        replace(source, '    for &(path, text) in BUNDLED {', '''    static DATA: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let data = DATA.get_or_init(|| miniz_oxide::inflate::decompress_to_vec_zlib(BUNDLED_DATA).unwrap());
    for &(path, start, len) in BUNDLED_RANGES {
        let text = std::str::from_utf8(&data[start..start + len]).unwrap();''')
    if name in ("json-value-output", "combined-value"):
        replace(binding, 'serde_json::to_string(&respond(request)).expect("a response has only string keys")',
                'serde_json::to_string(&serde_json::to_value(respond(request)).expect("string keys"))'
                '.expect("a response has only string keys")')
    if name == "direct-json-input":
        text = binding.read_text(encoding="utf-8")
        text = text.replace("serde_json::from_value(value)", "serde_json::from_slice(request)")
        import re
        text = re.sub(r"serde_json::from_value::<([^>]+)>\(value\)",
                      r"serde_json::from_slice::<\1>(request)", text)
        binding.write_text(text, encoding="utf-8")
    if name in ("no-write", "read-only"):
        replace(binding, '''        return match serde_json::from_value(value) {
            Ok(w) => write(w),
            Err(e) => invalid(e),
        };''', '        return Response::Error("measurement: write omitted".into());')
    if name in ("no-catalog", "read-only"):
        replace(binding, '''        return match serde_json::from_value(value) {
            Ok(c) => catalog(c),
            Err(e) => invalid(e),
        };''', '        return Response::Error("measurement: catalog omitted".into());')
    if name in ("no-snapshot", "read-only"):
        text = binding.read_text(encoding="utf-8")
        start = text.index("    if let Some(p) = request.parts {")
        end = text.index("    if request.lazy_unplaced {", start)
        text = text[:start] + '    if request.parts.is_some() { return Response::Error("measurement: snapshot omitted".into()); }\n' + text[end:]
        binding.write_text(text, encoding="utf-8")
    return {"OPT_LEVEL": "z"} if name.startswith("combined") else {}
