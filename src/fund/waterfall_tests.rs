use super::*;

fn sums_to(w: &Waterfall) {
    let tiers = [w.return_of_capital, w.preferred, w.catch_up, w.carry];
    let lp: f64 = tiers.iter().map(|t| t.lp).sum();
    let gp: f64 = tiers.iter().map(|t| t.gp).sum();
    let tol = 1e-9 * (1.0 + w.distributable);
    assert!((lp - w.lp_total).abs() < tol, "{w:?}");
    assert!((gp - w.gp_total).abs() < tol, "{w:?}");
    assert!(
        (w.lp_total + w.gp_total - w.distributable).abs() < tol,
        "{w:?}"
    );
}

#[test]
fn pref_compounding_modes() -> Result<()> {
    let a = preferred_return(1_000.0, 0.08, 5.0, Compounding::Annual)?;
    assert!((a - 1_000.0 * (1.08f64.powi(5) - 1.0)).abs() < 1e-9);
    let half = preferred_return(1_000.0, 0.08, 0.5, Compounding::Annual)?;
    assert!((half - 1_000.0 * (1.08f64.sqrt() - 1.0)).abs() < 1e-9);
    assert!(preferred_return(1.0, -0.1, 1.0, Compounding::Simple).is_err());
    assert!(preferred_return(1.0, 0.1, f64::NAN, Compounding::Simple).is_err());
    Ok(())
}

#[test]
fn pref_schedule_without_returns_matches_closed_form() -> Result<()> {
    let b = preferred_return_schedule(&[100.0, 0.0, 0.0, 0.0], &[0.0; 4], 0.08)?;
    assert!((b.capital - 100.0).abs() < 1e-12);
    let want = preferred_return(100.0, 0.08, 3.0, Compounding::Annual)?;
    assert!((b.preferred - want).abs() < 1e-9);
    // Over-returned capital never goes negative.
    let over = preferred_return_schedule(&[100.0, 0.0], &[0.0, 200.0], 0.08)?;
    assert_eq!(
        over,
        PrefBalance {
            capital: 0.0,
            preferred: 0.0
        }
    );
    assert!(preferred_return_schedule(&[1.0], &[], 0.08).is_err());
    Ok(())
}

#[test]
fn tiers_fill_in_order() -> Result<()> {
    let t = WaterfallTerms::default();
    // Below capital: everything is return of capital.
    let w = european_waterfall(60.0, 100.0, 8.0, t)?;
    assert!((w.return_of_capital.lp - 60.0).abs() < 1e-12 && w.gp_total == 0.0);
    assert!(
        (w.unreturned_capital - 40.0).abs() < 1e-12 && (w.unpaid_preferred - 8.0).abs() < 1e-12
    );
    sums_to(&w);
    // Inside the pref tier.
    let w = european_waterfall(105.0, 100.0, 8.0, t)?;
    assert!((w.preferred.lp - 5.0).abs() < 1e-12 && w.gp_total == 0.0);
    sums_to(&w);
    // Inside catch-up: GP takes all of it.
    let w = european_waterfall(109.0, 100.0, 8.0, t)?;
    assert!((w.catch_up.gp - 1.0).abs() < 1e-12 && w.catch_up.lp.abs() < 1e-12);
    sums_to(&w);
    // Fully caught up: GP holds 20% of profit.
    let w = european_waterfall(300.0, 100.0, 8.0, t)?;
    assert!((w.gp_total - 40.0).abs() < 1e-9);
    assert!((w.carry.gp - 38.0).abs() < 1e-9 && (w.carry.lp - 152.0).abs() < 1e-9);
    sums_to(&w);
    Ok(())
}

#[test]
fn partial_catch_up() -> Result<()> {
    // 80% catch-up, 20% carry: tier size = 0.2·8 / 0.6.
    let t = WaterfallTerms {
        catch_up: 0.8,
        carry: 0.2,
    };
    let w = european_waterfall(500.0, 100.0, 8.0, t)?;
    let size = 0.2 * 8.0 / 0.6;
    assert!((w.catch_up.gp + w.catch_up.lp - size).abs() < 1e-9);
    assert!((w.catch_up.gp - 0.8 * size).abs() < 1e-9);
    assert!((w.gp_total - 0.2 * 400.0).abs() < 1e-9);
    sums_to(&w);
    // No catch-up: GP gets 20% of profit above the pref only.
    let none = european_waterfall(
        500.0,
        100.0,
        8.0,
        WaterfallTerms {
            catch_up: 0.0,
            carry: 0.2,
        },
    )?;
    assert!((none.gp_total - 0.2 * 392.0).abs() < 1e-9);
    // Catch-up share at or below carry absorbs the rest at that share.
    let slow = european_waterfall(
        500.0,
        100.0,
        8.0,
        WaterfallTerms {
            catch_up: 0.1,
            carry: 0.2,
        },
    )?;
    assert!(slow.carry.gp == 0.0 && (slow.gp_total - 0.1 * 392.0).abs() < 1e-9);
    sums_to(&slow);
    Ok(())
}

#[test]
fn waterfall_rejects_bad_terms() {
    let bad = WaterfallTerms {
        catch_up: 1.5,
        carry: 0.2,
    };
    assert!(matches!(
        european_waterfall(1.0, 1.0, 0.0, bad),
        Err(FinanceError::InvalidInput { .. })
    ));
    assert!(european_waterfall(-1.0, 1.0, 0.0, WaterfallTerms::default()).is_err());
    assert!(european_waterfall(1.0, 1.0, f64::INFINITY, WaterfallTerms::default()).is_err());
    assert!(matches!(
        american_waterfall(&[], WaterfallTerms::default(), Losses::CarriedForward),
        Err(FinanceError::TooShort { .. })
    ));
}

#[test]
fn american_carryforward_offsets_losses() -> Result<()> {
    let deals = [
        Deal {
            capital: 100.0,
            preferred: 8.0,
            proceeds: 0.0,
        },
        Deal {
            capital: 100.0,
            preferred: 8.0,
            proceeds: 400.0,
        },
    ];
    let t = WaterfallTerms::default();
    let alone = american_waterfall(&deals, t, Losses::Isolated)?;
    let carried = american_waterfall(&deals, t, Losses::CarriedForward)?;
    // Standalone: second deal pays 20% of its 300 profit.
    assert!((alone.total.gp_total - 60.0).abs() < 1e-9);
    // Carried forward: the 108 shortfall is repaid first; carry on 200 profit.
    assert!((carried.total.gp_total - 40.0).abs() < 1e-9);
    assert!(carried.clawback.abs() < 1e-9);
    assert!((alone.clawback - 20.0).abs() < 1e-9);
    for w in alone
        .deals
        .iter()
        .chain(&carried.deals)
        .chain([&alone.total, &carried.total])
    {
        sums_to(w);
    }
    assert!(carried.total.unreturned_capital.abs() < 1e-12);
    Ok(())
}
