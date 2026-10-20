use finance_rs::tvm::{pmt, pv, When};
use rstest::rstest;

#[rstest]
#[case(0.0, 12.0, 1_200.0)]
#[case(0.01, 12.0, 10_000.0)]
#[case(0.005, 24.0, 25_000.0)]
fn pmt_and_pv_round_trip(#[case] rate: f64, #[case] nper: f64, #[case] principal: f64) {
    let payment = pmt(rate, nper, principal, 0.0, When::End).unwrap();
    let recovered = pv(rate, nper, payment, 0.0, When::End).unwrap();
    assert!((recovered - principal).abs() < principal * 1e-10 + 1e-8);
}
