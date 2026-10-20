"""Values-only portfolio parity cases from PyPortfolioOpt 1.6.0."""

import numpy as np
import pandas as pd
import scipy.cluster.hierarchy as sch
import ffn
from pypfopt import BlackLittermanModel, HRPOpt, expected_returns, risk_models
from pypfopt.black_litterman import market_implied_prior_returns


def cases():
    # PyPortfolioOpt 1.6 checks this SciPy private constant; SciPy 1.18 removed it.
    if not hasattr(sch, "_LINKAGE_METHODS"):
        sch._LINKAGE_METHODS = {"single", "complete", "average", "weighted", "centroid", "median", "ward"}
    returns = pd.DataFrame(
        [
            [0.010, 0.020, -0.005],
            [0.015, -0.010, 0.008],
            [-0.006, 0.012, 0.004],
            [0.009, 0.005, 0.011],
            [-0.002, 0.018, -0.004],
            [0.013, -0.007, 0.006],
        ],
        columns=["A", "B", "C"],
    )
    ppy = 12
    mu = expected_returns.mean_historical_return(
        returns, returns_data=True, compounding=True, frequency=ppy
    )
    cov = risk_models.sample_cov(returns, returns_data=True, frequency=ppy)
    weights = pd.Series({"A": 0.5, "B": 0.3, "C": 0.2})
    delta = 2.5
    prior = market_implied_prior_returns(weights, delta, cov)
    p = np.array([[1.0, -1.0, 0.0]])
    q = np.array([0.03])
    omega = np.array([[0.0025]])
    model = BlackLittermanModel(
        cov, pi=prior, P=p, Q=q, omega=omega, tau=0.05, risk_aversion=delta
    )
    hrp_model = HRPOpt(returns=returns)
    hrp = hrp_model.optimize()
    ffn_inverse_vol = ffn.calc_inv_vol_weights(returns)
    return {
        "reference": {"PyPortfolioOpt": "1.6.0", "ffn": "1.4.1"},
        "tolerance": {
            "mean_historical": {"atol": 1e-10, "rtol": 1e-10},
            "sample_cov": {"atol": 1e-10, "rtol": 1e-10},
            "market_implied_returns": {"atol": 1e-10, "rtol": 1e-10},
            "black_litterman_mean": {"atol": 1e-10, "rtol": 1e-10},
            "black_litterman_cov": {"atol": 1e-10, "rtol": 1e-10},
            "hrp": {"atol": 1e-8, "rtol": 1e-8},
            "ffn_inverse_vol": {"atol": 1e-10, "rtol": 1e-10},
        },
        "cases": [
            {"fn": "mean_historical", "args": {"returns": returns.values.tolist(), "ppy": ppy}, "expected": mu.values.tolist()},
            {"fn": "sample_cov", "args": {"returns": returns.values.tolist(), "ppy": ppy}, "expected": cov.values.tolist()},
            {"fn": "market_implied_returns", "args": {"cov": cov.values.tolist(), "weights": weights.values.tolist(), "delta": delta}, "expected": prior.values.tolist()},
            {"fn": "black_litterman_mean", "args": {"cov": cov.values.tolist(), "weights": weights.values.tolist(), "delta": delta, "P": p.tolist(), "Q": q.tolist(), "omega": omega.tolist(), "tau": 0.05}, "expected": model.bl_returns().values.tolist()},
            {"fn": "black_litterman_cov", "args": {"cov": cov.values.tolist(), "weights": weights.values.tolist(), "delta": delta, "P": p.tolist(), "Q": q.tolist(), "omega": omega.tolist(), "tau": 0.05}, "expected": model.bl_cov().values.tolist()},
            {"fn": "hrp", "args": {"returns": returns.values.tolist()}, "expected": [hrp[k] for k in returns.columns]},
            {"fn": "ffn_inverse_vol", "args": {"returns": returns.values.tolist()}, "expected": ffn_inverse_vol.values.tolist()},
        ],
    }
