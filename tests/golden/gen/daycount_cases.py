"""pyxirr golden cases for finance_rs::daycount."""

import datetime as dt

import numpy as np
import pyxirr

CONVENTIONS = {
    "ACT/365F": "ACT_365F", "ACT/360": "ACT_360", "ACT/364": "ACT_364",
    "ACT/365.25": "ACT_365_25", "ACT/ACT ISDA": "ACT_ACT_ISDA",
    "NL/360": "NL_360", "NL/365": "NL_365", "30U/360": "THIRTY_U_360",
    "30/360 ISDA": "THIRTY_360_ISDA", "30E/360": "THIRTY_E_360",
    "30E/360 ISDA": "THIRTY_E_360_ISDA", "30E+/360": "THIRTY_E_PLUS_360",
}

EDGE = [
    ("2020-02-29", "2021-02-28"), ("2020-01-31", "2020-03-31"),
    ("2019-02-28", "2020-02-29"), ("2020-02-28", "2020-03-01"),
    ("2020-02-29", "2020-03-01"), ("2019-12-31", "2020-12-31"),
    ("2021-01-31", "2021-02-28"), ("2020-03-31", "2020-02-29"),
    ("2018-07-15", "2026-10-16"), ("2024-01-01", "2024-01-01"),
    ("2023-08-31", "2023-09-30"), ("2023-08-30", "2023-10-31"),
    ("1999-12-31", "2000-02-29"), ("2100-02-28", "2104-02-29"),
]


def cases():
    rng = np.random.default_rng(16)
    pairs = [(dt.date.fromisoformat(a), dt.date.fromisoformat(b)) for a, b in EDGE]
    base = dt.date(1990, 1, 1).toordinal()
    for _ in range(60):
        a = base + int(rng.integers(0, 15_000))
        b = a + int(rng.integers(-800, 4_000))
        pairs.append((dt.date.fromordinal(a), dt.date.fromordinal(b)))
    # Month-end heavy pairs exercise the 30/360 rules.
    for y in (2019, 2020):
        for m in range(1, 13):
            end = (dt.date(y + (m == 12), m % 12 + 1, 1) - dt.timedelta(days=1))
            pairs.append((end, end.replace(year=y + 1) if (end.month, end.day) != (2, 29)
                          else dt.date(y + 1, 2, 28)))
    out = []
    for name, attr in CONVENTIONS.items():
        dc = getattr(pyxirr.DayCount, attr)
        for a, b in pairs:
            out.append({"fn": "year_fraction",
                        "args": {"day_count": name, "start": a.isoformat(), "end": b.isoformat()},
                        "expected": pyxirr.year_fraction(a, b, dc)})
    return {
        "reference": ["pyxirr"],
        "tolerance": {"year_fraction": {"atol": 1e-14, "rtol": 0.0}},
        "cases": out,
    }
