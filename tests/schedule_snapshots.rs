use finance_rs::amortization::level_payment;

#[test]
fn amortization_schedule_snapshot() {
    let schedule = level_payment(1_000.0, 0.01, 3).unwrap();
    insta::assert_snapshot!(format!("{schedule:#?}"));
}
