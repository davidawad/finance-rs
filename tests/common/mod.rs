//! Golden-fixture loader shared by the parity tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::Value;

/// One fixture file written by `tests/golden/gen/generate.py`.
pub struct Golden {
    // type-strength: allow(json-value-field) the fixture is free-form JSON written by generate.py
    pub doc: Value,
}

impl Golden {
    pub fn load(name: &str) -> Self {
        let path = format!("{}/tests/golden/{name}.json", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        Self {
            doc: serde_json::from_str(&text).unwrap(),
        }
    }

    /// Cases for `func`.
    pub fn cases(&self, func: &str) -> Vec<&Value> {
        self.doc["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["fn"] == func)
            .collect()
    }

    /// `(atol, rtol)` for `func`.
    pub fn tol(&self, func: &str) -> (f64, f64) {
        let t = &self.doc["tolerance"][func];
        (
            t["atol"]
                .as_f64()
                .unwrap_or_else(|| panic!("no atol for {func}")),
            t["rtol"].as_f64().unwrap_or(0.0),
        )
    }
}

pub fn f(v: &Value, key: &str) -> f64 {
    v["args"][key]
        .as_f64()
        .unwrap_or_else(|| panic!("arg {key} in {v}"))
}

pub fn fs(v: &Value, key: &str) -> Vec<f64> {
    v["args"][key]
        .as_array()
        .unwrap_or_else(|| panic!("arg {key} in {v}"))
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

/// Running tally of a parity check: cases seen and worst absolute error.
#[derive(Default)]
pub struct Tally {
    pub cases: usize,
    pub max_abs: f64,
    pub max_rel: f64,
    pub failures: Vec<String>,
}

impl Tally {
    /// Compare one result to the fixture's `expected` (null = must be an error).
    pub fn check<E: std::fmt::Debug>(
        &mut self,
        case: &Value,
        got: Result<f64, E>,
        (atol, rtol): (f64, f64),
    ) {
        self.cases += 1;
        match (case["expected"].as_f64(), got) {
            (None, Err(_)) => {}
            (None, Ok(g)) => self
                .failures
                .push(format!("{case}: expected error, got {g}")),
            (Some(want), Err(e)) => self
                .failures
                .push(format!("{case}: want {want}, got {e:?}")),
            (Some(want), Ok(g)) => {
                let err = (g - want).abs();
                self.max_abs = self.max_abs.max(err);
                if want != 0.0 {
                    self.max_rel = self.max_rel.max(err / want.abs());
                }
                if err > atol + rtol * want.abs() {
                    self.failures
                        .push(format!("{case}: want {want}, got {g}, |err| {err:e}"));
                }
            }
        }
    }

    /// Fail with every mismatch; print the parity row for the report.
    pub fn finish(self, func: &str, reference: &str) {
        println!(
            "PARITY {func} | {reference} | {} cases | max abs err {:.1e} | max rel err {:.1e}",
            self.cases, self.max_abs, self.max_rel
        );
        assert!(self.cases > 0, "{func}: no golden cases");
        assert!(
            self.failures.is_empty(),
            "{func}: {} mismatches:\n{}",
            self.failures.len(),
            self.failures.join("\n")
        );
    }
}
