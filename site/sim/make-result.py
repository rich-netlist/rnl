"""Turns the AC result of a Spectre run of site/sim/ota5t-review-tb.sp into site/src/assets/ota-sim.js, the curve that the
introduction page shows beside the example.

    python site/sim/make-result.py RAW_AC_FILE
    python site/sim/make-result.py --restamp OLD_TESTBENCH

The run itself is not part of the build (the simulator is not in CI): the testbench is copied to a folder, a first line
"simulator lang=spice" is put in front of it, and

    spectre -64 tb.sp +escchars -format psfascii -raw tb.raw

writes tb.raw/frequencySweep.ac, the file this script reads (PSF ASCII). The script keeps the gain and the phase of
V(out) for the 1 V differential input of the testbench, the simulator's name and version, the analysis statement, and the
hash of the testbench's circuit lines; site/build.py checks that the circuit of the result is the present one.

The hash covers only the circuit lines: the lines that do not start with "*" and are not empty. The RNL records and
the other comments are left out, since the simulator reads them as comments and a new drawing does not change the
result. When only comments changed, --restamp OLD_TESTBENCH stamps the present result with the present testbench without
a new run: OLD_TESTBENCH is the testbench the result was made from (its hash, of the circuit lines or of the whole text as
the script stamped it before, must be the one in ota-sim.js), and its circuit lines must be those of the present one.
"""
import argparse
import hashlib
import json
import math
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TESTBENCH = ROOT / "site" / "sim" / "ota5t-review-tb.sp"
OUT = ROOT / "site" / "src" / "assets" / "ota-sim.js"
HEAD = ("/* The AC response of site/sim/ota5t-review-tb.sp from a Spectre run, made by site/sim/make-result.py; do not edit.\n"
        "   site/build.py checks that the circuit of the testbench is the one of the result (sha256 of its circuit lines). */\n")


def read(path):
    return Path(path).read_text(encoding="utf-8").replace("\r\n", "\n")


def circuit_lines(text):
    """The lines a simulator reads as the circuit: not empty and not a comment ("*"). site/build.py does the same."""
    return [line for line in text.split("\n") if line.strip() and not line.startswith("*")]


def circuit_hash(text):
    return hashlib.sha256("\n".join(circuit_lines(text)).encode("utf-8")).hexdigest()[:12]


def parse_ac(text):
    """[(freq, {node: complex})] of a PSF ASCII AC file."""
    header = text.split("\nVALUE\n", 1)
    version = re.search(r'"version" "([^"]+)"', header[0]).group(1)
    points = []
    for line in header[1].splitlines():
        m = re.match(r'"freq" (\S+)', line)
        if m:
            points.append((float(m.group(1)), {}))
            continue
        m = re.match(r'"([^"]+)" \((\S+) (\S+)\)', line)
        if m and points:
            points[-1][1][m.group(1)] = complex(float(m.group(2)), float(m.group(3)))
    return version, points


def write(result):
    OUT.write_text(HEAD + "window.RNLOtaSim = " + json.dumps(result, separators=(",", ":")) + ";\n", encoding="utf-8")


def restamp(old_path):
    """Stamp the present result with the present testbench, whose circuit lines are those of OLD_TESTBENCH, the one
    the result was made from."""
    result = json.loads(re.search(r"window\.RNLOtaSim = (.*);\s*$", OUT.read_text(encoding="utf-8"), re.S).group(1))
    old, text = read(old_path), read(TESTBENCH)
    if result["sha256"] not in (circuit_hash(old), hashlib.sha256(old.encode("utf-8")).hexdigest()[:12]):
        raise SystemExit("%s: not the testbench of the present result (sha256 %s)" % (old_path, result["sha256"]))
    if circuit_lines(old) != circuit_lines(text):
        raise SystemExit("the circuit lines of %s changed since the result was made: run the testbench again" % TESTBENCH)
    result["sha256"] = circuit_hash(text)
    write(result)
    print("restamped: sha256 of the circuit lines %s" % result["sha256"])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("raw", nargs="?")
    ap.add_argument("--restamp", metavar="OLD_TESTBENCH")
    args = ap.parse_args()
    if args.restamp:
        return restamp(args.restamp)
    if not args.raw:
        ap.error("give RAW_AC_FILE or --restamp OLD_TESTBENCH")
    version, points = parse_ac(read(args.raw))
    freq, gain, phase = [], [], []
    last = None
    for f, v in points:
        z = v["out"] / (v["inp"] - v["inn"])
        deg = math.degrees(math.atan2(z.imag, z.real))
        if last is not None:
            while deg - last > 180:
                deg -= 360
            while deg - last < -180:
                deg += 360
        last = deg
        freq.append(float("%.4g" % f))
        gain.append(round(20 * math.log10(abs(z)), 2))
        phase.append(round(deg, 1) + 0.0)
    text = read(TESTBENCH)
    result = {
        "simulator": "Spectre",
        "version": version,
        "analysis": next(line for line in text.split("\n") if line.startswith(".ac ")),
        "command": "spectre -64 tb.sp +escchars -format psfascii -raw tb.raw",
        "testbench": "site/sim/ota5t-review-tb.sp",
        "sha256": circuit_hash(text),
        "freq": freq, "gain_db": gain, "phase_deg": phase,
    }
    write(result)
    dc = gain[0]
    f3 = next((f for f, g in zip(freq, gain) if g <= dc - 3), None)
    f0 = next((f for f, g in zip(freq, gain) if g <= 0), None)
    print("version %s, %d points, DC gain %.2f dB, -3 dB at %s Hz, unity gain at %s Hz" % (version, len(freq), dc, f3, f0))


if __name__ == "__main__":
    main()
