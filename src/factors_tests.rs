use crate::factors::{attribution, exposures, fama_french_alpha, rolling_beta};
use crate::{FinanceError, Result};

#[test]
fn factor_regression_and_attribution_validate_and_align_samples() -> Result<()> {
    let market = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let size = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let value = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
    let momentum = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
    let quality = [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let returns = [0.02, 0.04, 0.01, 0.03, -0.01, 0.015, 0.025];
    let benchmark = [-0.03, -0.02, -0.01, 0.01, 0.02, 0.03, 0.04];
    let factors: [&[f64]; 2] = [&market, &size];
    let fit = exposures(&returns, &factors)?;
    let contribution = attribution(&fit, &factors)?;
    assert_eq!(contribution.len(), 3);
    assert_eq!(
        attribution(&fit, &[]),
        Err(FinanceError::InvalidInput {
            what: "exposures",
            reason: "coefficient count must equal factors plus intercept"
        })
    );
    assert!(matches!(
        attribution(&fit, &[&[], &size]),
        Err(FinanceError::TooShort { .. })
    ));
    assert!(matches!(
        attribution(&fit, &[&[0.1], &size]),
        Err(FinanceError::LengthMismatch { .. })
    ));
    assert!(matches!(
        attribution(&fit, &[&[0.1, f64::NAN, 0.2, 0.3, 0.4, 0.5, 0.6], &size]),
        Err(FinanceError::NonFinite { .. })
    ));
    assert!(matches!(
        exposures(&returns, &[]),
        Err(FinanceError::TooShort { .. })
    ));

    let three: [&[f64]; 3] = [&market, &size, &value];
    let five: [&[f64]; 5] = [&market, &size, &value, &momentum, &quality];
    assert!(fama_french_alpha(&returns, &three).is_ok());
    assert!(fama_french_alpha(&returns, &five).is_ok());
    assert!(fama_french_alpha(&returns, &factors).is_err());

    assert!(rolling_beta(&returns, &benchmark, 2).is_ok());
    assert!(matches!(
        rolling_beta(&returns, &benchmark[..6], 2),
        Err(FinanceError::LengthMismatch { .. })
    ));
    assert!(matches!(
        rolling_beta(&returns, &market, 1),
        Err(FinanceError::TooShort { .. })
    ));
    assert!(matches!(
        rolling_beta(&returns, &benchmark, 8),
        Err(FinanceError::TooShort { .. })
    ));
    Ok(())
}
