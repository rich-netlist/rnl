"""Summarize downloaded CI evidence without compiling, executing WASM or installing tools."""
import argparse
import json
from pathlib import Path
import statistics


def summarize(directories):
    campaigns, rows, breakdown = [], [], None
    for directory in directories:
        evidence = json.loads((directory/"results.json").read_text(encoding="utf-8"))
        identity = {k: v for k, v in evidence.items() if k != "variants"}
        campaigns.append(identity)
        for original in evidence["variants"]:
            row = dict(original, group=evidence["group"], run_id=evidence["run_id"], run_attempt=evidence["run_attempt"])
            timings = directory/row["name"]/"timings.json"
            if timings.exists():
                data = json.loads(timings.read_text(encoding="utf-8"))
                row["runtime"] = data
                ratios = [r["candidate_ms"]/r["baseline_ms"] for r in data["results"]]
                row["runtime_median_ratio"] = statistics.median(ratios)
                row["runtime_max_ratio"] = max(ratios)
            rows.append(row)
        path = directory/"symbols/breakdown.json"
        if path.exists():
            breakdown = json.loads(path.read_text(encoding="utf-8"))
            # Complete symbol list remains in the linked artifact; keep the summary small.
            breakdown.pop("functions", None)
    return {"campaigns": campaigns, "variants": rows, "breakdown": breakdown}


def table(value):
    lines = ["| Group / variant | Raw bytes | gzip-9 | Brotli-11 | ABI differences | Runtime ratio |",
             "| --- | ---: | ---: | ---: | ---: | ---: |"]
    for row in value["variants"]:
        equality = row.get("equality", {})
        difference = f"{equality['differing']}/{equality['checked']}" if equality else row["status"]
        ratio = f"{row['runtime_median_ratio']:.3f}x" if "runtime_median_ratio" in row else "not run"
        cells = [row["group"]+" / "+row["name"], *(f"{row[k]:,}" if k in row else "—" for k in ("raw", "gzip", "brotli")), difference, ratio]
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("directories", type=Path, nargs="+")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    value = summarize(args.directories)
    args.out.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(table(value))
