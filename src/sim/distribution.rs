use super::Distribution;

pub(super) fn from_normal(distribution: &Distribution, z: f64) -> f64 {
    let u = crate::math::normal_cdf(z).clamp(f64::EPSILON, 1.0 - f64::EPSILON);
    match distribution {
        Distribution::Normal { mean, sd } => mean + sd * z,
        Distribution::LogNormal { mu, sigma } => (mu + sigma * z).exp(),
        Distribution::Uniform { low, high } => low + (high - low) * u,
        Distribution::Triangular { low, mode, high } => triangular(*low, *mode, *high, u),
        Distribution::Pert { low, mode, high } => pert(*low, *mode, *high, u),
        Distribution::Discrete { scenarios, weights } => {
            let total: f64 = weights.iter().sum();
            let mut cumulative = 0.0;
            for (value, weight) in scenarios.iter().zip(weights) {
                cumulative += weight / total;
                if u <= cumulative {
                    return *value;
                }
            }
            scenarios.last().copied().unwrap_or(0.0)
        }
    }
}

fn triangular(low: f64, mode: f64, high: f64, u: f64) -> f64 {
    let span = high - low;
    if span == 0.0 {
        return low;
    }
    let cut = (mode - low) / span;
    if u < cut {
        low + (u * span * (mode - low)).sqrt()
    } else {
        high - ((1.0 - u) * span * (high - mode)).sqrt()
    }
}

fn pert(low: f64, mode: f64, high: f64, u: f64) -> f64 {
    let span = high - low;
    if span == 0.0 {
        return low;
    }
    let a = 1.0 + 4.0 * (mode - low) / span;
    let b = 1.0 + 4.0 * (high - mode) / span;
    low + span * beta_quantile(u, a, b)
}

fn beta_quantile(p: f64, a: f64, b: f64) -> f64 {
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..64 {
        let mid = (low + high) / 2.0;
        if beta_cdf(mid, a, b) < p {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}

fn beta_cdf(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front =
        (log_gamma(a + b) - log_gamma(a) - log_gamma(b) + a * x.ln() + b * (-x).ln_1p()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_fraction(a, b, x) / a
    } else {
        1.0 - front * beta_fraction(b, a, 1.0 - x) / b
    }
}

#[allow(clippy::many_single_char_names)] // Conventional symbols follow the published formula or recurrence notation.
fn beta_fraction(a: f64, b: f64, x: f64) -> f64 {
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < 1e-300 {
        d = 1e-300;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=100 {
        let mf = f64::from(m);
        let m2 = 2.0 * mf;
        let mut aa = mf * (b - mf) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        h *= d * c;
        aa = -(a + mf) * (qab + mf) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 3e-14 {
            break;
        }
    }
    h
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn log_gamma(z: f64) -> f64 {
    const C: [f64; 8] = [
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if z < 0.5 {
        return std::f64::consts::PI.ln()
            - (std::f64::consts::PI * z).sin().ln()
            - log_gamma(1.0 - z);
    }
    let x = C
        .iter()
        .enumerate()
        .fold(0.999_999_999_999_809_9, |sum, (i, c)| {
            sum + c / (z + i as f64 - 0.0)
        });
    let t = z + 6.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (z - 0.5) * t.ln() - t + x.ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantile_mapping_covers_each_marginal() {
        let z = 0.0;
        assert_eq!(
            from_normal(&Distribution::Normal { mean: 4.0, sd: 2.0 }, z),
            4.0
        );
        assert_eq!(
            from_normal(
                &Distribution::LogNormal {
                    mu: 0.0,
                    sigma: 1.0
                },
                z
            ),
            1.0
        );
        let uniform = from_normal(
            &Distribution::Uniform {
                low: 10.0,
                high: 20.0,
            },
            z,
        );
        assert!((uniform - 15.0).abs() < 1e-12);
        let triangle = from_normal(
            &Distribution::Triangular {
                low: 0.0,
                mode: 5.0,
                high: 10.0,
            },
            z,
        );
        assert!((triangle - 5.0).abs() < 1e-6);
        let pert_value = from_normal(
            &Distribution::Pert {
                low: 0.0,
                mode: 5.0,
                high: 10.0,
            },
            z,
        );
        assert!((pert_value - 5.0).abs() < 1e-6);
        let discrete = from_normal(
            &Distribution::Discrete {
                scenarios: vec![2.0, 8.0],
                weights: vec![0.5, 0.5],
            },
            z,
        );
        assert_eq!(discrete, 2.0);
        assert_eq!(triangular(3.0, 3.0, 3.0, 0.7), 3.0);
        assert_eq!(pert(3.0, 3.0, 3.0, 0.7), 3.0);
        assert_eq!(
            from_normal(
                &Distribution::Discrete {
                    scenarios: vec![],
                    weights: vec![]
                },
                z
            ),
            0.0
        );
    }

    #[test]
    fn beta_helpers_cover_both_tails_and_reflection() {
        assert_eq!(beta_cdf(0.0, 2.0, 3.0), 0.0);
        assert_eq!(beta_cdf(1.0, 2.0, 3.0), 1.0);
        assert!(beta_cdf(0.25, 2.0, 3.0) < beta_cdf(0.75, 2.0, 3.0));
        assert!((beta_quantile(0.5, 2.0, 2.0) - 0.5).abs() < 1e-10);
        assert!((log_gamma(0.25) - 1.288_022_524_698_077_4).abs() < 1e-10);
        assert!(beta_fraction(2.0, 3.0, 0.2).is_finite());
        for a in [0.5, 1.0, 2.0, 3.0, 5.0] {
            for b in [0.5, 1.0, 2.0, 4.0, 8.0] {
                let qap = a + 1.0;
                let qab = a + b;
                let x = qap / qab;
                assert!(beta_fraction(a, b, x.min(0.999_999)).is_finite());
                assert!(beta_fraction(a, b, 1e-300).is_finite());
            }
        }
        // Degenerate helper inputs exercise the continued-fraction
        // denominator clamps directly; the public sampler validates shapes.
        for (a, b, x) in [
            (1.0, 1.0, 1.0),
            (1.0, -5.0, 1.0),
            (2.0, 3.0, 1.0),
            (1.0, 4.0, 0.5),
            (1.0, 1.0, 2.0),
            (-10.0, 10.0, -3.5),
        ] {
            let _ = beta_fraction(a, b, x);
        }
    }
}
