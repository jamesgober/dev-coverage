<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <strong>dev-coverage</strong>
    <br>
    <sup><sub>CODE COVERAGE WITH REGRESSION GATES</sub></sup>
</h1>
<p align="center">
    <a href="https://crates.io/crates/dev-coverage"><img alt="crates.io" src="https://img.shields.io/crates/v/dev-coverage.svg"></a>
    <a href="https://crates.io/crates/dev-coverage"><img alt="downloads" src="https://img.shields.io/crates/d/dev-coverage.svg"></a>
    <a href="https://github.com/jamesgober/dev-coverage/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/jamesgober/dev-coverage/actions/workflows/ci.yml/badge.svg"></a>
    <img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue.svg?style=flat-square" title="Rust Version">
    <a href="https://docs.rs/dev-coverage"><img alt="docs.rs" src="https://docs.rs/dev-coverage/badge.svg"></a>
</p>

<p align="center">
    <strong>Wraps <code>cargo-llvm-cov</code> and gates PR coverage against a stored baseline.</strong> Line, function, and region kill rates as machine-readable verdicts.
</p>

<br>

<div align="center">
    <strong>Part of the <a href="https://crates.io/crates/dev-tools"><code>dev-*</code></a> verification collection.</strong><br>
    <sub>Also available as the <code>coverage</code> feature of the <a href="https://crates.io/crates/dev-tools"><code>dev-tools</code></a> umbrella crate &mdash; one dependency, every verification layer.</sub>
</div>

<br>

---

## What it does

`dev-coverage` drives [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov)
against your project, parses the JSON output, and emits results as a
[`dev-report::Report`](https://docs.rs/dev-report). It compares against a
stored baseline so CI gates, release pipelines, and AI assistants can
act on coverage regressions without scraping free-form text.

## Why a separate crate

Test coverage is the single most direct way to ask "how much of this
code is actually exercised?" Without it, every other quality check is
an opinion. With it, you can answer "did this PR drop line coverage
below 80%?" or "did this PR introduce a 5-point regression vs. main?"
as a yes/no decision.

`dev-coverage` makes those questions programmable, not interactive.

## Quick start

```toml
[dependencies]
dev-coverage = "0.9"
```

One-time tool install:

```bash
cargo install cargo-llvm-cov
```

Drive it from code:

```rust,no_run
use dev_coverage::{CoverageRun, CoverageThreshold};

let run = CoverageRun::new("my-crate", "0.1.0");
let result = run.execute()?;

let threshold = CoverageThreshold::min_line_pct(80.0);
let check = result.into_check_result(threshold);
// `check` is a `dev_report::CheckResult` ready to push into a Report.
# Ok::<(), dev_coverage::CoverageError>(())
```

## Threshold types

| Threshold                          | What it measures                                    |
|------------------------------------|-----------------------------------------------------|
| `CoverageThreshold::MinLinePct`    | Percent of executable lines exercised by tests.     |
| `CoverageThreshold::MinFunctionPct`| Percent of functions called by at least one test.   |
| `CoverageThreshold::MinRegionPct`  | Percent of basic blocks (branch points) exercised.  |

Line coverage is the most common. Region coverage is the strictest.

## Baseline workflow

The headline feature beyond raw measurement: persist a baseline, then
flag regressions on the next run.

```rust,no_run
use dev_coverage::{
    Baseline, BaselineStore, CoverageRun, JsonFileBaselineStore,
};

let run = CoverageRun::new("my-crate", "0.1.0");
let result = run.execute()?;

let store = JsonFileBaselineStore::new("coverage-baselines");

// Compare against last run on main.
if let Some(baseline) = store.load("main", "my-crate")? {
    let diff = result.diff(&baseline, /* tolerance_pct */ 1.0);
    if diff.regressed {
        eprintln!(
            "coverage regressed: line {:+.2}pp, function {:+.2}pp, region {:+.2}pp",
            diff.line_pct_delta,
            diff.function_pct_delta,
            diff.region_pct_delta,
        );
    }
}

// Persist for next time.
store.save("main", &result.to_baseline())?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`JsonFileBaselineStore` writes one `<root>/<scope>/<name>.json` per
baseline with atomic write-temp-rename semantics; a partial write that
survives a crash will not corrupt the comparison on the next run.

## `Producer` integration

`CoverageProducer` plugs coverage into a multi-producer pipeline driven
by [`dev-tools`](https://github.com/jamesgober/dev-tools):

```rust,no_run
use dev_coverage::{CoverageProducer, CoverageRun, CoverageThreshold};
use dev_report::Producer;

let producer = CoverageProducer::new(
    CoverageRun::new("my-crate", "0.1.0"),
    CoverageThreshold::min_line_pct(80.0),
);

let report = producer.produce();
println!("{}", report.to_json().unwrap());
```

When a baseline is wired in via `with_baseline`, the producer pushes a
second `CheckResult` named `coverage::regression::<subject>` carrying
the deltas in its `detail` field, with verdict `Fail (Error)` if the
regression exceeds the tolerance.

## Examples

| File                              | What it shows                                                       |
|-----------------------------------|---------------------------------------------------------------------|
| `examples/basic.rs`               | Run coverage against the current crate; emit a `CheckResult`.       |
| `examples/with_threshold.rs`      | Every `CoverageThreshold` variant against a constructed result.     |
| `examples/baseline.rs`            | Save a baseline, then diff a new run against it.                    |
| `examples/producer.rs`            | Wrap a run in `CoverageProducer` (gated by `DEV_COVERAGE_EXAMPLE_RUN`). |

## Requirements

[`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) must be
installed on the system. The crate detects absence and surfaces a
`CoverageError::ToolNotInstalled` rather than panicking.

