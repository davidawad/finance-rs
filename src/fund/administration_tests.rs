use super::*;
#[test]
fn call_allocation_conserves_cents() {
    let r = capital_call_allocate(10., &[("a".into(), 100., 0.), ("b".into(), 100., 0.)]).unwrap();
    assert_eq!(r.iter().map(|x| x.amount).sum::<f64>(), 10.);
    let zero = capital_call_allocate(0.0, &[("funded".into(), 100., 100.)]).unwrap();
    assert_eq!(zero[0].amount, 0.0);
}
#[test]
fn capital_account_and_equalization_textbook() {
    assert_eq!(
        capital_account_roll_forward(
            50.,
            20.,
            10.,
            CapitalAllocations {
                income: 2.,
                realized: 0.,
                unrealized: 0.,
                fees: 1.,
                carry: 1.
            }
        )
        .unwrap(),
        60.
    );
    assert_eq!(equalization(25., 100., 40., 0.1, 365).unwrap().total, 11.);
}
#[test]
fn recall_and_coverage() {
    assert_eq!(
        recallable(
            20.,
            RecallTerms {
                cap: 30.,
                recalled_to_date: 5.
            }
        )
        .unwrap()
        .amount,
        20.
    );
    assert_eq!(unfunded_coverage(120., 100.).unwrap().ratio, 1.2);
}
#[test]
fn j_curve_finds_trough_and_recovery() {
    let j = j_curve(&[-10., -5., 8., 10.]).unwrap();
    assert_eq!(j.trough, -15.);
    assert_eq!(j.breakeven_period, Some(3));
}
#[test]
fn bad_inputs_reject() {
    assert!(gp_commitment(100., 1.1).is_err());
    assert!(capital_call_allocate(1., &[]).is_err());
    assert!(matches!(
        capital_call_allocate(1., &[("fully funded".into(), 100.0, 100.0)]),
        Err(FinanceError::Undefined {
            what: "unfunded commitments"
        })
    ));
}

#[cfg(feature = "decimal")]
#[test]
fn decimal_account_preserves_exact_cents() {
    use rust_decimal::Decimal;
    let d = |value| Decimal::new(value, 2);
    let end = capital_account_roll_forward_decimal(
        d(10000),
        d(1000),
        d(2500),
        DecimalCapitalAllocations {
            income: d(50),
            realized: d(25),
            unrealized: d(0),
            fees: d(10),
            carry: d(5),
        },
    )
    .unwrap();
    assert_eq!(end, d(8560));
    assert!(capital_account_roll_forward_decimal(
        d(10000),
        d(-1),
        d(0),
        DecimalCapitalAllocations {
            income: d(0),
            realized: d(0),
            unrealized: d(0),
            fees: d(0),
            carry: d(0),
        }
    )
    .is_err());
}
