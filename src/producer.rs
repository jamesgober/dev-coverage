//! [`Producer`] integration: wrap a [`CoverageRun`] + [`CoverageThreshold`]
//! and emit a [`Report`] every time the producer runs.
//!
//! [`Producer`]: dev_report::Producer
//! [`Report`]: dev_report::Report

use dev_report::{CheckResult, Producer, Report, Severity};

use crate::{Baseline, CoverageRun, CoverageThreshold};

/// `Producer` adapter that runs a [`CoverageRun`], compares the result
/// against the configured [`CoverageThreshold`], and (optionally) flags
/// regressions against a stored [`Baseline`].
///
/// Subprocess failures map to a failing `CheckResult` named
/// `coverage::<subject>` with `Severity::Critical`. No panics.
///
/// # Example
///
/// ```no_run
/// use dev_coverage::{CoverageProducer, CoverageRun, CoverageThreshold};
/// use dev_report::Producer;
///
/// let producer = CoverageProducer::new(
///     CoverageRun::new("my-crate", "0.1.0"),
///     CoverageThreshold::min_line_pct(80.0),
/// );
/// let report = producer.produce();
/// println!("{}", report.to_json().unwrap());
/// ```
pub struct CoverageProducer {
    run: CoverageRun,
    threshold: CoverageThreshold,
    baseline: Option<Baseline>,
    regression_tolerance_pct: f64,
}

impl CoverageProducer {
    /// Build a producer with a threshold only.
    pub fn new(run: CoverageRun, threshold: CoverageThreshold) -> Self {
        Self {
            run,
            threshold,
            baseline: None,
            regression_tolerance_pct: 0.0,
        }
    }

    /// Compare each run against the given baseline. When the current
    /// result regresses by more than `tolerance_pct`, a separate
    /// `coverage::regression` check is pushed alongside the threshold check.
    pub fn with_baseline(mut self, baseline: Baseline, tolerance_pct: f64) -> Self {
        self.baseline = Some(baseline);
        self.regression_tolerance_pct = tolerance_pct;
        self
    }
}

impl Producer for CoverageProducer {
    fn produce(&self) -> Report {
        let subject = self.run.subject().to_string();
        let version = self.run.subject_version().to_string();
        let mut report = Report::new(&subject, &version).with_producer("dev-coverage");
        match self.run.execute() {
            Ok(result) => {
                if let Some(baseline) = &self.baseline {
                    let diff = result.diff(baseline, self.regression_tolerance_pct);
                    let detail = format!(
                        "line {:+.2}pp, function {:+.2}pp, region {:+.2}pp (tolerance {:.2}pp)",
                        diff.line_pct_delta,
                        diff.function_pct_delta,
                        diff.region_pct_delta,
                        self.regression_tolerance_pct
                    );
                    let regression_check = if diff.regressed {
                        CheckResult::fail(
                            format!("coverage::regression::{}", subject),
                            Severity::Error,
                        )
                        .with_detail(detail)
                        .with_tag("coverage")
                        .with_tag("regression")
                    } else {
                        CheckResult::pass(format!("coverage::regression::{}", subject))
                            .with_detail(detail)
                            .with_tag("coverage")
                    };
                    report.push(regression_check);
                }
                report.push(result.into_check_result(self.threshold));
            }
            Err(e) => {
                let detail = e.to_string();
                let check = CheckResult::fail(format!("coverage::{}", subject), Severity::Critical)
                    .with_detail(detail)
                    .with_tag("coverage")
                    .with_tag("subprocess");
                report.push(check);
            }
        }
        report.finish();
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dev_report::Verdict;

    #[test]
    fn produce_maps_run_failure_to_critical_fail() {
        // Point the run at an empty directory so it fails fast whether or
        // not cargo-llvm-cov is installed (no Cargo.toml to measure).
        // Running it against this crate would recursively measure the
        // crate's own test suite, which takes minutes.
        let dir = tempfile::tempdir().unwrap();
        let producer = CoverageProducer::new(
            CoverageRun::new("self", "0.0.0").in_dir(dir.path()),
            CoverageThreshold::min_line_pct(80.0),
        );
        let report = producer.produce();
        assert_eq!(report.subject, "self");
        assert_eq!(report.checks.len(), 1);
        let check = &report.checks[0];
        assert_eq!(check.name, "coverage::self");
        assert_eq!(check.verdict, Verdict::Fail);
        assert_eq!(check.severity, Some(Severity::Critical));
        assert!(check.has_tag("subprocess"));
    }

    #[test]
    fn produce_maps_missing_workdir_to_critical_fail() {
        let producer = CoverageProducer::new(
            CoverageRun::new("self", "0.0.0").in_dir("definitely/not/a/dir/7f3a"),
            CoverageThreshold::min_line_pct(80.0),
        )
        .with_baseline(
            Baseline {
                name: "self".into(),
                line_pct: 80.0,
                function_pct: 80.0,
                region_pct: 80.0,
            },
            1.0,
        );
        let report = producer.produce();
        // No regression check without a result to compare.
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].verdict, Verdict::Fail);
        assert!(report.checks[0]
            .detail
            .as_deref()
            .unwrap_or("")
            .contains("does not exist"));
    }
}
