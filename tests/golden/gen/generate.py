#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11,<3.14"
# dependencies = [
#   "numpy==2.5.4",
#   "numpy-financial==1.1.0",
#   "pyxirr==0.10.8",
#   "pandas==3.0.6",
#   "scipy==1.18.1",
#   "quantstats==0.0.86",
#   "empyrical-reloaded==0.5.12",
#   "statsmodels==0.15.0",
#   "QuantLib==1.44",
#   "ffn==1.4.1",
#   "PyPortfolioOpt==1.6.0",
# ]
# ///
"""Write finance-rs golden fixtures from the pinned Python references.

Run from anywhere:  uv run --script tests/golden/gen/generate.py
Each <module>_cases.py beside this file exposes `cases() -> dict` with
"reference" (library -> version), "tolerance" (fn -> {"atol","rtol"}) and
"cases" (a list of {"fn", "args", "expected"}). `expected` is a number, a
list of numbers, or null when the reference raises / returns NaN (the Rust
side must then return an error). Output is deterministic (fixed seeds,
sorted keys) so re-running on any box yields a byte-identical diff.
"""

import importlib
import json
import math
import subprocess
import sys
from importlib.metadata import version
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE.parent
MODULES = ["tvm", "cashflow", "daycount", "perf", "stats", "solve", "tvm_extras",
           "cashflow_extras", "rates", "perf_extra", "pm", "calendars", "funds", "portfolio"]


def clean(x):
    """NaN/inf -> None, numpy scalars -> float, recursively."""
    if isinstance(x, dict):
        return {k: clean(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [clean(v) for v in x]
    if hasattr(x, "item") and not isinstance(x, (str, bytes)):
        x = x.item()
    if isinstance(x, float) and not math.isfinite(x):
        return None
    return x


def main() -> None:
    sys.path.insert(0, str(HERE))
    only = set(sys.argv[1:])
    for name in MODULES:
        if only and name not in only:
            continue
        mod = importlib.import_module(f"{name}_cases")
        doc = mod.cases()
        doc["reference"] = {
            lib: ("closed form" if lib == "closed form" else version(lib))
            for lib in doc["reference"]
        }
        doc["generator"] = "tests/golden/gen/generate.py"
        path = OUT / f"{name}.json"
        path.write_text(json.dumps(clean(doc), indent=1, sort_keys=True) + "\n")
        print(f"{path.name}: {len(doc['cases'])} cases")
    if not only or "portfolio" in only:
        subprocess.run(["uv", "run", "--script", str(HERE / "portfolio_pyfolio.py")], check=True)


if __name__ == "__main__":
    main()
