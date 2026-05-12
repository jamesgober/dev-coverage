//! Demonstrate every `CoverageThreshold` variant against a constructed
//! `CoverageResult`. No subprocess; no external tooling required.
//!
//! ```text
//! cargo run --example with_threshold
//! ```

use dev_coverage::{CoverageResult, CoverageThreshold};

fn main() {
    let result = CoverageResult {
        name: "demo".into(),
        version: "0.1.0".into(),
        line_pct: 87.5,
        function_pct: 92.0,
        region_pct: 78.0,
        branch_pct: None,
        total_lines: 200,
        covered_lines: 175,
        total_functions: 50,
        covered_functions: 46,
        total_regions: 100,
        covered_regions: 78,
        files: Vec::new(),
    };

    for threshold in [
        CoverageThreshold::min_line_pct(80.0),
        CoverageThreshold::min_function_pct(90.0),
        CoverageThreshold::min_region_pct(80.0),
    ] {
        let check = result.clone().into_check_result(threshold);
        let label = match threshold {
            CoverageThreshold::MinLinePct(p) => format!("line >= {:.1}", p),
            CoverageThreshold::MinFunctionPct(p) => format!("function >= {:.1}", p),
            CoverageThreshold::MinRegionPct(p) => format!("region >= {:.1}", p),
        };
        println!(
            "{:<20} -> {:?} ({})",
            label,
            check.verdict,
            check.detail.as_deref().unwrap_or("(no detail)")
        );
    }
}
