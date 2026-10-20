"""pyxirr golden cases for finance_rs::cashflow extras (xfv, xnfv,
zero_crossing_points, is_conventional_cash_flow)."""

import datetime as dt

import numpy as np
import pyxirr

from daycount_cases import CONVENTIONS


def call(fn, *a, **kw):
    try:
        v = fn(*a, **kw)
    except Exception:
        return None
    return None if v is None or not np.isfinite(v) else float(v)


def day(rng, lo=2005, span=7_000):
    return dt.date.fromordinal(dt.date(lo, 1, 1).toordinal() + int(rng.integers(0, span)))


def cases():
    rng = np.random.default_rng(1019)
    out = []
    for i in range(40):
        start, cf, end = day(rng), day(rng), day(rng)
        if i % 2 == 0:
            start, cf, end = sorted([start, cf, end])
        cf_rate = float(rng.choice([0.0, 0.03, 0.05, 0.12, -0.04]))
        end_rate = float(rng.choice([0.0, 0.02, 0.08, 0.3]))
        amount = float(np.round(rng.normal(0.0, 1_000.0), 2))
        args = {"start": start.isoformat(), "cash_flow_date": cf.isoformat(), "end": end.isoformat(),
                "cash_flow_rate": cf_rate, "end_rate": end_rate, "amount": amount}
        out.append({"fn": "xfv", "args": args,
                    "expected": call(pyxirr.xfv, start, cf, end, cf_rate, end_rate, amount)})
        name = ["ACT/360", "30U/360", "ACT/ACT ISDA"][i % 3]
        dc = getattr(pyxirr.DayCount, CONVENTIONS[name])
        out.append({"fn": "xfv_day_count", "args": dict(args, day_count=name),
                    "expected": call(pyxirr.xfv, start, cf, end, cf_rate, end_rate, amount, day_count=dc)})
    for i in range(30):
        n = int(rng.integers(2, 20))
        dates = [day(rng, 2015, 3_000) for _ in range(n)]
        amounts = [float(v) for v in np.round(rng.normal(100.0, 500.0, n), 2)]
        # pyxirr rejects one-signed flows; finance-rs values them (as xnpv does).
        amounts[0] = -abs(amounts[0]) - 1.0
        amounts[-1] = abs(amounts[-1]) + 1.0
        iso = [d.isoformat() for d in dates]
        for rate in [0.0, 0.07, -0.1]:
            out.append({"fn": "xnfv", "args": {"rate": rate, "dates": iso, "amounts": amounts},
                        "expected": call(pyxirr.xnfv, rate, dates, amounts)})
        name = ["ACT/360", "30E/360", "ACT/ACT ISDA"][i % 3]
        dc = getattr(pyxirr.DayCount, CONVENTIONS[name])
        # pyxirr 0.10.8's xnfv discounts with ACT/365F whatever day_count says
        # and uses day_count only for the horizon; the reference here is
        # composed from pyxirr's own xnpv and year_fraction under one day count.
        horizon = pyxirr.year_fraction(min(dates), max(dates), dc)
        out.append({"fn": "xnfv_day_count",
                    "args": {"rate": 0.07, "dates": iso, "amounts": amounts, "day_count": name},
                    "expected": call(pyxirr.xnpv, 0.07, dates, amounts, day_count=dc) * 1.07 ** horizon})
    seqs = [[], [0.0], [1.0], [-1.0, 2.0], [-1.0, 0.0, 2.0, -3.0, 0.0, 0.0, 4.0], [0.0, 0.0, 1.0, -1.0],
            [-100.0, 230.0, -132.0], [5.0, 5.0, 5.0]]
    for _ in range(30):
        n = int(rng.integers(1, 15))
        seqs.append([float(v) for v in rng.choice([-3.0, -1.0, -0.5, 2.0, 7.5], n)])
    for s in seqs:
        if 0.0 in s:
            # pyxirr counts 0 as positive ([-1, 0, -1] crosses twice);
            # finance-rs gives zeros no sign, so these are unit-tested instead.
            continue
        out.append({"fn": "sign_changes", "args": {"amounts": s},
                    "expected": [int(i) for i in pyxirr.zero_crossing_points(s)] if s else []})
        out.append({"fn": "is_conventional", "args": {"amounts": s},
                    "expected": bool(pyxirr.is_conventional_cash_flow(s)) if s else False})
    tol = {"atol": 1e-9, "rtol": 1e-11}
    return {
        "reference": ["pyxirr"],
        "tolerance": {"xfv": tol, "xfv_day_count": tol, "xnfv": tol, "xnfv_day_count": tol,
                      "sign_changes": {"atol": 0.0}, "is_conventional": {"atol": 0.0}},
        "cases": out,
    }
