//! Run coverage on the current crate, print the percentages, emit a `CheckResult`.
//!
//! ```text
//! cargo install cargo-llvm-cov   # one-time setup
//! cargo run --example basic
//! ```
//!
//! Requires `cargo-llvm-cov` to be installed. If it is not, the example
//! prints a clear error and exits 0 (so `cargo build --examples` in CI
//! still succeeds without the tool).

use dev_coverage::{CoverageError, CoverageRun, CoverageThreshold};

fn main() {
    let run = CoverageRun::new("example", "0.1.0");
    let result = match run.execute() {
        Ok(r) => r,
        Err(CoverageError::ToolNotInstalled) => {
            eprintln!("cargo-llvm-cov is not installed; skipping the example.");
            eprintln!("Install with: cargo install cargo-llvm-cov");
            return;
        }
        Err(e) => {
            eprintln!("coverage run failed: {e}");
            return;
        }
    };

    println!("Line coverage:     {:.2}%", result.line_pct);
    println!("Function coverage: {:.2}%", result.function_pct);
    println!("Region coverage:   {:.2}%", result.region_pct);

    let threshold = CoverageThreshold::min_line_pct(80.0);
    let check = result.into_check_result(threshold);
    println!("\nVerdict: {:?}", check.verdict);
    if let Some(d) = check.detail {
        println!("Detail:  {d}");
    }
}