```bash
cargo install cargo-llvm-cov
```

The crate's own dependency footprint is small: `dev-report`, `serde`,
`serde_json`.

## Migration from `0.1.0`

`CoverageResult` gained `branch_pct`, `total_functions`,
`covered_functions`, `total_regions`, `covered_regions`, and `files`.
If you constructed `CoverageResult` literals in `0.1.0`, fill in the
new fields:

```rust
# use dev_coverage::CoverageResult;
let _r = CoverageResult {
    name: "x".into(),
    version: "0.1.0".into(),
    line_pct: 85.0,
    function_pct: 90.0,
    region_pct: 80.0,
    // new in 0.9.0:
    branch_pct: None,
    total_lines: 100,
    covered_lines: 85,
    total_functions: 20,
    covered_functions: 18,
    total_regions: 50,
    covered_regions: 40,
    files: Vec::new(),
};
```

The constructor surface (`CoverageRun::new`, `CoverageThreshold::min_*`,
`CoverageResult::into_check_result`) is unchanged.

## The `dev-*` collection

`dev-coverage` ships independently and is also re-exported by the
[`dev-tools`](https://crates.io/crates/dev-tools) umbrella crate as
the `coverage` feature. Sister crates cover the other verification
dimensions:

- [`dev-report`](https://crates.io/crates/dev-report) &mdash; report schema everything emits
- [`dev-fixtures`](https://crates.io/crates/dev-fixtures) &mdash; deterministic test fixtures
- [`dev-bench`](https://crates.io/crates/dev-bench) &mdash; performance and regression detection
- [`dev-async`](https://crates.io/crates/dev-async) &mdash; async runtime verification
- [`dev-stress`](https://crates.io/crates/dev-stress) &mdash; stress and soak workloads
- [`dev-chaos`](https://crates.io/crates/dev-chaos) &mdash; fault injection and recovery testing
- [`dev-security`](https://crates.io/crates/dev-security) &mdash; CVE / license / banned-crate audit
- [`dev-deps`](https://crates.io/crates/dev-deps) &mdash; unused / outdated dep detection
- [`dev-ci`](https://crates.io/crates/dev-ci) &mdash; GitHub Actions workflow generator
- [`dev-fuzz`](https://crates.io/crates/dev-fuzz) &mdash; fuzz testing workflow
- [`dev-flaky`](https://crates.io/crates/dev-flaky) &mdash; flaky-test detection
- [`dev-mutate`](https://crates.io/crates/dev-mutate) &mdash; mutation testing

## Status

`v0.9.x` is the pre-1.0 stabilization line. The API is feature-complete
for coverage measurement, baseline storage, and regression detection.
Production use is fine; `1.0` will pin the public API and the wire
format.

## Minimum supported Rust version

`1.85` — pinned in `Cargo.toml` via `rust-version` and verified by the
MSRV job in CI.

## License

Apache-2.0. See [LICENSE](LICENSE).




<!-- COPYRIGHT
---------------------------------->
<div align="center">
    <br>
    <h2></h2>
    Copyright &copy; 2026 James Gober.
</div>
