"""pyxirr golden cases for finance_rs::tvm extras (cumipmt, cumprinc, nfv)."""

import numpy as np
import pyxirr


def call(fn, *a, **kw):
    try:
        v = fn(*a, **kw)
    except Exception:
        return None
    return None if v is None or not np.isfinite(v) else float(v)


def valid(nper, start, end):
    # finance-rs (as Excel) rejects start < 1, end < start and end > nper;
    # pyxirr returns 0 or a partial sum for some of these.
    return 1 <= start <= end <= nper


def cases():
    rng = np.random.default_rng(1018)
    out = []
    loans = [(0.09 / 12, 360, 125_000.0), (0.005, 60, 20_000.0), (0.1, 10, 1_000.0),
             (0.0, 24, 5_000.0), (-0.002, 36, 10_000.0), (0.0075, 120, -50_000.0),
             (0.25, 30, 1e6)]
    for rate, nper, pv in loans:
        ranges = [(1, 1), (1, 12), (13, 24), (nper, nper), (1, nper), (0, 3), (5, 4), (2, nper + 1)]
        ranges += [tuple(sorted(int(v) for v in rng.integers(1, nper + 1, 2))) for _ in range(3)]
        for start, end in ranges:
            for when, begin in [("end", False), ("begin", True)]:
                args = {"rate": rate, "nper": nper, "pv": pv, "start": start, "end": end, "when": when}
                for fn in ["cumipmt", "cumprinc"]:
                    if fn == "cumprinc" and begin:
                        # pyxirr's begin-mode ppmt keeps period-1 interest that
                        # its own ipmt reports as 0, so its principal never sums
                        # to pv; finance-rs follows numpy-financial's ppmt.
                        continue
                    want = call(getattr(pyxirr, fn), rate, nper, pv, start, end,
                                pmt_at_beginning=begin) if valid(nper, start, end) else None
                    out.append({"fn": fn, "args": args, "expected": want})
    for _ in range(30):
        n = int(rng.integers(1, 25))
        values = [float(v) for v in np.round(rng.normal(50.0, 200.0, n), 2)]
        rate = float(rng.choice([0.0, 0.03, 0.08, 0.15, -0.05]))
        nper = float(rng.integers(n, n + 15))
        out.append({"fn": "nfv", "args": {"rate": rate, "nper": nper, "values": values},
                    "expected": call(pyxirr.nfv, rate, nper, values)})
    tol = {"atol": 1e-9, "rtol": 1e-11}
    return {
        "reference": ["pyxirr"],
        "tolerance": {"cumipmt": tol, "cumprinc": tol, "nfv": tol},
        "cases": out,
    }
