# Numerics and errors

The default API uses `f64` and rejects non-finite inputs. Fallible operations
return `FinanceError`; a successful scalar result is checked for overflow.
Stable operations such as `ln_1p` and `exp_m1` are used where cancellation is
important. Root solvers expose tolerances and convergence outcomes rather than
silently accepting an unconverged answer.

Use the optional `decimal` feature for supported money-exact TVM calculations
and cent-rounded amortization schedules. Decimal arithmetic does not change
the semantics of the `f64` APIs. For reproducible results, pass day count,
compounding, frequency, and annualization assumptions explicitly.
