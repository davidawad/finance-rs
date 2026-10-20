#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11,<3.14"
# dependencies = ["numpy==2.2.6", "pandas==2.2.3", "pyfolio-reloaded==0.9.9"]
# ///
"""Generate the Pyfolio turnover value used by the portfolio parity test."""

import json
from pathlib import Path

import pandas as pd
import pyfolio.timeseries as pf


HERE = Path(__file__).resolve().parent
OUT = HERE.parent


def main():
    dates = pd.to_datetime(["2024-01-02", "2024-01-03"])
    positions = pd.DataFrame(
        {"A": [50.0, 40.0], "B": [50.0, 60.0], "cash": [0.0, 0.0]},
        index=dates,
    )
    transactions = pd.DataFrame(
        {"amount": [-1.0, 1.0], "price": [10.0, 10.0]},
        index=pd.to_datetime(["2024-01-03", "2024-01-03"]),
    )
    turnover = float(pf.get_turnover(positions, transactions).iloc[-1])
    doc = {
        "reference": {"pyfolio-reloaded": "0.9.9"},
        "tolerance": {"turnover": {"atol": 1e-12, "rtol": 1e-12}},
        "cases": [
            {
                "fn": "turnover",
                "args": {"weights_before": [0.5, 0.5], "weights_after": [0.4, 0.6]},
                "expected": turnover,
            }
        ],
        "generator": "tests/golden/gen/portfolio_pyfolio.py",
    }
    (OUT / "portfolio_pyfolio.json").write_text(
        json.dumps(doc, indent=1, sort_keys=True) + "\n"
    )
    print("portfolio_pyfolio.json: 1 case")


if __name__ == "__main__":
    main()
