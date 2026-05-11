# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-05-11

### Added

- Initial crate skeleton.
- `CoverageRun` builder.
- `CoverageResult` with `line_pct`, `function_pct`, `region_pct`,
  `total_lines`, `covered_lines`.
- `CoverageThreshold` enum: `MinLinePct`, `MinFunctionPct`,
  `MinRegionPct`.
- `CoverageResult::into_check_result(threshold)` produces a
  `dev-report::CheckResult`.
- `CoverageError` for tool-missing / subprocess / parse failures.
- Smoke tests covering pass and fail threshold paths.

### Note

This is the name-claim release. The actual `cargo-llvm-cov` subprocess
integration lands in `0.9.1`. `CoverageRun::execute` returns a
zero-coverage stub result for now.

[Unreleased]: https://github.com/jamesgober/dev-coverage/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/jamesgober/dev-coverage/releases/tag/v0.9.0
