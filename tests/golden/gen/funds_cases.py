"""Pinned pyxirr PE-method parity fixtures."""

import numpy as np
import numpy_financial as npf
import pyxirr


def call(fn, *args):
    try:
        value = fn(*args)
    except Exception:
        return None
    if value is None or not np.isfinite(value):
        return None
    return float(value)


def cases():
    rng = np.random.default_rng(17013)
    out = []
    for _ in range(30):
        n = int(rng.integers(3, 12))
        split = int(rng.integers(1, n - 1))
        contributions = np.round(rng.uniform(0, 100, n), 2)
        distributions = np.round(rng.uniform(0, 80, n), 2)
        contributions[split:] = 0.0
        distributions[:split] = 0.0
        contributions = contributions.tolist()
        distributions = distributions.tolist()
        index = np.round(np.cumprod(rng.uniform(0.9, 1.12, n)) * 100, 4).tolist()
        nav = np.round(rng.uniform(0, 150, n), 2).tolist()
        args2 = (contributions, distributions)
        args3 = (contributions, distributions, index)
        args4 = (contributions, distributions, index, nav[-1])
        for name, fn, args in [
            ("moic", pyxirr.pe.moic_2, (*args2, nav[-1])),
            ("ks_pme", pyxirr.pe.ks_pme_2, (*args3, nav[-1])),
            ("ln_pme_nav", pyxirr.pe.ln_pme_nav_2, args3),
            ("ln_pme", pyxirr.pe.ln_pme_2, args3),
            ("pme_plus_lambda", pyxirr.pe.pme_plus_lambda_2, args4),
            ("pme_plus", pyxirr.pe.pme_plus_2, args4),
            ("m_pme", pyxirr.pe.m_pme_2, (*args3, nav)),
        ]:
            result = call(fn, *args)
            if name == "ln_pme" and result is not None:
                nav_value = pyxirr.pe.ln_pme_nav_2(*args)
                flows = [d - c for c, d in zip(contributions[:-1], distributions[:-1])]
                flows.append(nav_value - contributions[-1])
                numpy_result = call(npf.irr, flows)
                if numpy_result is None or abs(result - numpy_result) > 1e-8 * (1 + abs(result)):
                    continue  # pyxirr and numpy choose different roots for multi-root streams
            out.append({"fn": name, "args": {"contributions": contributions,
                "distributions": distributions, "index": index, "nav": nav,
                "nav_scalar": nav[-1]}, "expected": result})
    return {"reference": ["pyxirr"], "tolerance": {name: {"atol": 1e-8, "rtol": 1e-8}
            for name in ["moic", "ks_pme", "ln_pme_nav", "ln_pme", "pme_plus_lambda", "pme_plus", "m_pme"]}, "cases": out}
