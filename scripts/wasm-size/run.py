"""CI-only, fixed-source WASM size investigation. Artifacts survive partial failures."""
import argparse
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time
import zlib

import variants

HERE = Path(__file__).resolve().parent
BASE = "9ee4fe83d7401239472b4b9d5a568b3bb2d2422e"


def save(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def command(args, cwd, log, env=None, required=True, timeout=900):
    with log.open("w", encoding="utf-8") as f:
        f.write(json.dumps([str(a) for a in args]) + "\n")
        f.flush()
        try:
            p = subprocess.run([str(a) for a in args], cwd=cwd, env=env, stdout=f,
                               stderr=subprocess.STDOUT, timeout=timeout)
            code = p.returncode
        except subprocess.TimeoutExpired:
            code = 124
            f.write("\nTIMEOUT\n")
    if code and required:
        raise RuntimeError(f"Command failed ({code}): see {log}")
    return code


def size(path):
    import brotli
    data = path.read_bytes()
    return {"raw": len(data), "gzip": len(gzip.compress(data, compresslevel=9, mtime=0)),
            "brotli": len(brotli.compress(data, quality=11)), "sha256": hashlib.sha256(data).hexdigest()}


def core_tests(root, out, env=None):
    # The untouched solver's full suite was collected separately. Exercise all
    # libraries actually linked by this WASM and the native conformance snapshots.
    commands = [
        ["cargo", "test", "--locked", "--workspace", "--exclude", "rnl-solver", "--exclude", "rnl-cli"],
        ["cargo", "test", "--locked", "-p", "rnl-cli", "--test", "conformance"],
    ]
    return [command(cmd, root, out/f"core-native-{i}.log", env, required=False, timeout=1200)
            for i, cmd in enumerate(commands)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--group", choices=("profiles", "code", "binaryen", "analysis"), required=True)
    parser.add_argument("--only", default="", help="Comma-separated variant names; baseline is always included")
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted":
        raise SystemExit("Builds and measurement are restricted to GitHub-hosted CI runners")
    root, out = args.source.resolve(), args.out.resolve()
    if root == HERE.parents[1] or out == root or root in out.parents:
        raise SystemExit("Use separate disposable source and artifact directories")
    out.mkdir(parents=True, exist_ok=True)
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    if actual != BASE or subprocess.check_output(["git", "status", "--porcelain"], cwd=root):
        raise SystemExit("Source must be a clean checkout of the frozen public baseline")
    tracked = {p: (root/p).read_bytes() for p in subprocess.check_output(["git", "ls-files"], cwd=root, text=True).splitlines()}
    receipt = {"source_sha": actual, "harness_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=HERE, text=True).strip(),
               "group": args.group, "platform": platform.platform(), "cpu": platform.processor(),
               "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
               "cargo": subprocess.check_output(["cargo", "-V"], text=True).strip(),
               "node": subprocess.check_output(["node", "--version"], text=True).strip(),
               "python": platform.python_version(), "zlib": zlib.ZLIB_RUNTIME_VERSION,
               "run_id": os.environ.get("GITHUB_RUN_ID"), "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
               "compression": "gzip level 9, mtime=0; Brotli quality 11", "variants": []}
    save(out/"results.json", receipt)
    command(["python", "scripts/packages.py", "download"], root, out/"packages.log")
    command(["cargo", "tree", "--locked", "--target", "wasm32-unknown-unknown", "-p", "rnl-runtime-wasm", "-e", "normal"], root, out/"dependency-tree.txt")
    command(["rustc", "--print", "cfg", "--target", "wasm32-unknown-unknown"], root, out/"target-cfg.txt")
    # Reuse the repository's deterministic performance-input generator, without solving.
    spec = importlib.util.spec_from_file_location("performance_inputs", root/"scripts/solver-performance/prepare_inputs.py")
    generator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(generator)
    perf = []
    for family, n in (("chain", 1024), ("diffmirror", 128)):
        entry = f"/size/{family}-{n}.sp"
        data = generator.generate(family, n)
        perf.append({"name": f"{family}-{n}", "input_sha256": hashlib.sha256(data).hexdigest(),
                     "request": {"entry": entry, "files": {entry: data.decode()}, "primitives": True}})
    save(out/"performance-inputs.json", perf)
    native = root/"target/debug/rnl"
    if args.group == "profiles":
        receipt["core_native_tests"] = core_tests(root, out)
        save(out/"results.json", receipt)
        command(["cargo", "build", "--locked", "-p", "rnl-cli"], root, out/"native-build.log")
        # Existing harness checks are run as checks, never mislabelled as solver timing.
        manifest = json.loads((root/"tests/performance/manifest.json").read_text())
        for key in ("historical_source", "library_source"):
            command(["git", "fetch", "--no-tags", "origin", manifest[key]["commit"]], root, out/(key+"-fetch.log"))
        for test in ("test_inputs.py", "test_benchmark.py", "test_work.py"):
            command(["python", "scripts/solver-performance/"+test], root, out/test.replace(".py", ".log"))
    names = [n for n, _ in variants.PROFILES] if args.group == "profiles" else variants.CODE if args.group == "code" else ["baseline", "symbols"] if args.group == "analysis" else ["baseline", "opt-z"]
    if args.only:
        selected = set(args.only.split(",")) | {"baseline"}
        if not selected <= set(names):
            raise SystemExit("Unknown variants: " + repr(selected-set(names)))
        names = [name for name in names if name in selected]
    profile_map = dict(variants.PROFILES)
    for name in names:
        print(f"Measuring {args.group}/{name}", flush=True)
        dest = out/name
        dest.mkdir(exist_ok=True)
        row = {"name": name, "ablative": name in variants.ABLATIONS}
        receipt["variants"].append(row)
        try:
            overrides = variants.apply(root, name)
            overrides.update(profile_map.get(name, {}))
            env = dict(os.environ)
            env.update({"CARGO_PROFILE_WASM_"+k: v for k, v in overrides.items()})
            row["profile_overrides"] = overrides
            if name in ("compressed-packages", "combined", "combined-value"):
                # Lock only the declared prototype dependency; the baseline lock is restored afterwards.
                command(["cargo", "update", "--workspace"], root, dest/"prototype-lock.log", env)
                shutil.copy2(root/"Cargo.lock", dest/"Cargo.lock")
            patch = subprocess.check_output(["git", "diff", "--binary"], cwd=root)
            (dest/"prototype.patch").write_bytes(patch)
            for generated in ("size-bundle.rs", "size-bundle.zlib"):
                p = root/"crates/rnl-view/src"/generated
                if p.exists():
                    shutil.copy2(p, dest/generated)
            compact = root/"crates/rnl-runtime/data/size-words.json"
            if compact.exists():
                shutil.copy2(compact, dest/compact.name)
            started = time.monotonic()
            command(["node", "bindings/runtime-wasm/build.mjs", "--out", dest], root, dest/"build.log", env)
            row["build_seconds"] = time.monotonic()-started
            wasm = dest/"rnl_runtime_wasm.wasm"
            row.update(size(wasm))
            if name == "baseline":
                command(["node", HERE/"verify.mjs", "corpus", root, wasm, wasm, out/"corpus.json"], root, dest/"corpus.log")
            command(["node", HERE/"verify.mjs", "check", root, out, wasm, dest/"equality.json"], root, dest/"equality.log")
            equality = json.loads((dest/"equality.json").read_text())
            row["equality"] = {k: equality[k] for k in ("checked", "differing", "byte_identical")}
            if name == "baseline" and not equality["byte_identical"]:
                raise RuntimeError("Baseline self-replay failed: the corpus is not valid evidence")
            if equality["byte_identical"] and args.group != "analysis":
                command(["node", HERE/"verify.mjs", "benchmark", root, out, wasm, dest/"timings.json"], root, dest/"timings.log")
                row["js_tests_exit"] = command(["node", "bindings/runtime-wasm/tests/core.mjs", dest], root, dest/"core-tests.log", required=False)
            if args.group == "profiles" and name in ("baseline", "opt-z"):
                row["native_equality"] = []
                for extra in ([], ["--view", "--primitives", "--request", '{"primitives":true}'], ["--snapshot"]):
                    code = command(["node", "bindings/runtime-wasm/tests/equality.mjs", "--rnl", native, "--dist", dest, *extra], root, dest/f"native-equality-{len(row['native_equality'])}.log", required=False)
                    row["native_equality"].append(code)
            if args.group == "code" and name in ("compact-words", "compressed-packages", "json-value-output", "combined", "combined-value") and equality["byte_identical"]:
                row["core_native_tests"] = core_tests(root, dest, env)
            if name == "symbols":
                spec = importlib.util.spec_from_file_location("wasm_inspect", HERE/"wasm_bytes.py")
                audit_module = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(audit_module)
                save(dest/"breakdown.json", audit_module.audit(out/"baseline/rnl_runtime_wasm.wasm", wasm, root))
                for tool, extra in (("top", ["-n", "100"]), ("dominators", ["-d", "4", "-r", "100"]), ("monos", [])):
                    row["twiggy_"+tool] = command(["twiggy", tool, *extra, wasm], root, dest/f"twiggy-{tool}.txt", required=False, timeout=180)
            row["status"] = "MEASURED"
        except Exception as error:
            row["status"] = "ERROR"
            row["error"] = str(error)
            print(str(error), flush=True)
        finally:
            for path, content in tracked.items():
                p = root/path
                if p.read_bytes() != content:
                    p.write_bytes(content)
            for generated in ("size-bundle.rs", "size-bundle.zlib"):
                (root/"crates/rnl-view/src"/generated).unlink(missing_ok=True)
            (root/"crates/rnl-runtime/data/size-words.json").unlink(missing_ok=True)
            save(out/"results.json", receipt)
    if args.group == "binaryen":
        command(["wasm-opt", "--version"], root, out/"binaryen-version.txt")
        command(["cargo", "build", "--locked", "-p", "rnl-cli"], root, out/"native-build.log")
        for base in ("baseline", "opt-z"):
            for suffix, flags in (("Os", ["-Os"]), ("Oz", ["-Oz"]), ("Oz-no-inline", ["-Oz", "--always-inline-max-function-size", "0", "--one-caller-inline-max-function-size", "0", "--flexible-inline-max-function-size", "0"])):
                name = base+"-"+suffix
                dest = out/name
                dest.mkdir(exist_ok=True)
                row = {"name": name, "wasm_opt_flags": flags, "ablative": False}
                receipt["variants"].append(row)
                try:
                    wasm = dest/"rnl_runtime_wasm.wasm"
                    features = ["--enable-bulk-memory", "--enable-sign-ext", "--enable-nontrapping-float-to-int", "--enable-multivalue", "--enable-reference-types", "--enable-mutable-globals"]
                    command(["wasm-opt", out/base/"rnl_runtime_wasm.wasm", *flags, *features, "-o", wasm], root, dest/"build.log")
                    row.update(size(wasm))
                    command(["node", HERE/"verify.mjs", "check", root, out, wasm, dest/"equality.json"], root, dest/"equality.log")
                    equality = json.loads((dest/"equality.json").read_text())
                    row["equality"] = {k: equality[k] for k in ("checked", "differing", "byte_identical")}
                    if equality["byte_identical"]:
                        command(["node", HERE/"verify.mjs", "benchmark", root, out, wasm, dest/"timings.json"], root, dest/"timings.log")
                        for glue in (out/base).iterdir():
                            if glue.suffix in (".js", ".ts") or glue.name == "package.json":
                                shutil.copy2(glue, dest/glue.name)
                        row["js_tests_exit"] = command(["node", "bindings/runtime-wasm/tests/core.mjs", dest], root, dest/"core-tests.log", required=False)
                        if suffix == "Oz-no-inline":
                            row["native_equality"] = []
                            for extra in ([], ["--view", "--primitives", "--request", '{"primitives":true}'], ["--snapshot"]):
                                code = command(["node", "bindings/runtime-wasm/tests/equality.mjs", "--rnl", native, "--dist", dest, *extra], root, dest/f"native-equality-{len(row['native_equality'])}.log", required=False)
                                row["native_equality"].append(code)
                    row["status"] = "MEASURED"
                except Exception as error:
                    row.update(status="ERROR", error=str(error))
                save(out/"results.json", receipt)
    # Do not make an unsuccessful collection green; rejected ablations are expected.
    if any(r["status"] == "ERROR" for r in receipt["variants"]):
        raise SystemExit("One or more measurements failed; inspect preserved artifacts")
    exits = list(receipt.get("core_native_tests", []))
    for row in receipt["variants"]:
        exits += row.get("core_native_tests", []) + row.get("native_equality", [])
        exits += [row.get("js_tests_exit", 0)]
    if any(exits):
        raise SystemExit("One or more validation suites failed; inspect preserved artifacts")


if __name__ == "__main__":
    main()
