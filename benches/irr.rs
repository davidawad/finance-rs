//! Criterion benchmarks for the root finders: `irr` and `xirr`.
#![allow(missing_docs, clippy::unwrap_used)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finance_rs::cashflow::{xirr, xirr_times};
use finance_rs::tvm::{irr, irr_roots};
use finance_rs::Date;

fn flows(n: usize) -> Vec<f64> {
    let mut v = vec![-10_000.0];
    #[allow(clippy::cast_precision_loss)]
    v.extend((1..n).map(|i| 150.0 + (i % 7) as f64 * 20.0));
    v
}

fn bench(c: &mut Criterion) {
    let short = flows(10);
    let monthly = flows(360);
    let multi = [-100.0, 230.0, -132.0];
    c.bench_function("irr/10", |b| b.iter(|| irr(black_box(&short)).unwrap()));
    c.bench_function("irr/360", |b| b.iter(|| irr(black_box(&monthly)).unwrap()));
    c.bench_function("irr_roots/multi", |b| {
        b.iter(|| irr_roots(black_box(&multi)).unwrap());
    });
    let start = Date::new(2020, 1, 1).unwrap();
    let dates: Vec<Date> = (0..120).map(|i| start.add_months(i).unwrap()).collect();
    let amounts = flows(120);
    c.bench_function("xirr/120 dated", |b| {
        b.iter(|| xirr(black_box(&dates), black_box(&amounts)).unwrap());
    });
    c.bench_function("xirr/multi-root", |b| {
        b.iter(|| xirr_times(black_box(&[0.0, 1.0, 2.0]), black_box(&multi), 0.1).unwrap());
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
