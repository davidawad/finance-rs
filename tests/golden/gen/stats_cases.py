"""numpy / statsmodels golden cases for finance_rs::stats."""

import warnings

import numpy as np
import statsmodels.api as sm

METHODS = {"linear": "linear", "lower": "lower", "higher": "higher",
           "nearest": "nearest", "midpoint": "midpoint", "excel_exclusive": "weibull"}
QS = [0.0, 0.01, 0.1, 0.25, 0.4, 0.5, 0.75, 0.99, 1.0]


def num(v):
    v = float(v)
    return v if np.isfinite(v) else None


def series(rng):
    out = [[1.0], [1.0, 2.0], [3.0, 3.0, 3.0], [15.0, 20.0, 35.0, 40.0, 50.0],
           [1.0, 2.0, 3.0, 4.0], [-1.5, 2.25, 0.0, 7.0, -3.0, 2.25]]
    for _ in range(14):
        n = int(rng.integers(2, 25))
        xs = rng.normal(0.001, 0.02, n)
        if rng.random() < 0.3:
            xs = np.round(xs * 100) / 100  # ties
        out.append([float(x) for x in xs])
    return out


def excel_exc(xs, q):
    n = len(xs)
    if q < 1.0 / (n + 1) or q > n / (n + 1):
        return None
    return num(np.quantile(xs, q, method="weibull"))


def ols_case(rng, n, k, intercept):
    x = rng.normal(0.0, 1.0, (n, k))
    beta = rng.normal(0.0, 2.0, k)
    y = 0.5 + x @ beta + rng.normal(0.0, 0.3, n)
    design = sm.add_constant(x, has_constant="add") if intercept else x
    fit = sm.OLS(y, design).fit()
    return {"fn": "ols",
            "args": {"y": [float(v) for v in y], "xs": [[float(v) for v in col] for col in x.T],
                     "intercept": intercept},
            "expected": {"coef": [float(v) for v in fit.params], "se": [float(v) for v in fit.bse],
                         "t": [float(v) for v in fit.tvalues], "r2": float(fit.rsquared),
                         "adj_r2": float(fit.rsquared_adj), "resid": [float(v) for v in fit.resid]}}


def cases():
    rng = np.random.default_rng(20261017)
    out = []
    data = series(rng)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        for xs in data:
            a = np.array(xs)
            out.append({"fn": "mean", "args": {"xs": xs}, "expected": num(np.mean(a))})
            for ddof in [0, 1]:
                ok = len(xs) > ddof
                out.append({"fn": "variance", "args": {"xs": xs, "ddof": ddof},
                            "expected": num(np.var(a, ddof=ddof)) if ok else None})
                out.append({"fn": "stdev", "args": {"xs": xs, "ddof": ddof},
                            "expected": num(np.std(a, ddof=ddof)) if ok else None})
            for name, method in METHODS.items():
                for q in QS + [float(rng.random())]:
                    want = excel_exc(a, q) if name == "excel_exclusive" else num(
                        np.quantile(a, q, method=method))
                    out.append({"fn": "quantile", "args": {"xs": xs, "q": q, "method": name},
                                "expected": want})
        for i, xs in enumerate(data):
            ys = data[(i + 3) % len(data)]
            n = min(len(xs), len(ys))
            x, y = np.array(xs[:n]), np.array(ys[:n])
            if i % 4 == 0 and n > 1:
                y = 0.7 * x + 0.1 * y  # strongly correlated pairs
            for ddof in [0, 1]:
                out.append({"fn": "covariance",
                            "args": {"x": list(map(float, x)), "y": list(map(float, y)), "ddof": ddof},
                            "expected": num(np.cov(x, y, ddof=ddof)[0, 1]) if n > ddof else None})
            corr = np.corrcoef(x, y)[0, 1] if n > 1 else float("nan")
            out.append({"fn": "correlation", "args": {"x": list(map(float, x)), "y": list(map(float, y))},
                        "expected": num(corr)})
    for n, k in [(8, 1), (30, 1), (60, 2), (90, 3), (12, 4)]:
        for intercept in [True, False]:
            out.append(ols_case(rng, n, k, intercept))
    tol = {"atol": 1e-12, "rtol": 1e-10}
    return {
        "reference": ["numpy", "statsmodels"],
        "tolerance": {"mean": tol, "variance": tol, "stdev": tol, "quantile": tol,
                      "covariance": tol, "correlation": tol, "ols": {"atol": 1e-10, "rtol": 1e-9}},
        "cases": out,
    }
