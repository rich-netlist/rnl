"""Turns the AC result of a Spectre run of site/sim/ota5t-review-tb.sp into site/src/assets/ota-sim.js, the curve that the
introduction page shows beside the example.

    python site/sim/make-result.py RAW_AC_FILE   [--log SPECTRE_LOG]

The run itself is not part of the build (the simulator is not in CI): the testbench is copied to a folder, a first line
"simulator lang=spice" is put in front of it, and

    spectre -64 tb.sp +escchars -format psfascii -raw tb.raw

writes tb.raw/frequencySweep.ac, the file this script reads (PSF ASCII). The script keeps the gain and the phase of
V(out) for the 1 V differential input of the testbench, the simulator's name and version, the analysis statement, and the
hash of the testbench; site/build.py checks that the testbench of the result is the present one.
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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("raw")
    args = ap.parse_args()
    version, points = parse_ac(Path(args.raw).read_text(encoding="utf-8"))
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
    text = TESTBENCH.read_text(encoding="utf-8").replace("\r\n", "\n")
    result = {
        "simulator": "Spectre",
        "version": version,
        "analysis": next(line for line in text.split("\n") if line.startswith(".ac ")),
        "command": "spectre -64 tb.sp +escchars -format psfascii -raw tb.raw",
        "testbench": "site/sim/ota5t-review-tb.sp",
        "sha256": hashlib.sha256(text.encode("utf-8")).hexdigest()[:12],
        "freq": freq, "gain_db": gain, "phase_deg": phase,
    }
    OUT.write_text("/* The AC response of site/sim/ota5t-review-tb.sp from a Spectre run, made by site/sim/make-result.py; do not edit.\n"
                   "   site/build.py checks that the testbench is the one of the result (sha256). */\n"
                   "window.RNLOtaSim = " + json.dumps(result, separators=(",", ":")) + ";\n", encoding="utf-8")
    dc = gain[0]
    f3 = next((f for f, g in zip(freq, gain) if g <= dc - 3), None)
    f0 = next((f for f, g in zip(freq, gain) if g <= 0), None)
    print("version %s, %d points, DC gain %.2f dB, -3 dB at %s Hz, unity gain at %s Hz" % (version, len(freq), dc, f3, f0))


if __name__ == "__main__":
    main()
