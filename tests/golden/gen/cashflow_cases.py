"""pyxirr golden cases for finance_rs::cashflow (xnpv, xirr)."""

import datetime as dt

import numpy as np
import pyxirr

from daycount_cases import CONVENTIONS


def call(fn, *a, **kw):
    try:
        v = fn(*a, **kw)
    except Exception:
        return None
    return None if v is None else float(v)


def cases():
    rng = np.random.default_rng(1016)
    d = dt.date.fromisoformat
    fixed = [
        (["2020-01-01", "2020-03-01", "2021-01-01"], [-100.0, 50.0, 60.0]),
        (["2020-03-01", "2020-01-01", "2021-01-01"], [50.0, -100.0, 60.0]),
        (["2021-01-01", "2022-01-01", "2023-01-01"], [-100.0, 230.0, -132.0]),
        (["2015-06-30", "2016-06-30", "2026-06-30"], [-1_000.0, 50.0, 3_000.0]),
        (["2020-01-01", "2020-01-02"], [-100.0, 101.0]),
        (["2020-01-01", "2021-01-01"], [-100.0, 1.0]),
        (["2020-01-01", "2021-01-01"], [-1.0, 101.0]),
        (["2020-01-01", "2021-01-01", "2021-01-01"], [-100.0, 60.0, 50.0]),
        (["2020-01-01", "2021-01-01"], [100.0, 110.0]),
        (["2019-01-01", "2019-07-01", "2020-01-01", "2020-07-01"], [-500.0, 0.0, -500.0, 1_200.0]),
    ]
    series = [([d(s) for s in ds], am) for ds, am in fixed]
    for i in range(24):
        n = int(rng.integers(2, 30))
        start = dt.date(2010, 1, 1).toordinal() + int(rng.integers(0, 4_000))
        offs = np.sort(rng.integers(0, 3_650, n))
        dates = [dt.date.fromordinal(start + int(o)) for o in offs]
        amounts = [float(x) for x in np.round(rng.normal(200.0, 400.0, n), 2)]
        amounts[0] = -abs(amounts[0]) - 1_000.0
        if i % 3 == 0:  # some unsorted inputs
            perm = rng.permutation(n)
            dates = [dates[j] for j in perm]
            amounts = [amounts[j] for j in perm]
        series.append((dates, amounts))
    out = []
    for dates, amounts in series:
        iso = [x.isoformat() for x in dates]
        for rate in [0.0, 0.08, 0.35, -0.2]:
            out.append({"fn": "xnpv", "args": {"rate": rate, "dates": iso, "amounts": amounts},
                        "expected": call(pyxirr.xnpv, rate, dates, amounts)})
        out.append({"fn": "xirr", "args": {"dates": iso, "amounts": amounts},
                    "expected": call(pyxirr.xirr, dates, amounts)})
        for name in ["ACT/360", "30U/360", "30E/360", "ACT/ACT ISDA"]:
            dc = getattr(pyxirr.DayCount, CONVENTIONS[name])
            out.append({"fn": "xirr_day_count",
                        "args": {"dates": iso, "amounts": amounts, "day_count": name},
                        "expected": call(pyxirr.xirr, dates, amounts, day_count=dc)})
            out.append({"fn": "xnpv_day_count",
                        "args": {"rate": 0.08, "dates": iso, "amounts": amounts, "day_count": name},
                        "expected": call(pyxirr.xnpv, 0.08, dates, amounts, day_count=dc)})
    tol = {"atol": 1e-9, "rtol": 1e-11}
    return {
        "reference": ["pyxirr"],
        # pyxirr stops its Newton/Brent iteration at ~1e-10 in the rate.
        "tolerance": {"xnpv": tol, "xnpv_day_count": tol,
                      "xirr": {"atol": 1e-8, "rtol": 0.0},
                      "xirr_day_count": {"atol": 1e-8, "rtol": 0.0}},
        "cases": out,
    }
