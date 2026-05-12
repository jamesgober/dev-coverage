//! # dev-coverage
//!
//! Test coverage measurement and regression detection for Rust. Part of
//! the `dev-*` verification suite.
//!
//! Wraps `cargo-llvm-cov` — the modern Rust coverage standard — and emits
//! results as a [`dev_report::Report`]. Compares against a stored
//! baseline to flag regressions so AI agents and CI gates can decide
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
//! ## What dev-coverage provides
//!
//! - [`CoverageRun`] — builder around `cargo llvm-cov`.
//! - [`CoverageResult`] — line / function / region percentages plus
//!   per-file breakdown.
//! - [`CoverageThreshold`] — fail when coverage drops below an absolute
//!   floor.
//! - [`Baseline`] + [`BaselineStore`] — persist per-commit coverage so
//!   the next run can flag regressions.
//! - [`CoverageProducer`] — `dev_report::Producer` integration for
//!   pipelines that compose multiple producers via `dev-tools`.
//!
//! ## Requirements
//!
//! `cargo-llvm-cov` must be installed on the system:
//!
//! ```text
//! cargo install cargo-llvm-cov
//! ```
//!
//! The crate detects its absence and emits
//! [`CoverageError::ToolNotInstalled`] without panicking.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::io;
use std::path::PathBuf;
use std::process::Command;

use dev_report::{CheckResult, Evidence, Severity};
use serde::{Deserialize, Serialize};

pub mod baseline;
pub use baseline::{Baseline, BaselineStore, JsonFileBaselineStore};

mod producer;
pub use producer::CoverageProducer;

// ---------------------------------------------------------------------------
// CoverageRun
// ---------------------------------------------------------------------------

/// Configuration for a coverage run.
///
/// Wraps `cargo llvm-cov --json --summary-only`. Use the builder methods
/// to scope the run (working directory, workspace toggle, excludes,
/// features), then call [`execute`](Self::execute) to invoke the
/// subprocess and parse the result.
///
/// # Example
///
/// ```no_run
/// use dev_coverage::CoverageRun;
///
/// let run = CoverageRun::new("my-crate", "0.1.0")
///     .workspace()
///     .exclude("tests/*")
///     .all_features();
///
/// let _result = run.execute().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct CoverageRun {
    name: String,
    version: String,
    workdir: Option<PathBuf>,
    workspace: bool,
    excludes: Vec<String>,
    features: Vec<String>,
    all_features: bool,
    no_default_features: bool,
    per_file: bool,
}

