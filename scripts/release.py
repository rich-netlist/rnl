"""Validate release versions, package the CLI, smoke-test extracted binaries and compare builds.

Uses only Python 3.11's standard library. All builds are done by the calling workflow.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
TARGETS = ("x86_64-pc-windows-msvc", "x86_64-unknown-linux-musl",
           "aarch64-apple-darwin", "x86_64-apple-darwin")
SEMVER = re.compile(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*)?")


def versions(root=ROOT):
    core = tomllib.loads((root / "Cargo.toml").read_text("utf-8"))["workspace"]["package"]["version"]
    viewer = tomllib.loads((root / "apps/desktop/Cargo.toml").read_text("utf-8"))["package"]["version"]
    config = json.loads((root / "apps/desktop/tauri.conf.json").read_text("utf-8"))["version"]
    if viewer != config:
        raise ValueError(f"Viewer manifest version {viewer} differs from Tauri version {config}")
    for version in (core, viewer):
        if not SEMVER.fullmatch(version):
            raise ValueError(f"Invalid release version: {version}")
    return {"core": core, "viewer": viewer}


def validate(line, tag=None, require_notes=False, root=ROOT):
    version = versions(root)[line]
    if tag and tag != f"{line}-v{version}":
        raise ValueError(f"Tag {tag} does not match {line} version {version}")
    notes = root / ("release-notes/core" if line == "core" else "apps/desktop/release-notes") / f"{version}.md"
    if require_notes and (not notes.is_file() or not notes.read_text("utf-8").strip()):
        raise ValueError(f"Missing release notes: {notes}")
    return version


def sums(directory):
    directory = Path(directory)
    files = sorted(p for p in directory.iterdir() if p.is_file() and p.name != "SHA256SUMS.txt")
    if not files:
        raise ValueError(f"No release files in {directory}")
    (directory / "SHA256SUMS.txt").write_text("".join(
        f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in files), encoding="utf-8", newline="\n")


def package(exe, target, out):
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        stage = Path(tmp)
        shutil.copy2(exe, stage / ("rnl.exe" if "windows" in target else "rnl"))
        shutil.copy2(ROOT / "LICENSE", stage / "LICENSE")
        (stage / "LICENSES").mkdir()
        for name in ("CC0-1.0.txt", "OFL-1.1.txt"):
            shutil.copy2(ROOT / "LICENSES" / name, stage / "LICENSES" / name)
        (stage / "NOTICE.txt").write_text(
            "RNL code: MIT. Bundled standard packages: CC0-1.0.\n"
            "Noto Sans SC font metrics: Copyright 2014-2021 Adobe (http://www.adobe.com/).\n"
            "Noto is a trademark of Google Inc. Licensed under the SIL Open Font License 1.1.\n",
            encoding="utf-8", newline="\n")
        files = sorted(p for p in stage.rglob("*") if p.is_file())
        if "windows" in target:
            dest = out / f"rnl-{target}.zip"
            with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as archive:
                for path in files:
                    archive.write(path, path.relative_to(stage))
        else:
            dest = out / f"rnl-{target}.tar.gz"
            with tarfile.open(dest, "w:gz") as archive:
                for path in files:
                    archive.add(path, arcname=str(path.relative_to(stage)))
    sums(out)
    print(dest)


def run(exe, *args):
    result = subprocess.run([str(exe), *map(str, args)], cwd=ROOT, capture_output=True)
    return result.returncode, result.stdout, result.stderr


def smoke(exe, version, commit):
    code, stdout, stderr = run(exe, "--version")
    assert (code, stdout.strip(), stderr) == (0, f"rnl {version} ({commit})".encode(), b""), (code, stdout, stderr)
    case = ROOT / "tests/conformance/cases/01-rc-lowpass.sp"
    code, stdout, stderr = run(exe, "check", case)
    assert code == 0 and b"0 errors, 0 warnings" in stdout and not stderr, (code, stdout, stderr)
    code, stdout, stderr = run(exe, "check", ROOT / "tests/conformance/cases/12-diagnostics.sp")
    assert code == 1 and b"error " in stdout and b"no-such-package" in stdout, (code, stdout, stderr)
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp)
        result = run(exe, "render", case, "-o", out / "svg")
        assert result[0] == 0, result
        svgs = list((out / "svg").glob("*.svg"))
        assert svgs and all(b"<svg" in p.read_bytes() for p in svgs)
        result = run(exe, "solve", case, "-o", out / "solved", "--solver-options", "time_limit=none")
        assert result[0] == 0 and (out / "solved" / case.name).is_file(), result
    print(f"Smoke passed: {exe}")


def compare(exe, reference):
    cases = sorted(p for folder in ("cases", "cases-0.3")
                   for p in (ROOT / "tests/conformance" / folder).iterdir() if p.suffix in (".sp", ".scs"))
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "svg"
        for case in cases:
            for command in ("check", "view", "render"):
                args = [command, case] + (["-o", out] if command == "render" else [])
                def capture(binary):
                    if out.exists():
                        shutil.rmtree(out)
                    result = run(binary, *args)
                    files = {str(p.relative_to(out)): p.read_bytes() for p in out.rglob("*") if p.is_file()}
                    return result, files
                assert capture(exe) == capture(reference), f"Different output: {command} {case}"
    print(f"Byte-identical check/view/render results, exit codes and SVGs for {len(cases)} conformance files")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("validate")
    check.add_argument("line", choices=("core", "viewer"))
    check.add_argument("--tag")
    check.add_argument("--require-notes", action="store_true")
    pack = commands.add_parser("package")
    pack.add_argument("--exe", type=Path, required=True)
    pack.add_argument("--target", choices=TARGETS, required=True)
    pack.add_argument("--out", type=Path, required=True)
    test = commands.add_parser("smoke")
    test.add_argument("--exe", type=Path, required=True)
    test.add_argument("--version", required=True)
    test.add_argument("--commit", required=True)
    diff = commands.add_parser("compare")
    diff.add_argument("--exe", type=Path, required=True)
    diff.add_argument("--reference", type=Path, required=True)
    checksums = commands.add_parser("sums")
    checksums.add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.command == "validate":
        version = validate(args.line, args.tag, args.require_notes)
        print(version)
        if os.environ.get("GITHUB_OUTPUT"):
            with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
                output.write(f"version={version}\n")
    elif args.command == "package":
        package(args.exe, args.target, args.out)
    elif args.command == "smoke":
        smoke(args.exe.resolve(), args.version, args.commit)
    elif args.command == "compare":
        compare(args.exe.resolve(), args.reference.resolve())
    else:
        sums(args.directory)


if __name__ == "__main__":
    main()
