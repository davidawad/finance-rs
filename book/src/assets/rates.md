# 2. Rates and curves

Catalog mapping: [§2 Rates and curves](../../../../../../../docs/finance-rs/catalog.md#2-rates-and-curves).

Use `Rate`, `Compounding`, and `Frequency` to preserve quote conventions.
Convert quoted rates to the compounding basis used by a valuation before
discounting. The crate does not infer interpolation, day-count, or curve
construction policy for a caller.
