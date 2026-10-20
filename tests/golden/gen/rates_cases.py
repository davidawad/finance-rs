"""QuantLib / ffn golden cases for finance_rs::rates."""

import datetime as dt
import os
import tempfile

os.environ.setdefault("MPLCONFIGDIR", tempfile.mkdtemp())  # ffn imports matplotlib

import ffn  # noqa: E402
import numpy as np  # noqa: E402
import QuantLib as ql  # noqa: E402

FREQ = {"annual": ql.Annual, "semiannual": ql.Semiannual, "quarterly": ql.Quarterly,
        "monthly": ql.Monthly, "weekly": ql.Weekly, "daily": ql.Daily}
DC = {"ACT/365F": ql.Actual365Fixed(), "ACT/360": ql.Actual360()}
# QuantLib 1.44 refuses SimpleThenCompounded in InterestRate ("use the
# CashFlow methods"), so finance-rs checks that basis by its definition
# (simple up to one period, compounded after) in the property tests.
BASES = [("simple", None), ("continuous", None)] + [("compounded", f) for f in FREQ]
COMP = {"simple": ql.Simple, "continuous": ql.Continuous, "compounded": ql.Compounded}


def rate(value, basis, freq, dc="ACT/365F"):
    return ql.InterestRate(value, DC[dc], COMP[basis], FREQ[freq or "annual"])


def qdate(d):
    return ql.Date(d.day, d.month, d.year)


def cases():
    rng = np.random.default_rng(1020)
    out = []
    for basis, freq in BASES:
        for r in [-0.02, 0.0, 0.01, 0.05, 0.2]:
            ir = rate(r, basis, freq)
            for t in [0.0, 0.1, 0.5, 1.0, 2.5, 30.0]:
                args = {"rate": r, "basis": basis, "frequency": freq, "t": t}
                out.append({"fn": "compound_factor", "args": args, "expected": ir.compoundFactor(t)})
                out.append({"fn": "discount_factor", "args": args, "expected": ir.discountFactor(t)})
                if t == 0.0:
                    continue
                for to_basis, to_freq in BASES:
                    eq = ir.equivalentRate(COMP[to_basis], FREQ[to_freq or "annual"], t).rate()
                    out.append({"fn": "equivalent", "args": dict(args, to_basis=to_basis, to_frequency=to_freq),
                                "expected": eq})
    for i in range(24):
        d1 = dt.date.fromordinal(dt.date(2010, 1, 1).toordinal() + int(rng.integers(0, 5_000)))
        d2 = dt.date.fromordinal(d1.toordinal() + int(rng.integers(0, 4_000)))
        basis, freq = BASES[i % len(BASES)]
        dc = ["ACT/365F", "ACT/360"][i % 2]
        r = float(rng.choice([0.01, 0.04, 0.09]))
        out.append({"fn": "compound_factor_between",
                    "args": {"rate": r, "basis": basis, "frequency": freq, "day_count": dc,
                             "start": d1.isoformat(), "end": d2.isoformat()},
                    "expected": rate(r, basis, freq, dc).compoundFactor(qdate(d1), qdate(d2))})
    for m, f in [(1, "annual"), (2, "semiannual"), (4, "quarterly"), (12, "monthly"), (52, "weekly"), (365, "daily")]:
        for r in [-0.03, 0.0, 0.0525, 0.1, 0.35]:
            args = {"rate": r, "periods_per_year": float(m)}
            eff = rate(r, "compounded", f).equivalentRate(ql.Compounded, ql.Annual, 1.0).rate()
            nom = rate(r, "compounded", "annual").equivalentRate(ql.Compounded, FREQ[f], 1.0).rate()
            cont = rate(r, "compounded", f).equivalentRate(ql.Continuous, ql.Annual, 1.0).rate()
            disc = rate(r, "continuous", None).equivalentRate(ql.Compounded, FREQ[f], 1.0).rate()
            out.append({"fn": "effective", "args": args, "expected": eff})
            out.append({"fn": "nominal", "args": args, "expected": nom})
            out.append({"fn": "continuous_from_discrete", "args": args, "expected": cont})
            out.append({"fn": "discrete_from_continuous", "args": args, "expected": disc})
    for m in [1.0, 4.0, 12.0, 52.0, 252.0, 365.0]:
        for r in [-0.05, 0.0, 0.001, 0.01, 0.12, 0.5]:
            args = {"rate": r, "periods_per_year": m}
            out.append({"fn": "annualize", "args": args,
                        "expected": float(ffn.annualize(r, 1.0, one_year=m))})
            out.append({"fn": "deannualize", "args": args, "expected": float(ffn.deannualize(r, m))})
    # QuantLib forms 1 + r/m and raises it to m*t (and implies rates by
    # c^(1/(m*t)) - 1), which loses about m*t ulp at daily compounding over
    # 30 years; finance-rs works through ln_1p/exp_m1, so the tolerance
    # covers QuantLib's rounding, not ours.
    tol = {"atol": 1e-13, "rtol": 2e-12}
    return {
        "reference": ["QuantLib", "ffn"],
        "tolerance": {k: tol for k in ["compound_factor", "discount_factor", "equivalent",
                                        "compound_factor_between", "effective", "nominal",
                                        "continuous_from_discrete", "discrete_from_continuous",
                                        "annualize", "deannualize"]},
        "cases": out,
    }
