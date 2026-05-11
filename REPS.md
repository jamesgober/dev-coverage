# dev-coverage — Project Specification (REPS)

> Rust Engineering Project Specification.
> Normative language follows RFC 2119.

## 1. Purpose

`dev-coverage` MUST measure test coverage and emit results as
`dev-report::Report`. Output MUST be machine-readable so AI agents
and CI gates can act on it without parsing free-form output.

## 2. Scope

This crate MUST provide:

- A `CoverageRun` builder.
- A `CoverageResult` with at minimum line, function, and region
  percentages plus raw line counts.
- A `CoverageThreshold` enum covering line, function, and region
  thresholds.
- A `CheckResult` integration via `into_check_result`.

This crate SHOULD provide (later versions):

- Baseline storage (per-commit-hash JSON files).
- Diff against baseline with configurable regression tolerance.
- Per-file breakdown when available from the underlying tool.
- HTML report linking for human review.

This crate MUST NOT:

- Replace `cargo-llvm-cov`. It wraps it.
- Require `cargo-tarpaulin` or other alternatives. Pick one tool;
  the choice is `cargo-llvm-cov`.
- Run tests itself. The harness invokes `cargo test` through
  `cargo-llvm-cov`.

## 3. Determinism

Same source + same test input MUST produce the same coverage
percentages (within the limits of LLVM coverage instrumentation
itself). The crate MUST NOT introduce additional non-determinism.

## 4. Tool dependency

`cargo-llvm-cov` MUST be installed on the system. The crate detects
its absence and emits a clear `CoverageError::ToolNotInstalled`.

## 5. Stability

Through `0.9.x` the public API MAY shift. The `1.0` release pins the
API. The wire format of `CoverageResult` and the JSON shape emitted
through `dev-report` MUST stay stable from `1.0` onward.