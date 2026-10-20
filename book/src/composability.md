# Composability primitives

The library makes conventions explicit so a calculation can be reviewed and
reused without hidden market assumptions.

## Rate, Compounding, and Curve

`Rate` pairs a number with a `Compounding` convention and a `Frequency`.
Conversion methods produce equivalent effective, nominal, or continuous
rates. Curves are represented by dated rates/cash flows at the call site; use
the same `Rate` conversion before interpolation or discounting.

```rust
use finance_rs::rates::{Compounding, Frequency, Rate};

let annual = Rate::new(0.05, Compounding::Compounded, Frequency::Annual)?;
let monthly = annual.equivalent(Frequency::Monthly)?;
assert!(monthly.value() > 0.0);
# Ok::<(), finance_rs::FinanceError>(())
```

## DayCount

`DayCount` calculates year fractions from two `Date` values. Choose the market
convention at the boundary, then pass it through schedule and cash-flow
calculations. ACT/365F is explicit when using the default dated XIRR helpers.

## Calendar

`Calendar` adjusts business dates with a `BusinessDayConvention`; a joint
calendar combines market closures. The default calendar is weekends-only.
Enable `calendars` for recurring holiday rules for supported markets.

## Schedule

`Schedule` generates coupon dates from a start/end, tenor, direction, stub
rule, and business-day policy. It retains reference periods so day-count
fractions remain well-defined around irregular coupons.

## CashFlow and CashFlowStream

`CashFlow` is one dated amount with a `FlowKind`. `CashFlowStream` sorts and
merges same-date amounts, and provides netting, scaling, valuation, and sign
views. This is the shared input for XNPV/XIRR, bond risk, and fund-return
calculations.

```rust
use finance_rs::{cashflow::{CashFlow, CashFlowStream, FlowKind}, Date};

let stream = CashFlowStream::new([
    CashFlow::new(Date::new(2024, 1, 1)?, -100.0, FlowKind::Investment),
    CashFlow::new(Date::new(2025, 1, 1)?, 110.0, FlowKind::Distribution),
])?;
assert_eq!(stream.len(), 2);
# Ok::<(), finance_rs::FinanceError>(())
```
