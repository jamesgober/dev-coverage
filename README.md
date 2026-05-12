<h1 align="center">
    <strong>dev-coverage</strong>
    <br>
    <sup><sub>TEST COVERAGE FOR RUST</sub></sup>
</h1>

<p align="center">
    <a href="https://crates.io/crates/dev-coverage"><img alt="crates.io" src="https://img.shields.io/crates/v/dev-coverage.svg"></a>
    <a href="https://crates.io/crates/dev-coverage"><img alt="downloads" src="https://img.shields.io/crates/d/dev-coverage.svg"></a>
    <a href="https://docs.rs/dev-coverage"><img alt="docs.rs" src="https://docs.rs/dev-coverage/badge.svg"></a>
    <a href="https://github.com/jamesgober/dev-coverage/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/jamesgober/dev-coverage/actions/workflows/ci.yml/badge.svg"></a>
    <img alt="MSRV" src="https://img.shields.io/badge/msrv-1.85%2B-blue.svg?style=flat-square" title="Rust Version">
</p>

<p align="center">
    Test coverage measurement and regression detection.<br>
    Part of the <code>dev-*</code> verification suite.
</p>

---

## What it does

`dev-coverage` runs `cargo-llvm-cov` against your project, parses the
output, and emits results as `dev-report::Report`. It detects coverage
regressions against a stored baseline and produces a `CheckResult`
that AI agents and CI gates can act on.

## Why a separate crate

Test coverage is the single most important metric for understanding
test quality. Without it, you don't know how much of your code is
actually exercised. With it, you can ask: "did this PR drop coverage
below 80%?" and get a yes/no answer.

`dev-coverage` makes that question programmable, not interactive.

## Quick start

```toml
[dependencies]
dev-coverage = "0.9"
```

```rust
use dev_coverage::{CoverageRun, CoverageThreshold};

let run = CoverageRun::new("my-crate", "0.1.0");
let result = run.execute()?;

let threshold = CoverageThreshold::min_line_pct(80.0);
let check = result.into_check_result(threshold);
// check is a dev_report::CheckResult ready to push into a Report.
# Ok::<(), dev_coverage::CoverageError>(())
```

## Requirements

`cargo-llvm-cov` must be installed on the system:

```bash
cargo install cargo-llvm-cov
```

This is the only required external tool. The crate itself has no
dependencies beyond `dev-report`.

## Threshold types

| Threshold                          | What it measures                                     |
|------------------------------------|------------------------------------------------------|
| `CoverageThreshold::MinLinePct`    | Percent of executable lines exercised by tests.      |
| `CoverageThreshold::MinFunctionPct`| Percent of functions called by at least one test.    |
| `CoverageThreshold::MinRegionPct`  | Percent of basic blocks (branch points) exercised.   |

Line coverage is the most common; region coverage is the most strict.

## The `dev-*` suite

See [`dev-tools`](https://github.com/jamesgober/dev-tools) for the
full suite.

## Status

`v0.9.0` is the foundation release: API shape is defined, the
`cargo-llvm-cov` integration lands in `0.9.1`. Production use is
discouraged until `1.0`.

## Minimum supported Rust version

`1.85` — pinned in `Cargo.toml` and verified by CI.

## License

Apache-2.0. See [LICENSE](LICENSE).
