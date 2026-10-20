# 7. FX

Catalog mapping: [§7 FX](../../../../../../../docs/finance-rs/catalog.md#7-fx).

The options module includes FX option pricing with explicit domestic and
foreign rates. Keep currency units at the call site and use the matching
discount curves; `finance-rs` does not attach currency metadata to `f64`
amounts.
