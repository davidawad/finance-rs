"""scipy.optimize golden cases for finance_rs::solve.

Each case names a function family the Rust test rebuilds:
  poly(c)       f(x) = polyval(c, x)  (highest power first), f' its derivative
  cos_lin(k)    f(x) = cos(x) - k*x
  cos_map(a, b) g(x) = a*cos(x) + b    (fixed point)
  sqrt_map(c)   g(x) = sqrt(x + c)     (fixed point)
"""

import numpy as np
from scipy import optimize

XTOL = 1e-14


def cubic(rng):
    """A cubic with one root inside [a, b] and the other two outside."""
    r1 = float(rng.uniform(-5, 5))
    a, b = r1 - float(rng.uniform(0.2, 2.0)), r1 + float(rng.uniform(0.2, 2.0))
    r2 = b + float(rng.uniform(0.5, 5.0))
    r3 = a - float(rng.uniform(0.5, 5.0))
    scale = float(rng.choice([-2.0, -0.5, 1.0, 3.0]))
    return [float(c) for c in scale * np.poly([r1, r2, r3])], a, b


def cases():
    rng = np.random.default_rng(1017)
    out = []
    for _ in range(30):
        c, a, b = cubic(rng)
        f = lambda x, c=c: np.polyval(c, x)
        df = lambda x, c=c: np.polyval(np.polyder(c), x)
        root = optimize.brentq(f, a, b, xtol=XTOL)
        out.append({"fn": "brent", "args": {"family": "poly", "c": c, "a": a, "b": b},
                    "expected": float(root)})
        guess = float(rng.uniform(a, b))
        nr = optimize.newton(f, guess, fprime=df, tol=XTOL, maxiter=200)
        if a <= nr <= b:
            out.append({"fn": "newton_bracketed",
                        "args": {"family": "poly", "c": c, "a": a, "b": b, "guess": guess},
                        "expected": float(nr)})
    for k in [0.1, 0.5, 1.0, 2.0, 7.5]:
        f = lambda x, k=k: np.cos(x) - k * x
        out.append({"fn": "brent", "args": {"family": "cos_lin", "k": k, "a": 0.0, "b": 2.0},
                    "expected": float(optimize.brentq(f, 0.0, 2.0, xtol=XTOL))})
    for _ in range(12):
        roots = sorted(float(r) for r in rng.uniform(-9.0, 9.0, 3))
        if min(np.diff(roots)) < 0.05:
            continue
        c = [float(v) for v in np.poly(roots)]
        real = sorted(float(r.real) for r in np.roots(c) if abs(r.imag) < 1e-12)
        out.append({"fn": "scan_roots", "args": {"family": "poly", "c": c, "lo": -10.0, "hi": 10.0, "n": 2001},
                    "expected": real})
    for a, b in [(0.5, 0.0), (0.9, 0.3), (-0.7, 1.0), (0.2, -2.0), (0.99, 0.0)]:
        g = lambda x, a=a, b=b: a * np.cos(x) + b
        out.append({"fn": "fixed_point", "args": {"family": "cos_map", "a": a, "b": b, "x0": 1.0},
                    "expected": float(optimize.fixed_point(g, 1.0, xtol=1e-14, maxiter=500))})
    for c in [0.5, 2.0, 10.0]:
        g = lambda x, c=c: np.sqrt(x + c)
        out.append({"fn": "fixed_point", "args": {"family": "sqrt_map", "c": c, "x0": 1.0},
                    "expected": float(optimize.fixed_point(g, 1.0, xtol=1e-14, maxiter=500))})
    tol = {"atol": 1e-11, "rtol": 1e-12}
    return {
        "reference": ["scipy", "numpy"],
        "tolerance": {"brent": tol, "newton_bracketed": tol, "scan_roots": tol, "fixed_point": tol},
        "cases": out,
    }
