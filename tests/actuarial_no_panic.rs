//! Public actuarial APIs return errors on hostile numeric inputs, never panic.
use finance_rs::actuarial::retirement::{self, GuytonKlingerPolicy, InheritedIraRule};
use proptest::prelude::*;

fn nasty() -> impl Strategy<Value = f64> {
    prop_oneof![
        Just(f64::NAN),
        Just(f64::INFINITY),
        Just(f64::NEG_INFINITY),
        Just(0.0),
        Just(-1.0),
        Just(1e308),
        Just(-1e308),
        -2.0..2.0_f64,
    ]
}

proptest! {
    #[test]
    fn beneficiary_and_spending_rules_do_not_panic(
        balance in nasty(), ret in nasty(), divisor in nasty(), spend in nasty(),
        age in nasty(), years in 0_u32..=120,
    ) {
        let _ = retirement::inherited_ira_schedule(balance, ret, InheritedIraRule::LifeExpectancy {
            initial_divisor: divisor, years_elapsed: (years % 10) as u8,
        });
        let _ = retirement::inherited_ira_schedule(balance, ret, InheritedIraRule::TenYear {
            annual_divisor: Some(divisor), years_elapsed: (years % 10) as u8,
        });
        let _ = retirement::inherited_ira_schedule(balance, ret, InheritedIraRule::FiveYear {
            years_elapsed: (years % 5) as u8,
        });
        let policy = GuytonKlingerPolicy {
            modified_withdrawal_rule: true,
            preservation_threshold: divisor,
            preservation_cut: ret,
            prosperity_threshold: age,
            prosperity_raise: spend,
        };
        let _ = retirement::guyton_klinger_withdrawal(balance, spend, divisor, ret, age, u8::try_from(years).unwrap_or(u8::MAX), policy);
        let _ = retirement::blanchett_spending_change(age, spend);
        let _ = retirement::blanchett_spending_schedule(spend, age, years);
    }

    #[cfg(feature = "actuarial")]
    #[test]
    fn mortality_tables_do_not_panic(owner in 0_u8..=u8::MAX, spouse in 0_u8..=u8::MAX) {
        let _ = finance_rs::actuarial::tables::irs_joint_last_survivor_2025().factor(owner, spouse);
        let _ = finance_rs::actuarial::tables::soa_2012_iam(finance_rs::actuarial::tables::IamSex::Male);
        let _ = finance_rs::actuarial::tables::soa_2012_iam(finance_rs::actuarial::tables::IamSex::Female);
    }
}