impl CoverageRun {
    /// Begin a coverage run for the given crate name and version.
    ///
    /// `name` and `version` are descriptive — they identify the subject
    /// in the produced `dev-report::Report`. They do NOT need to match
    /// the package being measured (that is determined by `cargo`'s own
    /// resolution from the working directory).
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            workdir: None,
            workspace: false,
            excludes: Vec::new(),
            features: Vec::new(),
            all_features: false,
            no_default_features: false,
            per_file: false,
        }
    }

    /// Run `cargo llvm-cov` from `dir` instead of the current directory.
    pub fn in_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.workdir = Some(dir.into());
        self
    }

    /// Descriptive subject name passed in via [`new`](Self::new).
    pub fn subject(&self) -> &str {
        &self.name
    }

    /// Descriptive subject version passed in via [`new`](Self::new).
    pub fn subject_version(&self) -> &str {
        &self.version
    }

    /// Pass `--workspace` so every workspace member is measured.
    pub fn workspace(mut self) -> Self {
        self.workspace = true;
        self
    }

    /// Pass `--exclude <pattern>`. May be called multiple times.
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.excludes.push(pattern.into());
        self
    }

    /// Add a specific feature to enable. May be called multiple times.
    pub fn feature(mut self, name: impl Into<String>) -> Self {
        self.features.push(name.into());
        self
    }

    /// Pass `--all-features`.
    pub fn all_features(mut self) -> Self {
        self.all_features = true;
        self
    }

    /// Pass `--no-default-features`.
    pub fn no_default_features(mut self) -> Self {
        self.no_default_features = true;
        self
    }

    /// Request a per-file breakdown (drops `--summary-only`).
    ///
    /// Default is summary-only — much smaller JSON, faster parse. Enable
    /// this when you need [`CoverageResult::files`] populated.
    pub fn per_file(mut self) -> Self {
        self.per_file = true;
        self
    }

    /// Execute the run.
    ///
    /// Returns a [`CoverageResult`] on success, or a [`CoverageError`]
    /// describing what went wrong (missing tool, subprocess failure,
    /// parse failure).
    pub fn execute(&self) -> Result<CoverageResult, CoverageError> {
        detect_tool()?;
        let stdout = self.run_llvm_cov()?;
        parse_llvm_cov_json(&stdout, self.name.clone(), self.version.clone())
    }

    fn run_llvm_cov(&self) -> Result<String, CoverageError> {
        let mut cmd = Command::new("cargo");
        cmd.arg("llvm-cov");
        if !self.per_file {
            cmd.arg("--summary-only");
        }
        cmd.arg("--json");
        if self.workspace {
            cmd.arg("--workspace");
        }
        for pat in &self.excludes {
            cmd.args(["--exclude", pat]);
        }
        if self.all_features {
            cmd.arg("--all-features");
        }
        if self.no_default_features {
            cmd.arg("--no-default-features");
        }
        for feat in &self.features {
            cmd.args(["--features", feat]);
        }
        if let Some(dir) = self.workdir.as_ref() {
            cmd.current_dir(dir);
        }
        let output = cmd
            .output()
            .map_err(|e| CoverageError::SubprocessFailed(e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(CoverageError::SubprocessFailed(stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

fn detect_tool() -> Result<(), CoverageError> {
    let probe = Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output();
    match probe {
        Ok(out) if out.status.success() => Ok(()),
        Ok(_) => Err(CoverageError::ToolNotInstalled),
        Err(_) => Err(CoverageError::ToolNotInstalled),
    }
}

// ---------------------------------------------------------------------------
// CoverageResult + FileCoverage
// ---------------------------------------------------------------------------

/// Result of a coverage run.
///
/// Top-level percentages and counts are always populated. The
/// `files` vector is populated only when the run was configured with
/// [`CoverageRun::per_file`].
///
/// # Example
///
/// ```
/// use dev_coverage::CoverageResult;
///
/// let r = CoverageResult {
///     name: "my-crate".into(),
///     version: "0.1.0".into(),
///     line_pct: 87.5,
///     function_pct: 90.0,
///     region_pct: 82.0,
///     branch_pct: None,
///     total_lines: 200,
///     covered_lines: 175,
///     total_functions: 50,
///     covered_functions: 45,
///     total_regions: 100,
///     covered_regions: 82,
///     files: Vec::new(),
/// };
/// assert!(r.line_pct > 80.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageResult {
    /// Crate or subject name (descriptive; matches the `Report` subject).
    pub name: String,
    /// Subject version (descriptive; matches the `Report` subject_version).
    pub version: String,
    /// Percentage of executable lines exercised by tests. `0.0..=100.0`.
    pub line_pct: f64,
    /// Percentage of functions called by tests. `0.0..=100.0`.
    pub function_pct: f64,
    /// Percentage of regions (branch points) exercised. `0.0..=100.0`.
    pub region_pct: f64,
    /// Percentage of branches exercised. Not all builds emit branch
    /// counts; `None` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_pct: Option<f64>,
    /// Total executable lines.
    pub total_lines: u64,
    /// Lines exercised at least once.
    pub covered_lines: u64,
    /// Total functions.
    pub total_functions: u64,
    /// Functions called by at least one test.
    pub covered_functions: u64,
    /// Total regions.
    pub total_regions: u64,
    /// Regions exercised at least once.
    pub covered_regions: u64,
    /// Per-file breakdown. Empty unless the run was configured with
    /// [`CoverageRun::per_file`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileCoverage>,
}

/// Coverage measurements for a single source file.
///
/// Populated only when the parent run was configured with
/// [`CoverageRun::per_file`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    /// Absolute path emitted by `cargo llvm-cov`.
    pub filename: String,
    /// Line coverage percentage. `0.0..=100.0`.
    pub line_pct: f64,
    /// Function coverage percentage. `0.0..=100.0`.
    pub function_pct: f64,
    /// Region coverage percentage. `0.0..=100.0`.
    pub region_pct: f64,
    /// Total executable lines in this file.
    pub total_lines: u64,
    /// Lines in this file exercised at least once.
    pub covered_lines: u64,
}

impl CoverageResult {
    /// Convert this result into a [`CheckResult`] against the given threshold.
    ///
    /// Pass when the measured percentage meets or exceeds the threshold,
    /// otherwise fail with [`Severity::Warning`]. The verdict carries
    /// numeric evidence for both the actual and target percentages.
    ///
    /// # Example
    ///
    /// ```
    /// use dev_coverage::{CoverageResult, CoverageThreshold};
    /// use dev_report::Verdict;
    ///
    /// let r = CoverageResult {
    ///     name: "x".into(), version: "0.1.0".into(),
    ///     line_pct: 90.0, function_pct: 85.0, region_pct: 80.0,
    ///     branch_pct: None,
    ///     total_lines: 100, covered_lines: 90,
    ///     total_functions: 20, covered_functions: 17,
    ///     total_regions: 50, covered_regions: 40,
    ///     files: Vec::new(),
    /// };
    /// let c = r.into_check_result(CoverageThreshold::min_line_pct(80.0));
    /// assert_eq!(c.verdict, Verdict::Pass);
    /// ```
    pub fn into_check_result(self, threshold: CoverageThreshold) -> CheckResult {
        let (actual, target, label) = threshold.applied_to(&self);
        let name = format!("coverage::{}", self.name);
        let detail = format!("{label} coverage {actual:.2}% (threshold {target:.2}%)");
        let mut check = if actual < target {
            CheckResult::fail(name, Severity::Warning).with_detail(detail)
        } else {
            CheckResult::pass(name).with_detail(detail)
        };
        check = check
            .with_tag("coverage")
            .with_evidence(Evidence::numeric(format!("{label}_pct"), actual))
            .with_evidence(Evidence::numeric(format!("{label}_pct_threshold"), target))
            .with_evidence(Evidence::numeric_int(
                "total_lines",
                self.total_lines as i64,
            ))
            .with_evidence(Evidence::numeric_int(
                "covered_lines",
                self.covered_lines as i64,
            ));
        check
    }

    /// Compare this result against a stored baseline.
    ///
    /// Returns a [`CoverageDiff`] carrying signed deltas for each metric.
    /// `tolerance_pct` is the maximum negative delta tolerated before
    /// the diff is flagged as a regression — e.g. `tolerance_pct = 1.0`
    /// allows up to a 1-percentage-point drop without regressing.
    ///
    /// # Example
    ///
    /// ```
    /// use dev_coverage::{Baseline, CoverageResult};
    ///
    /// let r = CoverageResult {
    ///     name: "x".into(), version: "0.1.0".into(),
    ///     line_pct: 75.0, function_pct: 80.0, region_pct: 70.0,
    ///     branch_pct: None,
    ///     total_lines: 100, covered_lines: 75,
    ///     total_functions: 20, covered_functions: 16,
    ///     total_regions: 50, covered_regions: 35,
    ///     files: Vec::new(),
    /// };
    /// let baseline = Baseline {
    ///     name: "x".into(),
    ///     line_pct: 80.0, function_pct: 85.0, region_pct: 75.0,
    /// };
    /// let diff = r.diff(&baseline, 1.0);
    /// assert!(diff.regressed);
    /// assert_eq!(diff.line_pct_delta, -5.0);
    /// ```
    pub fn diff(&self, baseline: &Baseline, tolerance_pct: f64) -> CoverageDiff {
        let line = self.line_pct - baseline.line_pct;
        let func = self.function_pct - baseline.function_pct;
        let region = self.region_pct - baseline.region_pct;
        let worst = line.min(func).min(region);
        CoverageDiff {
            line_pct_delta: line,
            function_pct_delta: func,
            region_pct_delta: region,
            regressed: worst < -tolerance_pct,
        }
    }

    /// Convert this result into a [`Baseline`] suitable for persisting.
    pub fn to_baseline(&self) -> Baseline {
        Baseline {
            name: self.name.clone(),
            line_pct: self.line_pct,
            function_pct: self.function_pct,
            region_pct: self.region_pct,
        }
    }

    /// Return the `n` files with the lowest line coverage, sorted ascending.
    ///
    /// Useful for emitting evidence about which files most need attention.
    /// Returns an empty vector when `files` was not populated.
    pub fn least_covered_files(&self, n: usize) -> Vec<&FileCoverage> {
        let mut refs: Vec<&FileCoverage> = self.files.iter().collect();
        refs.sort_by(|a, b| {
            a.line_pct
                .partial_cmp(&b.line_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        refs.into_iter().take(n).collect()
    }
}

// ---------------------------------------------------------------------------
// CoverageThreshold
// ---------------------------------------------------------------------------

/// Threshold defining the minimum acceptable coverage.
#[derive(Debug, Clone, Copy)]
pub enum CoverageThreshold {
    /// Fail when `line_pct` is below the given percentage.
    MinLinePct(f64),
    /// Fail when `function_pct` is below the given percentage.
    MinFunctionPct(f64),
    /// Fail when `region_pct` is below the given percentage.
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

    fn applied_to(self, r: &CoverageResult) -> (f64, f64, &'static str) {
        match self {
            Self::MinLinePct(p) => (r.line_pct, p, "line"),
            Self::MinFunctionPct(p) => (r.function_pct, p, "function"),
            Self::MinRegionPct(p) => (r.region_pct, p, "region"),
        }
    }
}

// ---------------------------------------------------------------------------
// CoverageDiff
// ---------------------------------------------------------------------------

/// Signed deltas between a current [`CoverageResult`] and a stored
/// [`Baseline`].
///
/// Negative deltas indicate coverage dropped; `regressed` is `true`
/// when at least one delta exceeds the tolerance passed to
/// [`CoverageResult::diff`].
#[derive(Debug, Clone, Copy)]
pub struct CoverageDiff {
    /// Current `line_pct` minus baseline `line_pct`.
    pub line_pct_delta: f64,
    /// Current `function_pct` minus baseline `function_pct`.
    pub function_pct_delta: f64,
    /// Current `region_pct` minus baseline `region_pct`.
    pub region_pct_delta: f64,
    /// `true` when at least one delta is worse than the tolerance.
    pub regressed: bool,
}

// ---------------------------------------------------------------------------
// CoverageError
// ---------------------------------------------------------------------------

/// Errors that can arise during a coverage run.
#[derive(Debug)]
pub enum CoverageError {
    /// `cargo-llvm-cov` is not installed.
    ToolNotInstalled,
    /// The coverage subprocess returned a non-zero exit code.
    SubprocessFailed(String),
    /// The coverage output could not be parsed as JSON of the expected shape.
    ParseError(String),
    /// An I/O error occurred while reading or writing baseline files.
    Io(io::Error),
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolNotInstalled => {
                write!(
                    f,
                    "cargo-llvm-cov is not installed; run `cargo install cargo-llvm-cov`"
                )
            }
            Self::SubprocessFailed(s) => write!(f, "cargo llvm-cov failed: {s}"),
            Self::ParseError(s) => write!(f, "could not parse cargo llvm-cov output: {s}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for CoverageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for CoverageError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

// ---------------------------------------------------------------------------
// LLVM-cov JSON parser
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LlvmCovExport {
    #[serde(default)]
    data: Vec<LlvmCovData>,
}

#[derive(Deserialize)]
struct LlvmCovData {
    #[serde(default)]
    files: Vec<LlvmCovFile>,
    totals: LlvmCovTotals,
}

#[derive(Deserialize)]
struct LlvmCovFile {
    filename: String,
    summary: LlvmCovTotals,
}

#[derive(Deserialize)]
struct LlvmCovTotals {
    lines: LlvmCovMetric,
    functions: LlvmCovMetric,
    regions: LlvmCovMetric,
    #[serde(default)]
    branches: Option<LlvmCovMetric>,
}

#[derive(Deserialize, Default, Clone, Copy)]
struct LlvmCovMetric {
    #[serde(default)]
    count: u64,
    #[serde(default)]
    covered: u64,
    #[serde(default)]
    percent: f64,
}

fn parse_llvm_cov_json(
    json: &str,
    name: String,
    version: String,
) -> Result<CoverageResult, CoverageError> {
    let export: LlvmCovExport =
        serde_json::from_str(json).map_err(|e| CoverageError::ParseError(e.to_string()))?;
    let data = export
        .data
        .into_iter()
        .next()
        .ok_or_else(|| CoverageError::ParseError("export.data was empty".into()))?;
    let totals = data.totals;
    let files = data
        .files
        .into_iter()
        .map(|f| FileCoverage {
            filename: f.filename,
            line_pct: f.summary.lines.percent,
            function_pct: f.summary.functions.percent,
            region_pct: f.summary.regions.percent,
            total_lines: f.summary.lines.count,
            covered_lines: f.summary.lines.covered,
        })
        .collect();
    Ok(CoverageResult {
        name,
        version,
        line_pct: totals.lines.percent,
        function_pct: totals.functions.percent,
        region_pct: totals.regions.percent,
        branch_pct: totals.branches.map(|b| b.percent),
        total_lines: totals.lines.count,
        covered_lines: totals.lines.covered,
        total_functions: totals.functions.count,
        covered_functions: totals.functions.covered,
        total_regions: totals.regions.count,
        covered_regions: totals.regions.covered,
        files,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use dev_report::Verdict;

    fn sample_result(line: f64, func: f64, region: f64) -> CoverageResult {
        CoverageResult {
            name: "x".into(),
            version: "0.1.0".into(),
            line_pct: line,
            function_pct: func,
            region_pct: region,
            branch_pct: None,
            total_lines: 100,
            covered_lines: (line as u64),
            total_functions: 20,
            covered_functions: 16,
            total_regions: 50,
            covered_regions: 40,
            files: Vec::new(),
        }
    }

    #[test]
    fn threshold_pass_when_above() {
        let c = sample_result(90.0, 85.0, 80.0)
            .into_check_result(CoverageThreshold::min_line_pct(80.0));
        assert_eq!(c.verdict, Verdict::Pass);
        assert!(c.has_tag("coverage"));
        assert!(c.evidence.iter().any(|e| e.label == "line_pct"));
    }

    #[test]
    fn threshold_fail_when_below() {
        let c = sample_result(50.0, 60.0, 40.0)
            .into_check_result(CoverageThreshold::min_line_pct(80.0));
        assert_eq!(c.verdict, Verdict::Fail);
        assert_eq!(c.severity, Some(Severity::Warning));
    }

    #[test]
    fn threshold_function_and_region_paths() {
        let r = sample_result(90.0, 50.0, 90.0);
        let c = r
            .clone()
            .into_check_result(CoverageThreshold::min_function_pct(80.0));
        assert_eq!(c.verdict, Verdict::Fail);
        let c2 = sample_result(90.0, 85.0, 50.0)
            .into_check_result(CoverageThreshold::min_region_pct(80.0));
        assert_eq!(c2.verdict, Verdict::Fail);
    }

    #[test]
    fn diff_signs_deltas_correctly() {
        let r = sample_result(75.0, 80.0, 70.0);
        let b = Baseline {
            name: "x".into(),
            line_pct: 80.0,
            function_pct: 85.0,
            region_pct: 75.0,
        };
        let d = r.diff(&b, 0.0);
        assert!(d.line_pct_delta < 0.0);
        assert!(d.function_pct_delta < 0.0);
        assert!(d.region_pct_delta < 0.0);
        assert!(d.regressed);
    }

    #[test]
    fn diff_tolerance_accepts_small_drops() {
        let r = sample_result(79.5, 84.5, 74.5);
        let b = Baseline {
            name: "x".into(),
            line_pct: 80.0,
            function_pct: 85.0,
            region_pct: 75.0,
        };
        // 0.5pp drops everywhere; tolerance of 1.0 accepts.
        let d = r.diff(&b, 1.0);
        assert!(!d.regressed);
    }

    #[test]
    fn diff_improvement_is_not_regression() {
        let r = sample_result(95.0, 95.0, 95.0);
        let b = Baseline {
            name: "x".into(),
            line_pct: 80.0,
            function_pct: 85.0,
            region_pct: 75.0,
        };
        let d = r.diff(&b, 0.0);
        assert!(d.line_pct_delta > 0.0);
        assert!(!d.regressed);
    }

    #[test]
    fn least_covered_files_returns_sorted_subset() {
        let mut r = sample_result(80.0, 80.0, 80.0);
        r.files = vec![
            FileCoverage {
                filename: "a.rs".into(),
                line_pct: 90.0,
                function_pct: 90.0,
                region_pct: 90.0,
                total_lines: 10,
                covered_lines: 9,
            },
            FileCoverage {
                filename: "b.rs".into(),
                line_pct: 50.0,
                function_pct: 50.0,
                region_pct: 50.0,
                total_lines: 10,
                covered_lines: 5,
            },
            FileCoverage {
                filename: "c.rs".into(),
                line_pct: 70.0,
                function_pct: 70.0,
                region_pct: 70.0,
                total_lines: 10,
                covered_lines: 7,
            },
        ];
        let least = r.least_covered_files(2);
        assert_eq!(least.len(), 2);
        assert_eq!(least[0].filename, "b.rs");
        assert_eq!(least[1].filename, "c.rs");
    }

    #[test]
    fn parse_llvm_cov_summary_only() {
        let json = r#"{
            "type": "llvm.coverage.json.export",
            "version": "2.0.1",
            "data": [{
                "files": [],
                "totals": {
                    "lines":      { "count": 200, "covered": 170, "percent": 85.0 },
                    "functions":  { "count": 50,  "covered": 45,  "percent": 90.0 },
                    "regions":    { "count": 100, "covered": 80,  "percent": 80.0 },
                    "branches":   { "count": 30,  "covered": 24,  "percent": 80.0 }
                }
            }]
        }"#;
        let r = parse_llvm_cov_json(json, "x".into(), "0.1.0".into()).unwrap();
        assert_eq!(r.line_pct, 85.0);
        assert_eq!(r.function_pct, 90.0);
        assert_eq!(r.region_pct, 80.0);
        assert_eq!(r.branch_pct, Some(80.0));
        assert_eq!(r.total_lines, 200);
        assert_eq!(r.covered_lines, 170);
        assert!(r.files.is_empty());
    }

    #[test]
    fn parse_llvm_cov_with_files() {
        let json = r#"{
            "type": "llvm.coverage.json.export",
            "version": "2.0.1",
            "data": [{
                "files": [
                    {
                        "filename": "/abs/path/src/lib.rs",
                        "summary": {
                            "lines":     { "count": 100, "covered": 90, "percent": 90.0 },
                            "functions": { "count": 20,  "covered": 18, "percent": 90.0 },
                            "regions":   { "count": 50,  "covered": 42, "percent": 84.0 }
                        }
                    }
                ],
                "totals": {
                    "lines":     { "count": 100, "covered": 90, "percent": 90.0 },
                    "functions": { "count": 20,  "covered": 18, "percent": 90.0 },
                    "regions":   { "count": 50,  "covered": 42, "percent": 84.0 }
                }
            }]
        }"#;
        let r = parse_llvm_cov_json(json, "x".into(), "0.1.0".into()).unwrap();
        assert_eq!(r.files.len(), 1);
        assert_eq!(r.files[0].filename, "/abs/path/src/lib.rs");
        assert_eq!(r.files[0].line_pct, 90.0);
        // No branches section in this fixture.
        assert!(r.branch_pct.is_none());
    }

    #[test]
    fn parse_llvm_cov_rejects_empty_data() {
        let json = r#"{ "type": "llvm.coverage.json.export", "version": "2", "data": [] }"#;
        let r = parse_llvm_cov_json(json, "x".into(), "0.1.0".into());
        assert!(matches!(r, Err(CoverageError::ParseError(_))));
    }

    #[test]
    fn parse_llvm_cov_rejects_garbage() {
        let r = parse_llvm_cov_json("not json", "x".into(), "0.1.0".into());
        assert!(matches!(r, Err(CoverageError::ParseError(_))));
    }

    #[test]
    fn coverage_result_round_trips_through_json() {
        let r = sample_result(85.0, 88.0, 80.0);
        let s = serde_json::to_string(&r).unwrap();
        let back: CoverageResult = serde_json::from_str(&s).unwrap();
        assert_eq!(back.name, r.name);
        assert_eq!(back.line_pct, r.line_pct);
    }

    #[test]
    fn to_baseline_strips_per_file_detail() {
        let mut r = sample_result(85.0, 88.0, 80.0);
        r.files.push(FileCoverage {
            filename: "a.rs".into(),
            line_pct: 50.0,
            function_pct: 50.0,
            region_pct: 50.0,
            total_lines: 10,
            covered_lines: 5,
        });
        let b = r.to_baseline();
        assert_eq!(b.name, "x");
        assert_eq!(b.line_pct, 85.0);
        // Baseline doesn't carry per-file detail.
        let s = serde_json::to_string(&b).unwrap();
        assert!(!s.contains("a.rs"));
    }
}
