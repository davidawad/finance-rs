"""Golden cases for finance_rs::perf (quantstats 0.0.86 / empyrical-reloaded 0.5.12).

Every series is a deterministic draw (numpy default_rng, fixed seeds) rounded
to 1e-6 so the JSON round trip is exact and no observation sits within an ulp
of a quantile threshold or of zero.

Mapping to the Rust API:
- quantstats takes `rf` as an ANNUAL rate and de-annualizes it per period as
  (1 + rf) ** (1 / periods) - 1; the Rust functions take the same annual
  `rf` plus an explicit `periods_per_year`.
- quantstats `cagr` uses years = count / periods; the Rust `cagr` takes the
  same `periods_per_year`, so the year count is identical.
- Series carry a daily calendar index, so quantstats drawdown_details "days"
  equals the drawdown length in periods (`max_drawdown_duration`).
- Historical VaR is empyrical `value_at_risk` (np.percentile, linear);
  everything else is quantstats.
- `expected: null` means the reference returns NaN (Rust returns an error).
  information_ratio returns 0 for a zero tracking error and greeks fills a
  NaN beta with 0; the Rust side returns FinanceError::Undefined for both, so
  those cases are forced to null here (documented deviation).
"""

import warnings

import numpy as np
import pandas as pd
import quantstats as qs
import empyrical as ep

warnings.filterwarnings("ignore")


def _series(values):
    idx = pd.date_range("2020-01-01", periods=len(values), freq="D")
    return pd.Series(np.round(np.asarray(values, dtype=float), 6), index=idx)


def _series_set():
    rng = np.random.default_rng(20261016)
    daily = rng.normal(0.0005, 0.011, 252)

    rng = np.random.default_rng(7)
    bench = rng.normal(0.0004, 0.009, 252)
    strat = 0.0002 + 1.2 * bench + rng.normal(0.0, 0.004, 252)

    rng = np.random.default_rng(99)
    heavy = rng.normal(-0.004, 0.03, 200)
    heavy[[40, 41, 120, 121, 122, 180]] = [-0.25, -0.18, -0.3, -0.12, -0.2, -0.4]
    heavy = np.clip(heavy, -0.95, 0.5)

    rng = np.random.default_rng(3)
    monthly = rng.normal(0.008, 0.045, 60)

    rng = np.random.default_rng(11)
    with_zeros = rng.normal(0.0, 0.01, 120)
    with_zeros[::7] = 0.0
    with_zeros[0] = -0.015  # drawdown from the very first period

    up_only = np.round(np.linspace(0.001, 0.01, 30), 6)
    short = [0.01, -0.02, 0.03, -0.01, 0.02]
    flat = [0.0] * 10
    return {
        "daily": (daily, 252.0, None),
        "bench": (strat, 252.0, bench),
        "heavy": (heavy, 252.0, None),
        "monthly": (monthly, 12.0, None),
        "zeros": (with_zeros, 252.0, None),
        "up_only": (up_only, 252.0, None),
        "short": (short, 252.0, None),
        "flat": (flat, 252.0, None),
    }


def _drawdown_rows(r):
    dd = qs.stats.to_drawdown_series(r)
    det = qs.stats.drawdown_details(dd)
    pos = {d.strftime("%Y-%m-%d"): i for i, d in enumerate(r.index)}
    rows = []
    for _, row in det.iterrows():
        rows.append([
            pos[row["start"]],
            pos[row["valley"]],
            pos[row["end"]],
            int(row["days"]),
            float(row["max drawdown"]) / 100.0,
        ])
    return rows


def cases():
    out = []

    def add(fn, args, expected):
        if callable(expected):
            try:
                expected = expected()
            except ZeroDivisionError:  # quantstats calmar with zero drawdown
                expected = None
        out.append({"fn": fn, "args": args, "expected": expected})

    for name, (vals, ppy, bvals) in _series_set().items():
        r = _series(vals)
        rl = r.tolist()
        base = {"series": name, "returns": rl}
        add("compounded_return", base, qs.stats.comp(r))
        add("cumulative_returns", base, qs.stats.compsum(r).tolist())
        add("volatility", {**base, "periods_per_year": ppy},
            qs.stats.volatility(r, periods=ppy))
        add("cagr", {**base, "periods_per_year": ppy}, qs.stats.cagr(r, periods=ppy))
        add("calmar", {**base, "periods_per_year": ppy}, lambda: qs.stats.calmar(r, periods=ppy))
        add("max_drawdown", base, qs.stats.max_drawdown(r))
        add("drawdown_series", base, qs.stats.to_drawdown_series(r).tolist())
        rows = _drawdown_rows(r)
        add("drawdown_details", base, rows)
        add("max_drawdown_duration", base, max((x[3] for x in rows), default=0))
        add("win_rate", base, qs.stats.win_rate(r))
        for rf in (0.0, 0.04):
            a = {**base, "rf": rf, "periods_per_year": ppy}
            add("sharpe", a, qs.stats.sharpe(r, rf=rf, periods=ppy))
            add("sortino", a, qs.stats.sortino(r, rf=rf, periods=ppy))
            for req in (0.0, 0.1):
                add("omega", {**a, "required_return": req},
                    qs.stats.omega(r, rf=rf, required_return=req, periods=ppy))
        for conf in (0.95, 0.99):
            a = {**base, "confidence": conf}
            add("var_parametric", a, qs.stats.value_at_risk(r, confidence=conf))
            add("var_historical", a, ep.value_at_risk(r.to_numpy(), cutoff=1 - conf))
            add("cvar_parametric", a,
                qs.stats.conditional_value_at_risk(r, confidence=conf, method="parametric"))
            add("cvar_historical", a,
                qs.stats.conditional_value_at_risk(r, confidence=conf, method="historical"))
        if bvals is not None:
            b = _series(bvals)
            pairs = [(b.tolist(), "bench")]
        else:
            # Lagged copy of itself (first period 0) as a benchmark, plus a flat benchmark
            # (zero variance -> Undefined beta / alpha).
            pairs = [(np.concatenate([[0.0], r.to_numpy()[:-1]]).tolist(), "lagged"), ([0.0] * len(rl), "flat")]
        for bl, bname in pairs:
            b = _series(bl)
            a = {**base, "benchmark": b.tolist(), "benchmark_name": bname,
                 "periods_per_year": ppy}
            g = qs.stats.greeks(r, b, periods=ppy)
            undefined_beta = float(b.var()) == 0.0
            add("beta", a, None if undefined_beta else g["beta"])
            add("alpha", a, None if undefined_beta else g["alpha"])
            te = float((r - b).std())
            add("information_ratio", a,
                None if te == 0.0 else qs.stats.information_ratio(r, b))

    tol = {"atol": 1e-12, "rtol": 1e-10}
    fns = sorted({c["fn"] for c in out})
    return {
        "reference": ["quantstats", "empyrical-reloaded", "numpy", "pandas", "scipy"],
        "tolerance": {fn: dict(tol) for fn in fns},
        "cases": out,
    }
