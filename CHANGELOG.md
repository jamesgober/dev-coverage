# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.3] - 2026-11-12

### Changed

- Depends on `dev-report` 0.9.9 from crates.io instead of a sibling
  checkout, so a fresh clone builds and CI tests against the released
  version.

[0.9.3]: https://github.com/jamesgober/dev-coverage/releases/tag/v0.9.3

## [0.9.2] - 2026-10-09

Parser, tool-detection and baseline fixes from a review pass, a run
time limit, and the MSRV rollback to Rust 1.75. The rollback follows
`dev-fixtures` 0.9.5 swapping `tempfile` for `mod-tempdir` 1.0, which
removed the `getrandom 0.4.2 -> edition2024` chain that held the dev-*
collection at 1.85.

### Added

- `CoverageRun::timeout(Duration)`. Without it a hung test blocked
  `execute()` forever. When the limit expires the whole
  `cargo llvm-cov` process tree is killed (`taskkill /T` on Windows, the
  process group on Unix) and reaped.
- `VERSION` constant with the crate version as compiled, so tools that
  bundle this crate can report what is actually linked.

### Fixed

- `branch_pct` was `Some(0.0)` when branch coverage was off: current
  cargo-llvm-cov (0.8.x) always emits a `branches` block, with a count
  of 0 when branches are not measured. It is now `None` in that case.
- A missing `percent` field read as 0%. It is now computed from
  `covered / count`, and is 0 (never NaN) when the count is 0.
- Any failing `cargo llvm-cov --version` was reported as
  `ToolNotInstalled`, and the probe ran in the wrong directory. Only
  cargo's "no such command" error means not installed now; other
  failures become `SubprocessFailed` with stderr. The probe runs in the
  work dir, and a missing work dir gives a clear error.
- `SubprocessFailed` now includes the exit status, not only stderr
  (which could be empty).
- A NaN threshold passed every run, and a NaN tolerance or baseline
  value hid regressions. Both now fail.
- Baseline store: saving deleted the old file before renaming, so a
  save was not atomic, and concurrent saves shared one temp file. The
  rename now replaces the target directly and the temp name is per
  process. `load` no longer races between an existence check and the
  read. A `scope` or `name` of `..`, an absolute path or a drive prefix
  could write outside the store root; those are now rejected with
  `InvalidInput` (nested branch names like `feature/x` still work).
- A unit test ran a full coverage pass of the crate itself when
  cargo-llvm-cov was installed (over three minutes).

### Changed

- `per_file()` dropped `--summary-only` and parsed the full export even
  though the summary already carries per-file totals, and `files` was
  filled on every run although the docs say it stays empty unless
  `per_file()` is set. Every run now uses the summary export, and
  `files` is filled only with `per_file()`, as documented. Callers that
  read `files` without calling `per_file()` need to add that call.
- `rust-version` lowered from `1.85` to `1.75`. CI's MSRV job now
  builds on 1.75 against an MSRV-compatible lockfile; it was still
  pinned to 1.85.

### Documentation

- `exclude()` takes a package spec, not a file pattern; the rustdoc
  example used `"tests/*"`.
- README tagline and the `Cargo.toml` description said "kill rates"
  (mutation-testing wording) instead of coverage.
- New README section on scoping and time limits; MSRV section says 1.75.
- `docs/API.md` brought up to date (missing fields, the `Io` error, and
  examples that did not compile).

[0.9.2]: https://github.com/jamesgober/dev-coverage/releases/tag/v0.9.2

## [0.9.1] - 2026-05-12

Documentation and SEO pass. No code changes.

### Changed

- README header standardized to match the collection-wide template: Rust logo image, MSRV badge between CI and docs.rs (was at the end of the badge list), copyright block at bottom. MSRV badge label switched from lowercase `msrv` to uppercase `MSRV` for visual consistency across the collection.
- Subtitle now reads `CODE COVERAGE WITH REGRESSION GATES` (was `TEST COVERAGE FOR RUST`). Lifts the regression-gating value to the title.
- Tagline rewritten to lead with what the crate does (wraps llvm-cov, gates against a baseline) rather than the part-of-suite framing.
- `## What it does` consumer language widened beyond AI agents.
- `## The dev-* suite` retitled to `The dev-* collection` and expanded from a one-liner to the full 14-crate map.
- `Cargo.toml` description rewritten: lists what the crate measures (line / function / region kill rates), what it stores (baselines), what it emits (verdicts).
- `Cargo.toml` keywords retuned: dropped `verification` and `ai-tools`, added `ci` and `regression` for crates.io search.

### Added

- "Part of the `dev-*` verification collection" block on the README, under the intro, linking the umbrella `dev-tools` crate.

[0.9.1]: https://github.com/jamesgober/dev-coverage/releases/tag/v0.9.1

## [0.9.0] - 2026-05-12

This is the foundation release. Everything below lands together; the
prior `0.1.0` was a name-claim placeholder.

### Added

