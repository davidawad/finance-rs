# Getting started

`finance-rs` is a standard-library-first Rust library for financial math. The
crate's [catalog](../../../../../../docs/finance-rs/catalog.md) lists the API
surface and reference semantics. Features add optional integrations: `chrono`
date conversion, `decimal` money calculations, `serde` serialization,
`calendars` market holidays, and `actuarial` tables.

Add the crate to a project and use a typed `Result` at the boundary:

```rust
use finance_rs::tvm::{pmt, When};

let monthly = pmt(0.06 / 12.0, 360.0, 300_000.0, 0.0, When::End)?;
assert!((monthly + 1_798.65).abs() < 0.01);
# Ok::<(), finance_rs::FinanceError>(())
```

All examples are based on the checked crate API. Values paid out use the
spreadsheet convention and are negative; amounts received are positive. Start
with [composability](composability.md) to build calculations from explicit
rates, dates, calendars, and cash flows.
