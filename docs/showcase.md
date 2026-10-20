# Showcase

## Synthetic loan and dated-return example

![Computed loan payment and dated cash-flow return](showcase/finance-rs-demo.svg)

Reproduce the SVG from the checked-in Rust example and synthetic inputs:

```sh
./docs/showcase/make-screenshots.sh
```

The renderer calls `finance_rs::tvm::pmt` and `finance_rs::cashflow::xirr`; it contains no external data source.
