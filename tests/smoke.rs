//! Public-API smoke tests.
//!
//! `execute()` against a real `cargo llvm-cov` binary is exercised in
//! an `#[ignore]` test below; the rest construct values directly so the
//! shape of `CoverageResult` and the threshold paths are covered without
//! needing the tool installed.

use dev_coverage::{Baseline, CoverageResult, CoverageRun, CoverageThreshold, FileCoverage};
use dev_report::Verdict;

fn fixture(line: f64, func: f64, region: f64) -> CoverageResult {
    CoverageResult {
        name: "x".into(),
        version: "0.1.0".into(),
        line_pct: line,
        function_pct: func,
        region_pct: region,
        branch_pct: None,
        total_lines: 100,
        covered_lines: line as u64,
        total_functions: 20,
        covered_functions: (func / 5.0) as u64,
        total_regions: 50,
        covered_regions: (region / 2.0) as u64,
        files: Vec::new(),
    }
}

#[test]
fn run_builds_with_full_builder_chain() {
    let _ = CoverageRun::new("test-crate", "0.1.0")
        .workspace()
        .all_features()
        .exclude("tests/*")
        .feature("alpha")
        .per_file();
}

#[test]
fn run_accessors_round_trip_args_passed_to_new() {
    let run = CoverageRun::new("alpha", "1.2.3");
    assert_eq!(run.subject(), "alpha");
    assert_eq!(run.subject_version(), "1.2.3");
}

#[test]
fn threshold_pass_when_meets_target() {
    let c = fixture(85.0, 90.0, 80.0).into_check_result(CoverageThreshold::min_line_pct(80.0));
    assert_eq!(c.verdict, Verdict::Pass);
}

#[test]
fn threshold_fail_when_below() {
    let c = fixture(50.0, 60.0, 40.0).into_check_result(CoverageThreshold::min_line_pct(80.0));
    assert_eq!(c.verdict, Verdict::Fail);
}

#[test]
fn each_threshold_type_works() {
    let r = fixture(85.0, 90.0, 75.0);
    let line = r
        .clone()
        .into_check_result(CoverageThreshold::min_line_pct(80.0));
    let func = r
        .clone()
        .into_check_result(CoverageThreshold::min_function_pct(80.0));
    let region = r.into_check_result(CoverageThreshold::min_region_pct(80.0));
    assert_eq!(line.verdict, Verdict::Pass);
    assert_eq!(func.verdict, Verdict::Pass);
    assert_eq!(region.verdict, Verdict::Fail);
}

#[test]
fn diff_against_baseline_signs_correctly() {
    let r = fixture(75.0, 80.0, 70.0);
    let baseline = Baseline {
        name: "x".into(),
        line_pct: 80.0,
        function_pct: 85.0,
        region_pct: 75.0,
    };
    let d = r.diff(&baseline, 0.0);
    assert!(d.regressed);
    assert!(d.line_pct_delta < 0.0);
}

#[test]
fn to_baseline_drops_per_file_detail() {
    let mut r = fixture(80.0, 85.0, 75.0);
    r.files = vec![FileCoverage {
        filename: "src/lib.rs".into(),
        line_pct: 50.0,
        function_pct: 50.0,
        region_pct: 50.0,
        total_lines: 10,
        covered_lines: 5,
    }];
    let b = r.to_baseline();
    assert_eq!(b.name, "x");
    assert_eq!(b.line_pct, 80.0);
}

/// Real subprocess test. Only runs when `cargo-llvm-cov` is installed.
///
/// **Important:** when running this test from a `cargo test` invocation
/// inside this same crate, set `CARGO_TARGET_DIR` to a path *outside*
/// the workspace. Otherwise the outer `cargo test` holds the lock on
/// `target/` and the inner `cargo llvm-cov` blocks waiting for it
/// indefinitely (cargo's well-known target-dir lock).
///
/// ```text
/// CARGO_TARGET_DIR=/tmp/llvm-cov-target cargo test -- --ignored
/// ```
///
/// CI runs this test? No. The deadlock makes it impractical there; the
/// JSON parser is covered by the fixtures in `src/lib.rs`. This test is
/// here as a local smoke check for contributors who want to verify the
/// end-to-end pipeline against their own `cargo-llvm-cov` install.
#[test]
#[ignore = "requires cargo-llvm-cov + CARGO_TARGET_DIR outside the workspace"]
fn execute_against_real_llvm_cov() {
    let run = CoverageRun::new("dev-coverage", "0.9.0");
    let r = run.execute().expect("cargo-llvm-cov is installed");
    assert!(r.line_pct >= 0.0);
    assert!(r.line_pct <= 100.0);
}
