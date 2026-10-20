"""Parity cases for portfolio/risk ratios against pinned qs/emp/ffn."""

import numpy as np
import pandas as pd
import quantstats as qs
import empyrical as ep
import ffn
from scipy.stats import kurtosis, norm, skew


def cases():
    rng = np.random.default_rng(20261017)
    values = np.round(rng.normal(0.001, 0.015, 80), 6)
    values[::13] = 0.0
    r = pd.Series(values, index=pd.date_range("2021-01-01", periods=len(values), freq="D"))
    args = {"series": "daily", "returns": values.tolist(), "rf": 0.02, "periods_per_year": 252.0}
    out = []

    def add(fn, expected, extra=None):
        a = dict(args)
        if extra:
            a.update(extra)
        out.append({"fn": fn, "args": a, "expected": float(expected)})

    add("downside_deviation", ep.downside_risk(r, required_return=0.0, period="daily", annualization=252.0))
    add("skew", qs.stats.skew(r))
    add("kurtosis", qs.stats.kurtosis(r))
    add("smart_sharpe", qs.stats.smart_sharpe(r, rf=0.02, periods=252))
    add("smart_sortino", qs.stats.smart_sortino(r, rf=0.02, periods=252))
    add("adjusted_sortino", qs.stats.adjusted_sortino(r, rf=0.02, periods=252))
    add("probabilistic_sharpe", qs.stats.probabilistic_sharpe_ratio(r, rf=0.0, periods=252, annualize=False))
    add("expected_max_sharpe", ffn.calc_expected_max_sharpe(50, 0.2), {"n_trials": 50, "sr_variance": 0.04})
    add("risk_return_ratio", ffn.calc_risk_return_ratio(r, annualize=False))
    add("gain_to_pain", qs.stats.gain_to_pain_ratio(r))
    add("payoff_ratio", qs.stats.payoff_ratio(r))
    add("profit_factor", qs.stats.profit_factor(r))
    add("profit_ratio", qs.stats.profit_ratio(r))
    add("tail_ratio", qs.stats.tail_ratio(r))
    add("common_sense_ratio", qs.stats.common_sense_ratio(r))
    add("cpc_index", qs.stats.cpc_index(r))
    add("outlier_win_ratio", qs.stats.outlier_win_ratio(r))
    add("outlier_loss_ratio", qs.stats.outlier_loss_ratio(r))
    add("stability", ep.stability_of_timeseries(r))

    # DSR is the same PSR calculation with ffn's expected maximum threshold.
    trials, sr_variance = 50, 0.04
    benchmark = ffn.calc_expected_max_sharpe(trials, sr_variance**0.5)
    sr = r.mean() / r.std(ddof=1)
    n = len(r)
    denominator = np.sqrt(1 - skew(r, bias=False) * sr + ((kurtosis(r, fisher=False, bias=False) - 1) / 4) * sr**2)
    dsr = norm.cdf((sr - benchmark) * np.sqrt(n - 1) / denominator)
    # This fixture records the reference threshold and PSR inputs so Rust can
    # apply the same Bailey-López de Prado probability calculation.
    add("deflated_sharpe", dsr, {"n_trials": trials, "sr_variance": sr_variance})

    return {
        "reference": ["quantstats", "empyrical-reloaded", "ffn"],
        "tolerance": {fn: {"atol": 1e-10, "rtol": 1e-8} for fn in {c["fn"] for c in out}},
        "cases": out,
    }
