# Parity and reference behavior

Golden fixtures compare formulas with the reference libraries named in the
[crate catalog](../../../../../../docs/finance-rs/catalog.md). Property tests
check identities such as `NPV(IRR) = 0`, rate-conversion round trips, and
cash-flow conservation. The crate deliberately returns typed errors in cases
where some reference packages return `NaN`, zero, or an arbitrary root.

The crate README has the detailed list of reference versions, parity
exceptions, and fixture-generation instructions. A parity exception must name
the input shape and semantic reason; fixtures must not be regenerated just to
make an implementation change pass.
