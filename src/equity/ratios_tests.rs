use super::*;
use proptest::prelude::*;
#[test]
fn textbook_ratios_and_comps() -> Result<()> {
    assert!((multiples::pe(120.0, 6.0)? - 20.0).abs() < 1e-12);
    assert!((multiples::peg(20.0, 0.10)? - 2.0).abs() < 1e-12);
    assert!((yields::shareholder(3.0, 2.0, 100.0)? - 0.05).abs() < 1e-12);
    assert!((returns::roe(15.0, 100.0)? - 0.15).abs() < 1e-12);
    assert!((dupont(15.0, 100.0, 50.0, 100.0)? - 0.15).abs() < 1e-12);
    assert!((ncav_per_share(100.0, 40.0, 10.0, 10.0)? - 5.0).abs() < 1e-12);
    assert!((graham_number(4.0, 25.0)? - 47.434_164_902_525_69).abs() < 1e-12);
    let c = comps_implied_value(10.0, &[8.0, 10.0, 12.0, 20.0], PeerStat::Quartiles)?;
    assert!((c.central - 110.0).abs() < 1e-12);
    assert!((short_carry(0.08, 0.02, 0.01)? - 0.05).abs() < 1e-12);
    assert!((fcf_margin(20.0, 100.0)? - 0.2).abs() < 1e-12);
    assert!(multiples::pe(10.0, 0.0).is_err());
    assert!(yields::earnings(10.0, 0.0).is_err());
    Ok(())
}

proptest! {
    #[test]
    fn ratio_identities(
        x in 0.01_f64..1e6, d in 0.01_f64..1e6,
        income in -1e5_f64..1e5, sales in 0.01_f64..1e6,
        assets in 0.01_f64..1e6, equity in 0.01_f64..1e6,
    ) {
        prop_assert!((multiples::pe(x * d, d)? - x).abs() < 1e-8 * x.max(1.0));
        prop_assert!((returns::roe(income, equity)? - income / equity).abs() < 1e-10);
        prop_assert!((returns::roa(income, assets)? - income / assets).abs() < 1e-10);
        prop_assert!((dupont(income, sales, assets, equity)? - income / equity).abs() < 1e-10);
        prop_assert!((fcf_margin(income, sales)? - income / sales).abs() < 1e-10);
    }
}
