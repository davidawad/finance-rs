# 3. Fixed income

Catalog mapping: [§3 Fixed income](../../../../../../../docs/finance-rs/catalog.md#3-fixed-income).

The `fi` module prices fixed-coupon bonds and reports yield, accrued interest,
Macaulay and modified duration, convexity, and DV01. Dated cash-flow pricing
uses the shared `CashFlowStream` and an explicit day count.
