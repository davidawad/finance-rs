use finance_rs::cds::{upfront_from_running, CdsInstrument, CdsKind, CreditCurve, HazardNode};
use finance_rs::Date;
#[allow(clippy::cast_sign_loss, clippy::unwrap_used)] // Fixture dates are validated ISO values.
fn parse_date(value: &str) -> Date {
    let mut parts = value.split('-').map(|part| part.parse::<i32>().unwrap());
    let year = parts.next().unwrap();
    let month = parts.next().unwrap() as u32;
    let day = parts.next().unwrap() as u32;
    Date::new(year, month, day).unwrap()
}

#[test]
#[allow(clippy::too_many_lines, clippy::unwrap_used)] // Keep the golden comparison in one readable case loop.
fn quantlib_isda_flat_and_term_curve_goldens_cover_all_valuation_outputs() {
    let goldens: serde_json::Value =
        serde_json::from_str(include_str!("golden/cds_isda_quantlib.json")).unwrap();
    for case in goldens["cases"].as_array().unwrap() {
        let effective = parse_date(case["valuation_date"].as_str().unwrap());
        let maturity = parse_date(case["maturity"].as_str().unwrap());
        let curve = CreditCurve {
            valuation_date: effective,
            nodes: case["hazard_nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|node| HazardNode {
                    maturity: parse_date(node["maturity"].as_str().unwrap()),
                    hazard_rate: node["hazard_rate"].as_f64().unwrap(),
                })
                .collect(),
        };
        let instrument = CdsInstrument::new(
            CdsKind::SingleName {
                reference_entity: case["name"].as_str().unwrap().to_owned(),
            },
            effective,
            maturity,
            case["notional"].as_f64().unwrap(),
            case["coupon"].as_f64().unwrap(),
            case["recovery"].as_f64().unwrap(),
            0.0,
        )
        .unwrap();
        let value = instrument
            .value(&curve, case["discount_rate"].as_f64().unwrap())
            .unwrap();
        let label = case["name"].as_str().unwrap();
        assert_relative(
            value.protection_leg,
            case["protection_leg"].as_f64().unwrap(),
            1e-8,
            label,
            "protection_leg",
        );
        assert_relative(
            value.risky_annuity,
            case["rpv01"].as_f64().unwrap(),
            1e-8,
            label,
            "rpv01",
        );
        assert_relative(
            value.premium_leg,
            case["coupon"].as_f64().unwrap() * case["rpv01"].as_f64().unwrap(),
            1e-8,
            label,
            "premium_leg",
        );
        assert_relative(
            value.clean_value,
            case["clean_price"].as_f64().unwrap(),
            1e-8,
            label,
            "clean_price",
        );
        assert_relative(
            value.dirty_value,
            case["dirty_price"].as_f64().unwrap(),
            1e-8,
            label,
            "dirty_price",
        );
        assert_relative(
            value.dirty_value - value.clean_value,
            case["accrued"].as_f64().unwrap(),
            1e-8,
            label,
            "accrued",
        );
        assert_relative(
            value.par_spread,
            case["par_spread"].as_f64().unwrap(),
            1e-8,
            label,
            "par_spread",
        );
        assert_relative(
            upfront_from_running(
                &instrument,
                &curve,
                case["discount_rate"].as_f64().unwrap(),
                case["par_spread"].as_f64().unwrap(),
            )
            .unwrap(),
            case["upfront"].as_f64().unwrap(),
            1e-8,
            label,
            "upfront",
        );
        for ((node, expected_hazard), expected_survival) in curve
            .nodes
            .iter()
            .zip(case["hazard_rates"].as_array().unwrap())
            .zip(case["survival_probabilities"].as_array().unwrap())
        {
            assert_relative(
                curve.hazard_at(node.maturity),
                expected_hazard.as_f64().unwrap(),
                1e-8,
                label,
                "hazard_rate",
            );
            assert_relative(
                curve.survival_probability(node.maturity),
                expected_survival.as_f64().unwrap(),
                1e-8,
                label,
                "survival_probability",
            );
        }
        assert_relative(
            curve.nodes.last().unwrap().hazard_rate,
            case["hazard_rate"].as_f64().unwrap(),
            1e-8,
            label,
            "terminal_hazard_rate",
        );
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64, case: &str, field: &str) {
    let scale = expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        (actual - expected).abs() / scale <= tolerance,
        "{case}.{field}: actual {actual:.17e}, expected {expected:.17e}"
    );
}
