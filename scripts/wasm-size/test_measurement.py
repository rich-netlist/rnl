"""Small offline checks for byte accounting and safe, reproducible prototypes."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

import variants
import wasm_bytes

ROOT = Path(__file__).resolve().parents[2]


class MeasurementTests(unittest.TestCase):
    def test_section_and_function_accounting(self):
        data = b"\0asm\1\0\0\0" + b"\x0a\x04\x01\x02\x00\x0b" + b"\x00\x04\x01xAB"
        self.assertEqual(wasm_bytes.functions(data), [(0, 11, 14)])
        self.assertEqual(wasm_bytes.noncustom(data), data[:14])
        self.assertEqual(sum(end-start for _, start, _, end in wasm_bytes.sections(data))+8, len(data))
        with self.assertRaises(ValueError):
            wasm_bytes.sections(data[:-1])

    def test_uleb(self):
        self.assertEqual(wasm_bytes.uleb(b"\xe5\x8e\x26", 0), (624485, 3))

    def test_v0_symbol_crate_disambiguators(self):
        name = "<rnl_runtime[91ba1dec054bb563]::interp::Interp>::keyword"
        self.assertEqual(wasm_bytes.symbol_crate(name), "rnl_runtime")
        self.assertEqual(wasm_bytes.canonical_name(name), "<rnl_runtime::interp::Interp>::keyword")
        self.assertEqual(wasm_bytes.symbol_crate("core[0123456789abcdef]::fmt::write"), "core")
        self.assertEqual(wasm_bytes.symbol_crate("function[12]"), "<unattributed>")
        self.assertEqual(wasm_bytes.symbol_crate("<str>::replace::<alloc::string::String>"), "<unattributed>")

    def test_prototype_anchors_and_data_are_lossless(self):
        import json
        import shutil
        import zlib
        # No compiler, generated artifact, downloaded dependency or source-tree mutation.
        for name in variants.CODE:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                for rel in ("bindings/runtime-wasm/src/lib.rs", "crates/rnl-view/Cargo.toml", "crates/rnl-view/src/lib.rs", "crates/rnl-runtime/src/words.rs", "crates/rnl-runtime/data/words-0.2.json"):
                    (root/rel).parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(ROOT/rel, root/rel)
                shutil.copytree(ROOT/"libraries/standard", root/"libraries/standard")
                variants.apply(root, name)
                words = "crates/rnl-runtime/data/words-0.2.json"
                self.assertEqual(json.loads((root/words).read_text(encoding="utf-8")), json.loads((ROOT/words).read_text(encoding="utf-8")))
                self.assertEqual((root/words).read_bytes(), (ROOT/words).read_bytes())
                compact = root/"crates/rnl-runtime/data/size-words.json"
                if compact.exists():
                    self.assertEqual(json.loads(compact.read_text(encoding="utf-8")), json.loads((ROOT/words).read_text(encoding="utf-8")))
                compressed = root/"crates/rnl-view/src/size-bundle.zlib"
                if compressed.exists():
                    original = b"".join(p.read_bytes() for p in sorted((root/"libraries/standard").glob("*/*.rnl")))
                    self.assertEqual(zlib.decompress(compressed.read_bytes()), original)


if __name__ == "__main__":
    unittest.main()
