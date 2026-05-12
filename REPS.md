# dev-coverage — Project Specification (REPS)

> Rust Engineering Project Specification.
> Normative language follows RFC 2119.

## 1. Purpose

`dev-coverage` MUST measure test coverage and emit results as a
`dev-report::Report`. Output MUST be machine-readable so AI agents and
CI gates can act on it without parsing free-form output.

## 2. Scope

This crate MUST provide:

- A `CoverageRun` builder with `workspace`, `in_dir`, `exclude`,
  `feature`, `all_features`, `no_default_features`, and `per_file`
  toggles.
- A `CoverageResult` carrying line, function, and region percentages,
  raw count totals for each, an optional branch percentage, and an
  optional per-file breakdown.
- A `CoverageThreshold` enum covering line, function, and region
  thresholds.
- A `Baseline` value plus a `BaselineStore` trait, with a default
  filesystem-backed `JsonFileBaselineStore` implementation.
- A `CoverageDiff` type produced by `CoverageResult::diff(&baseline, tolerance_pct)`.
- A `CoverageProducer` that implements `dev_report::Producer` so the
  same shape that drives `dev-bench`, `dev-stress`, etc. can drive
  coverage as well.
- A `CoverageResult::into_check_result(threshold)` adapter producing a
  `dev_report::CheckResult` tagged `coverage` with numeric evidence
  for both the measured and the threshold percentages.

This crate MAY provide later:

- HTML report linking for human review.
- Per-line annotations (e.g. for IDE integration).
- Branch-coverage threshold variants.

This crate MUST NOT:

- Replace `cargo-llvm-cov`. It wraps it.
- Require `cargo-tarpaulin` or other alternatives. Pick one tool;
  the choice is `cargo-llvm-cov`.
- Run tests itself. The harness invokes `cargo test` via
  `cargo-llvm-cov`.

## 3. Determinism

Same source + same test input MUST produce the same coverage
percentages (within the limits of LLVM coverage instrumentation
itself). The crate MUST NOT introduce additional non-determinism into
the measurements or the resulting `CheckResult`.

Two diffs of the same `(current, baseline)` pair MUST be byte-equal.

## 4. Tool dependency

`cargo-llvm-cov` MUST be installed on the system. The crate detects
its absence and emits `CoverageError::ToolNotInstalled`. Subprocess
failures (non-zero exit) MUST surface as
`CoverageError::SubprocessFailed(stderr)`; parse failures MUST surface
as `CoverageError::ParseError(detail)`. Neither MUST cause a panic.

## 5. JSON wire format

The `CoverageResult`, `Baseline`, and `FileCoverage` types MUST be
serializable via `serde_json`. Field names MUST use `snake_case`.
Optional fields with default values (e.g. an empty `files` vector,
`branch_pct = None`) MUST be omitted on serialization so v0.9.0
documents round-trip byte-equivalently when no per-file data is
present.

## 6. Baseline storage

The `BaselineStore` trait MUST:

- Treat `load` as tolerant of missing data: return `Ok(None)` when no
  baseline exists for the given `(scope, name)` pair.
- Make `save` atomic on the same filesystem: a partial write that
  survives a crash MUST NOT result in a corrupt baseline on disk.

`(scope, name)` MUST be treated as the identity of a baseline. Two
baselines with the same `name` but different `scope` values MUST NOT
collide.

## 7. Producer contract

`CoverageProducer::produce()` MUST always return a `Report`. It MUST
NOT panic on subprocess failure; instead, it MUST emit a single
`CheckResult::fail` named `coverage::<subject>` with
`Severity::Critical` and the error message in the `detail` field.

When a baseline is configured, `produce()` MUST emit an additional
`CheckResult` named `coverage::regression::<subject>` carrying the
signed deltas in the `detail` field. The verdict MUST be `Fail
(Severity::Error)` when the diff is flagged as regressed, `Pass`
otherwise.

## 8. Stability

Through `0.9.x` the public API MAY shift. The `1.0` release pins the
API. The wire format of `CoverageResult` and `Baseline`, and the JSON
shape emitted through `dev-report`, MUST stay stable from `1.0`
onward.