- Real `cargo llvm-cov --json --summary-only` subprocess integration in `CoverageRun::execute`. Detects tool absence and emits `CoverageError::ToolNotInstalled` without panicking.
- Builder methods on `CoverageRun`: `in_dir(path)`, `workspace()`, `exclude(pattern)`, `feature(name)`, `all_features()`, `no_default_features()`, `per_file()`. Each maps to the corresponding `cargo llvm-cov` flag.
- `CoverageRun::subject()` and `CoverageRun::subject_version()` accessors.
- `CoverageResult` expanded: now carries `total_functions`, `covered_functions`, `total_regions`, `covered_regions`, an optional `branch_pct`, and a `files: Vec<FileCoverage>` per-file breakdown (populated when `per_file()` is set on the run).
- `FileCoverage` type for per-file detail.
- `CoverageResult::diff(&baseline, tolerance_pct) -> CoverageDiff` compares a current run against a stored baseline. The `regressed` flag fires when any of `line`, `function`, or `region` drops by more than the tolerance.
- `CoverageResult::to_baseline()` strips per-file detail down to a `Baseline` ready for persistence.
- `CoverageResult::least_covered_files(n)` returns the lowest-coverage files in ascending order; useful for emitting evidence about hotspots.
- New `baseline` module with the `Baseline`, `BaselineStore`, and `JsonFileBaselineStore` types. `JsonFileBaselineStore` writes one `<root>/<scope>/<name>.json` file per baseline with atomic write-temp-rename semantics so partial writes never corrupt a comparison.
- New `producer` module exposing `CoverageProducer`: a `dev_report::Producer` adapter that wraps a `CoverageRun` + `CoverageThreshold` and (optionally) a baseline. Subprocess failures map to a `CheckResult::fail` named `coverage::<subject>` with `Severity::Critical` — no panics.
- `CoverageDiff` type carrying signed `line_pct_delta`, `function_pct_delta`, `region_pct_delta` plus the `regressed` flag.
- `CoverageError::Io(io::Error)` variant for filesystem failures (baseline reads/writes).
- `From<io::Error> for CoverageError` so `?` works against I/O errors.
- Examples: `with_threshold.rs` (every threshold variant against a constructed result, no subprocess), `baseline.rs` (save → load → diff workflow), `producer.rs` (Producer integration, gated by `DEV_COVERAGE_EXAMPLE_RUN=1`).
- `examples/basic.rs` polished: gracefully handles `CoverageError::ToolNotInstalled` so `cargo run --example basic` exits cleanly even when `cargo-llvm-cov` is absent.
- 28 unit tests across `lib.rs`, `baseline.rs`, and `producer.rs`. Coverage includes: threshold pass/fail paths, function and region thresholds, JSON parsing fixtures (summary-only and per-file), parse-error handling, baseline round-trip through `JsonFileBaselineStore`, scope isolation, overwrite semantics, and `CoverageDiff` sign/tolerance logic.
- 8 integration tests in `tests/smoke.rs`. One real-subprocess test gated by `#[ignore]` — it requires `cargo-llvm-cov` *and* `CARGO_TARGET_DIR` pointing outside the workspace, because the outer `cargo test` already holds the workspace target-dir lock that the inner `cargo llvm-cov` would otherwise block on.

### Changed

- `cargo install cargo-llvm-cov` is now a real runtime requirement (previously declared but the code did not actually invoke it).
- README rewritten: removes the "API shape only; subprocess in 0.9.1" disclaimer, documents the baseline workflow, lists the producer integration, and pins MSRV at 1.85.
- REPS.md tightened: the "SHOULD provide" items (baseline storage, diff against baseline, per-file breakdown) are now MUST-have for 0.9.x.
- CI workflow: new `integration` job installs `cargo-llvm-cov` via `taiki-e/install-action` and verifies the tool runs (`cargo llvm-cov --version`) plus that this crate compiles against the freshly-installed toolchain. The full subprocess pipeline is not exercised in CI because invoking `cargo llvm-cov` from inside `cargo test` deadlocks on the workspace target-dir lock; the JSON parser is verified instead by the fixture-based unit tests. Path-dep `../dev-report` is cloned in every job so sibling-only checkouts work end-to-end. `actions/checkout` is at `v5`.

### Dependencies

- Added: `serde` 1.0 (derive feature), `serde_json` 1.0. Both required for parsing `cargo llvm-cov` JSON output and for serializing baselines.
- Added: `tempfile` 3 as a `dev-dependency` for the baseline round-trip tests.

### Note

`0.1.0` was a name-claim publish with a stub `execute()`. Crates depending on `0.1` will compile against `0.9` because the public API of `0.1.0` (the constructors, the threshold enum, the `into_check_result` flow) is a subset of `0.9.0`'s. Field additions on `CoverageResult` mean direct struct construction must be updated — see the migration block in the README.

[Unreleased]: https://github.com/jamesgober/dev-coverage/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/jamesgober/dev-coverage/releases/tag/v0.9.0
[0.1.0]: https://github.com/jamesgober/dev-coverage/releases/tag/v0.1.0
