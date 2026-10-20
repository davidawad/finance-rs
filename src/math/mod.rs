//! L0 numerics with no notion of money or dates: the standard normal
//! distribution. Every module that needs `Φ`, `φ` or `Φ⁻¹` (options, VaR,
//! Kelly, probabilistic Sharpe) takes it from here; `perf` re-exports the
//! three functions under their original paths.

// "QuantLib", "VaR" are product names and abbreviations, not code.
#![allow(clippy::doc_markdown)]

mod normal;

pub use normal::{normal_cdf, normal_pdf, normal_ppf};
