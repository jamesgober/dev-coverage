//! Save a baseline, then diff a new run against it.
//!
//! ```text
//! cargo run --example baseline
//! ```
//!
//! No subprocess; the example constructs `CoverageResult` values
//! directly to keep the focus on the baseline + diff workflow.

use dev_coverage::{Baseline, BaselineStore, CoverageResult, JsonFileBaselineStore};

fn fake_result(line: f64, func: f64, region: f64) -> CoverageResult {
    CoverageResult {
        name: "demo".into(),
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

fn main() -> std::io::Result<()> {
    let tmp = std::env::temp_dir().join("dev-coverage-baseline-example");
    std::fs::create_dir_all(&tmp)?;
    let store = JsonFileBaselineStore::new(&tmp);

    // Establish a baseline.
    let yesterday = fake_result(85.0, 90.0, 80.0);
    store.save("main", &yesterday.to_baseline())?;
    println!(
        "Saved baseline at {}",
        store.path_for("main", "demo").display()
    );

    // Today's run drops below the baseline.
    let today = fake_result(82.0, 88.5, 76.0);
    let baseline: Baseline = store
        .load("main", "demo")?
        .expect("baseline written one line ago");

    let tolerance_pct = 1.0;
    let diff = today.diff(&baseline, tolerance_pct);
    println!("\nDeltas vs. baseline (tolerance {:.2}pp):", tolerance_pct);
    println!("  line     {:+.2}pp", diff.line_pct_delta);
    println!("  function {:+.2}pp", diff.function_pct_delta);
    println!("  region   {:+.2}pp", diff.region_pct_delta);
    println!("\nRegressed: {}", diff.regressed);

    // Don't leave temp files behind.
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(())
}
