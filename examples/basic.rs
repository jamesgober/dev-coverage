//! Minimal example: run coverage and emit a CheckResult.
//!
//! Run with: `cargo run --example basic`

use dev_coverage::{CoverageRun, CoverageThreshold};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let run = CoverageRun::new("example", "0.1.0");
    let result = run.execute()?;

    println!("Line coverage:     {:.2}%", result.line_pct);
    println!("Function coverage: {:.2}%", result.function_pct);
    println!("Region coverage:   {:.2}%", result.region_pct);

    let threshold = CoverageThreshold::min_line_pct(80.0);
    let check = result.into_check_result(threshold);
    println!("\nVerdict: {:?}", check.verdict);
    if let Some(d) = check.detail {
        println!("Detail:  {d}");
    }
    Ok(())
}
