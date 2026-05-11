//! # dev-coverage
//!
//! Test coverage measurement and regression detection for Rust. Part
//! of the `dev-*` verification suite.
//!
//! Wraps `cargo-llvm-cov` (the modern Rust coverage standard) and
//! emits results as `dev-report::Report`. Detects coverage regressions
//! against a stored baseline so AI agents and CI gates can decide
//! whether a PR drops coverage too far.
//!
//! ## Quick example
//!
//! ```no_run
//! use dev_coverage::{CoverageRun, CoverageThreshold};
//!
//! let run = CoverageRun::new("my-crate", "0.1.0");
//! let result = run.execute().unwrap();
//!
//! let threshold = CoverageThreshold::min_line_pct(80.0);
//! let check = result.into_check_result(threshold);
//! ```
//!
//! ## Status
//!
//! Pre-1.0. The `0.9.0` release defines the shape of the API; the
//! actual `cargo-llvm-cov` integration lands in `0.9.1`.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use dev_report::{CheckResult, Severity};

/// Configuration for a coverage run.
#[derive(Debug, Clone)]
pub struct CoverageRun {
    name: String,
    version: String,
}

impl CoverageRun {
    /// Begin a coverage run for the given crate name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    /// Execute the coverage run.
    ///
    /// In `0.9.0` this is a stub; the actual `cargo llvm-cov`
    /// invocation lands in `0.9.1`.
    pub fn execute(&self) -> Result<CoverageResult, CoverageError> {
        // Stub: returns a zero-coverage result for now.
        Ok(CoverageResult {
            name: self.name.clone(),
            version: self.version.clone(),
            line_pct: 0.0,
            function_pct: 0.0,
            region_pct: 0.0,
            total_lines: 0,
            covered_lines: 0,
        })
    }
}

/// Result of a coverage run.
#[derive(Debug, Clone)]
pub struct CoverageResult {
    /// Crate name.
    pub name: String,
    /// Crate version.
    pub version: String,
    /// Percentage of executable lines that were exercised by tests.
    pub line_pct: f64,
    /// Percentage of functions that were called by tests.
    pub function_pct: f64,
    /// Percentage of regions (branch points) that were exercised.
    pub region_pct: f64,
    /// Total executable lines in the crate.
    pub total_lines: u64,
    /// Lines that were exercised at least once.
    pub covered_lines: u64,
}

/// Threshold defining the minimum acceptable coverage.
#[derive(Debug, Clone, Copy)]
pub enum CoverageThreshold {
    /// Fail if `line_pct < pct`.
    MinLinePct(f64),
    /// Fail if `function_pct < pct`.
    MinFunctionPct(f64),
    /// Fail if `region_pct < pct`.
    MinRegionPct(f64),
}

impl CoverageThreshold {
    /// Build a line-coverage threshold.
    pub fn min_line_pct(pct: f64) -> Self {
        Self::MinLinePct(pct)
    }

    /// Build a function-coverage threshold.
    pub fn min_function_pct(pct: f64) -> Self {
        Self::MinFunctionPct(pct)
    }

    /// Build a region-coverage threshold.
    pub fn min_region_pct(pct: f64) -> Self {
        Self::MinRegionPct(pct)
    }
}

impl CoverageResult {
    /// Convert this result into a `CheckResult` against the given threshold.
    pub fn into_check_result(self, threshold: CoverageThreshold) -> CheckResult {
        let name = format!("coverage::{}", self.name);
        let (actual, target, label) = match threshold {
            CoverageThreshold::MinLinePct(p) => (self.line_pct, p, "line"),
            CoverageThreshold::MinFunctionPct(p) => (self.function_pct, p, "function"),
            CoverageThreshold::MinRegionPct(p) => (self.region_pct, p, "region"),
        };
        let detail = format!("{label} coverage {actual:.2}% (threshold {target:.2}%)");
        if actual < target {
            CheckResult::fail(name, Severity::Warning).with_detail(detail)
        } else {
            CheckResult::pass(name).with_detail(detail)
        }
    }
}

/// Errors that can arise during a coverage run.
#[derive(Debug)]
pub enum CoverageError {
    /// The `cargo-llvm-cov` tool is not installed.
    ToolNotInstalled,
    /// The coverage subprocess failed.
    SubprocessFailed(String),
    /// The coverage output could not be parsed.
    ParseError(String),
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolNotInstalled => write!(f, "cargo-llvm-cov is not installed"),
            Self::SubprocessFailed(s) => write!(f, "subprocess failed: {s}"),
            Self::ParseError(s) => write!(f, "parse error: {s}"),
        }
    }
}

impl std::error::Error for CoverageError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_returns_a_result() {
        let run = CoverageRun::new("x", "0.1.0");
        let r = run.execute().unwrap();
        assert_eq!(r.name, "x");
    }

    #[test]
    fn threshold_pass() {
        let r = CoverageResult {
            name: "x".into(),
            version: "0.1.0".into(),
            line_pct: 90.0,
            function_pct: 85.0,
            region_pct: 80.0,
            total_lines: 100,
            covered_lines: 90,
        };
        let c = r.into_check_result(CoverageThreshold::min_line_pct(80.0));
        assert!(matches!(c.verdict, dev_report::Verdict::Pass));
    }

    #[test]
    fn threshold_fail() {
        let r = CoverageResult {
            name: "x".into(),
            version: "0.1.0".into(),
            line_pct: 50.0,
            function_pct: 60.0,
            region_pct: 40.0,
            total_lines: 100,
            covered_lines: 50,
        };
        let c = r.into_check_result(CoverageThreshold::min_line_pct(80.0));
        assert!(matches!(c.verdict, dev_report::Verdict::Fail));
    }
}
