"""Closed-form prediction-market and Kelly reference values."""

import math


def shin(ps):
    total = sum(ps)
    def probs(z):
        out = []
        for p in ps:
            x = p * p / total
            out.append(2 * x / (math.sqrt(z*z + 4*(1-z)*x) + z))
        return out
    lo, hi = 0.0, 1.0
    for _ in range(100):
        mid = (lo + hi) / 2
        if sum(probs(mid)) > 1:
            lo = mid
        else:
            hi = mid
    return probs((lo + hi) / 2)


def cases():
    out = []
    def add(fn, args, expected):
        out.append({"fn": fn, "args": args, "expected": expected})

    for p, b in [(0.6, 2.0), (0.4, 3.0), (0.5, 1.0)]:
        raw = (b*p - (1-p))/b
        add("binary_raw", {"probability": p, "net_odds": b}, raw)
        add("binary", {"probability": p, "net_odds": b}, min(1, max(0, raw)))
    for ps in ([0.55, 0.50], [0.40, 0.35, 0.30]):
        add("multiplicative", {"probabilities": ps}, [p/sum(ps) for p in ps])
        adj = (sum(ps)-1)/len(ps)
        add("additive", {"probabilities": ps}, [p-adj for p in ps])
        k_lo, k_hi = 0.0, 1_000_000.0
        for _ in range(100):
            k = (k_lo+k_hi)/2
            if sum(p**k for p in ps) > 1:
                k_lo = k
            else:
                k_hi = k
        k = (k_lo+k_hi)/2
        add("power", {"probabilities": ps}, [p**k for p in ps])
        add("shin", {"probabilities": ps}, shin(ps))
    for d in [1.5, 2.0, 4.0]:
        add("decimal_to_prob", {"decimal_odds": d}, 1/d)
    for a in [100.0, -110.0, 250.0]:
        p = 100/(a+100) if a > 0 else -a/(-a+100)
        add("american_to_prob", {"american_odds": a}, p)
    for a, b in [(2.0, 1.0), (1.0, 3.0)]:
        add("fractional_to_prob", {"numerator": a, "denominator": b}, b/(a+b))
    for p, b in [(0.6, 2.0), (0.4, 3.0)]:
        add("fractional", {"full_fraction": (b*p-(1-p))/b, "fraction": 0.5}, max(0, (b*p-(1-p))/b)*0.5)
    add("capped", {"fraction": 0.4, "cap": 0.25}, 0.25)
    add("continuous", {"mu": 0.12, "sigma": 0.2, "risk_free": 0.02}, 2.5)
    add("fee", {"ask": 0.4, "rate": 0.07}, 0.0168)
    add("kalshi", {"contracts": 10.0, "price": 0.4, "rate": 0.07}, 0.17)
    add("polymarket", {"contracts": 10.0, "price": 0.4, "rate": 0.02}, 0.048)
    add("held_side_price", {"yes_price": 0.63, "side": "no"}, 0.37)
    add("implied_probability", {"price": 0.4, "payout": 1.0}, 0.4)
    add("overround", {"values": [0.55, 0.50]}, 0.05)
    add("all_in_cost", {"ask": 0.4, "fee_rate": 0.07}, 0.4168)
    add("ask_of_cost", {"cost": 0.4168, "fee_rate": 0.07}, 0.4)
    add("ev", {"p_true": 0.55, "price": 0.4, "fee": 0.02}, 0.13)
    add("breakeven_probability", {"price": 0.4, "fee": 0.02}, 0.42)
    add("annualized_return", {"ev": 0.1, "cost": 0.5, "days": 90.0}, (1.2)**(365/90)-1)
    add("arbitrage", {"yes_ask": 0.45, "no_ask": 0.50, "fees": 0.02}, 0.03)
    add("kelly_fraction", {"p_true": 0.6, "cost": 0.4, "payout": 1.0}, 1/3)
    add("brier", {"forecasts": [0.8, 0.3], "outcomes": [1.0, 0.0]}, ((.8-1)**2+(.3-0)**2)/2)
    add("log_loss", {"forecasts": [0.8, 0.3], "outcomes": [1.0, 0.0]}, -(math.log(.8)+math.log(.7))/2)
    return {"reference": ["closed form"], "tolerance": {fn: {"atol": 2e-10, "rtol": 2e-10} for fn in sorted({c["fn"] for c in out})}, "cases": out}
