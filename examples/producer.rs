//! Use `CoverageProducer` to integrate coverage into a multi-producer
//! pipeline.
//!
//! Constructs the producer and prints the API surface. The actual
//! subprocess call is gated behind `DEV_COVERAGE_EXAMPLE_RUN=1` so this
//! example does not spawn `cargo llvm-cov` on every CI invocation.
//!
//! ```text
//! cargo run --example producer
//! DEV_COVERAGE_EXAMPLE_RUN=1 cargo run --example producer
//! ```

use dev_coverage::{CoverageProducer, CoverageRun, CoverageThreshold};
use dev_report::Producer;

fn main() {
    let producer = CoverageProducer::new(
        CoverageRun::new("my-crate", "0.1.0"),
        CoverageThreshold::min_line_pct(80.0),
    );
    println!("Constructed CoverageProducer for 'my-crate' v0.1.0.");

    if std::env::var("DEV_COVERAGE_EXAMPLE_RUN").is_ok() {
        let report = producer.produce();
        println!("{}", report.to_json().expect("serialize report"));
    } else {
        println!("Set DEV_COVERAGE_EXAMPLE_RUN=1 to spawn `cargo llvm-cov`");
        println!("in the current directory and print the resulting JSON report.");
    }
}
