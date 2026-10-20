"""numpy-financial golden cases for finance_rs::tvm."""

import itertools
import warnings

import numpy as np
import numpy_financial as npf

WHENS = ["end", "begin"]
RATES = [0.0, 0.001, 0.005, 0.0425 / 12, 0.07, 0.25, -0.02]
NPERS = [1, 12, 60, 360, 7.5]


def scalar(fn, *args, **kw):
    with warnings.catch_warnings(), np.errstate(all="ignore"):
        warnings.simplefilter("ignore")
        try:
            v = fn(*args, **kw)
        except Exception:  # reference raised: Rust must error
            return None
    v = float(np.asarray(v).reshape(-1)[0])
    return v


def cases():
    out = []
    add = lambda fn, args, expected: out.append(
        {"fn": fn, "args": args, "expected": expected}
    )
    for rate, nper, when in itertools.product(RATES, NPERS, WHENS):
        for pv_, fv_ in [(250_000.0, 0.0), (-10_000.0, 2_500.0), (0.0, 1e6)]:
            a = {"rate": rate, "nper": nper, "pv": pv_, "fv": fv_, "when": when}
            add("pmt", a, scalar(npf.pmt, rate, nper, pv_, fv_, when))
        for pmt_, pv_ in [(-1_250.0, 0.0), (-100.0, -5_000.0), (300.0, 20_000.0)]:
            a = {"rate": rate, "nper": nper, "pmt": pmt_, "pv": pv_, "when": when}
            add("fv", a, scalar(npf.fv, rate, nper, pmt_, pv_, when))
            a = {"rate": rate, "nper": nper, "pmt": pmt_, "fv": pv_, "when": when}
            add("pv", a, scalar(npf.pv, rate, nper, pmt_, pv_, when))
        if float(nper).is_integer() and nper > 1:
            for per in sorted({1, 2, int(nper) // 2, int(nper)}):
                a = {"rate": rate, "per": per, "nper": nper, "pv": 180_000.0,
                     "fv": -20_000.0, "when": when}
                add("ipmt", a, scalar(npf.ipmt, rate, per, nper, 180_000.0, -20_000.0, when))
                add("ppmt", a, scalar(npf.ppmt, rate, per, nper, 180_000.0, -20_000.0, when))
    for rate, when in itertools.product(RATES, WHENS):
        for pmt_, pv_, fv_ in [(-150.0, 8_000.0, 0.0), (-2_000.0, 300_000.0, -50_000.0),
                               (-500.0, 0.0, 100_000.0), (-1.0, 1_000.0, 0.0)]:
            a = {"rate": rate, "pmt": pmt_, "pv": pv_, "fv": fv_, "when": when}
            add("nper", a, scalar(npf.nper, rate, pmt_, pv_, fv_, when))
    for nper, pmt_, pv_, fv_, when in [
        (48, -250.0, 10_000.0, 0.0, "end"), (48, -250.0, 10_000.0, 0.0, "begin"),
        (360, -1_798.65, 300_000.0, 0.0, "end"), (10, 0.0, -100.0, 200.0, "end"),
        (12, -100.0, 1_000.0, 0.0, "begin"), (60, -500.0, 0.0, 40_000.0, "end"),
        (5, -300.0, 1_000.0, 0.0, "end"), (24, 50.0, -1_000.0, 0.0, "end"),
    ]:
        a = {"nper": nper, "pmt": pmt_, "pv": pv_, "fv": fv_, "when": when}
        add("rate", a, scalar(npf.rate, nper, pmt_, pv_, fv_, when))
    # Round trips: the payment numpy-financial computes for a known rate.
    for r, nper, when in itertools.product([0.001, 0.0425 / 12, 0.03, 0.07, 0.25],
                                           [6, 36, 120, 360], WHENS):
        pmt_ = round(float(npf.pmt(r, nper, 50_000.0, -5_000.0, when)), 2)
        a = {"nper": nper, "pmt": pmt_, "pv": 50_000.0, "fv": -5_000.0, "when": when}
        add("rate", a, scalar(npf.rate, nper, pmt_, 50_000.0, -5_000.0, when))
    rng = np.random.default_rng(20261016)
    series = [
        [-100.0, 39.0, 59.0, 55.0, 20.0],
        [-40_000.0, 5_000.0, 8_000.0, 12_000.0, 30_000.0],
        [-120_000.0, 39_000.0, 30_000.0, 21_000.0, 37_000.0, 46_000.0],
        [-100.0, 0.0, 0.0, 74.0],
        [-100.0, 100.0, 0.0, -7.0],
        [-5.0, 10.5, 1.0, -8.0, 1.0],
        [-1_000.0, 300.0, 400.0, 500.0, 200.0],
        [-100.0, 50.0],
        [-100.0, 230.0, -132.0],
        [10.0, -15.0, 5.5],
    ]
    for _ in range(12):
        n = int(rng.integers(3, 40))
        flows = list(np.round(rng.normal(30.0, 60.0, n), 2))
        flows[0] = -abs(float(rng.normal(500.0, 200.0)))
        series.append([float(x) for x in flows])
    for values in series:
        for rate in [0.0, 0.05, 0.12, -0.03]:
            add("npv", {"rate": rate, "values": values}, scalar(npf.npv, rate, values))
        add("irr", {"values": values}, scalar(npf.irr, values))
        for fr, rr in [(0.1, 0.12), (0.05, 0.05)]:
            add("mirr", {"values": values, "finance_rate": fr, "reinvest_rate": rr},
                scalar(npf.mirr, values, fr, rr))
    add("irr", {"values": [1.0, 2.0, 3.0]}, scalar(npf.irr, [1.0, 2.0, 3.0]))
    add("mirr", {"values": [1.0, 2.0], "finance_rate": 0.1, "reinvest_rate": 0.1},
        scalar(npf.mirr, [1.0, 2.0], 0.1, 0.1))
    tol = {"atol": 1e-9, "rtol": 1e-11}
    return {
        "reference": ["numpy-financial", "numpy"],
        "tolerance": {
            **{fn: tol for fn in ["pmt", "fv", "pv", "ipmt", "ppmt", "nper", "npv", "mirr"]},
            # np.roots (companion-matrix eigenvalues) carries ~1e-12 error on
            # long series; numpy-financial's rate stops Newton at |Δ| < 1e-6.
            "irr": {"atol": 1e-9, "rtol": 0.0},
            "rate": {"atol": 1e-8, "rtol": 0.0},
        },
        "cases": out,
    }
