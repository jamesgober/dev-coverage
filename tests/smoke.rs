use dev_coverage::{CoverageResult, CoverageRun, CoverageThreshold};

#[test]
fn smoke_run_builds() {
    let _r = CoverageRun::new("test-crate", "0.1.0");
}

#[test]
fn smoke_execute_returns_result() {
    let run = CoverageRun::new("test-crate", "0.1.0");
    let r = run.execute().unwrap();
    assert_eq!(r.name, "test-crate");
    assert_eq!(r.version, "0.1.0");
}

#[test]
fn smoke_threshold_pass_when_coverage_meets_target() {
    let r = CoverageResult {
        name: "x".into(),
        version: "0.1.0".into(),
        line_pct: 85.0,
        function_pct: 90.0,
        region_pct: 80.0,
        total_lines: 100,
        covered_lines: 85,
    };
    let c = r.into_check_result(CoverageThreshold::min_line_pct(80.0));
    assert!(matches!(c.verdict, dev_report::Verdict::Pass));
}

#[test]
fn smoke_threshold_fail_when_below() {
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

#[test]
fn smoke_each_threshold_type_works() {
    let r = CoverageResult {
        name: "x".into(),
        version: "0.1.0".into(),
        line_pct: 85.0,
        function_pct: 90.0,
        region_pct: 75.0,
        total_lines: 100,
        covered_lines: 85,
    };

    let line = r
        .clone()
        .into_check_result(CoverageThreshold::min_line_pct(80.0));
    assert!(matches!(line.verdict, dev_report::Verdict::Pass));

    let func = r
        .clone()
        .into_check_result(CoverageThreshold::min_function_pct(80.0));
    assert!(matches!(func.verdict, dev_report::Verdict::Pass));

    let region = r.into_check_result(CoverageThreshold::min_region_pct(80.0));
    assert!(matches!(region.verdict, dev_report::Verdict::Fail));
}
