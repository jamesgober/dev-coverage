# dev-coverage — API Reference

> Hand-written reference. Mirrors `cargo doc --open` output but with
> curated examples and structure. The rustdoc on docs.rs is the
> authoritative source.

## Table of contents

- [`CoverageRun`](#coveragerun)
  - [`CoverageRun::new`](#coveragerunnew)
  - [Builder methods](#builder-methods)
  - [`CoverageRun::execute`](#coveragerunexecute)
- [`CoverageResult`](#coverageresult)
  - [Fields](#coverageresult-fields)
  - [`CoverageResult::into_check_result`](#coverageresultinto_check_result)
  - [`CoverageResult::diff`](#coverageresultdiff)
  - [`CoverageResult::to_baseline`](#coverageresultto_baseline)
  - [`CoverageResult::least_covered_files`](#coverageresultleast_covered_files)
- [`FileCoverage`](#filecoverage)
- [`CoverageThreshold`](#coveragethreshold)
- [`CoverageDiff`](#coveragediff)
- [`Baseline`, `BaselineStore`, `JsonFileBaselineStore`](#baselines)
- [`CoverageProducer`](#coverageproducer)
- [`CoverageError`](#coverageerror)

---

## `CoverageRun`

```rust
pub struct CoverageRun { /* private */ }
```

Configuration for a `cargo llvm-cov --summary-only --json` run. Holds
the subject name and version so the resulting `CheckResult` carries
identifying information.

### `CoverageRun::new`

```rust
pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self
```

| Parameter | Type                | Description                                        |
|-----------|---------------------|----------------------------------------------------|
| `name`    | `impl Into<String>` | Subject name. Descriptive; cargo picks the package. |
| `version` | `impl Into<String>` | Subject version. Descriptive.                      |

### Builder methods

| Method                          | Effect                                                                 |
|---------------------------------|------------------------------------------------------------------------|
| `in_dir(dir)`                   | Run from `dir` instead of the current directory.                        |
| `workspace()`                   | Pass `--workspace`.                                                     |
| `exclude(spec)`                 | Pass `--exclude <spec>`: a workspace package to leave out (not a path). |
| `feature(name)`                 | Pass `--features <name>`. Repeatable.                                   |
| `all_features()`                | Pass `--all-features`.                                                  |
| `no_default_features()`         | Pass `--no-default-features`.                                           |
| `per_file()`                    | Keep the per-file breakdown in `CoverageResult::files`.                 |
| `timeout(limit)`                | Kill the whole process tree if the run exceeds `limit`.                 |
| `subject()` / `subject_version()` | Read back the values passed to `new`.                               |

### `CoverageRun::execute`

```rust
pub fn execute(&self) -> Result<CoverageResult, CoverageError>
```

Returns:
- `Ok(CoverageResult)` on success.
- `Err(CoverageError::ToolNotInstalled)` if `cargo` cannot be started or
  reports that the `llvm-cov` subcommand does not exist.
- `Err(CoverageError::SubprocessFailed)` if the working directory is
  missing, the run exits non-zero (for example a failing test), or the
  `timeout` expires (message starts with `timed out after`).
- `Err(CoverageError::ParseError)` if the output is not the expected JSON.

```rust
use dev_coverage::CoverageRun;

let run = CoverageRun::new("my-crate", "0.1.0");
let result = run.execute()?;
println!("Line coverage: {:.2}%", result.line_pct);
# Ok::<(), dev_coverage::CoverageError>(())
```

---

## `CoverageResult`

### CoverageResult fields

| Field               | Type                | Description                                                        |
|---------------------|---------------------|--------------------------------------------------------------------|
| `name`              | `String`            | Subject name from `CoverageRun`.                                   |
| `version`           | `String`            | Subject version.                                                   |
| `line_pct`          | `f64`               | Percent of executable lines exercised (0.0-100.0).                 |
| `function_pct`      | `f64`               | Percent of functions called by at least one test.                  |
| `region_pct`        | `f64`               | Percent of code regions exercised.                                 |
| `branch_pct`        | `Option<f64>`       | Branch percentage; `None` unless branch coverage was collected.    |
| `total_lines`       | `u64`               | Total executable lines counted.                                    |
| `covered_lines`     | `u64`               | Lines exercised at least once.                                     |
| `total_functions`   | `u64`               | Total functions.                                                   |
| `covered_functions` | `u64`               | Functions called at least once.                                    |
| `total_regions`     | `u64`               | Total regions.                                                     |
| `covered_regions`   | `u64`               | Regions exercised at least once.                                   |
| `files`             | `Vec<FileCoverage>` | Per-file breakdown; empty unless the run used `per_file()`.        |

A percentage is `0.0` (never NaN) when nothing of that kind was counted.

### `CoverageResult::into_check_result`

```rust
pub fn into_check_result(self, threshold: CoverageThreshold) -> CheckResult
```

Pass when the measured percentage meets the threshold; fail with
`Severity::Warning` otherwise (a NaN threshold also fails). The check
is named `coverage::<name>`, tagged `coverage`, and carries numeric
evidence `<kind>_pct`, `<kind>_pct_threshold`, `total_lines`,
`covered_lines`.

### `CoverageResult::diff`

```rust
pub fn diff(&self, baseline: &Baseline, tolerance_pct: f64) -> CoverageDiff
```

Signed deltas (current minus baseline) for line, function and region
coverage. `regressed` is `true` when any delta is below
`-tolerance_pct`, or when any value involved is NaN.

### `CoverageResult::to_baseline`

```rust
pub fn to_baseline(&self) -> Baseline
```

The three headline percentages, ready to persist.

### `CoverageResult::least_covered_files`

```rust
pub fn least_covered_files(&self, n: usize) -> Vec<&FileCoverage>
```

The `n` files with the lowest line coverage, ascending. Empty when
`files` is empty.

---

## `FileCoverage`

| Field           | Type     | Description                                     |
|-----------------|----------|-------------------------------------------------|
| `filename`      | `String` | Absolute path as emitted by `cargo llvm-cov`.   |
| `line_pct`      | `f64`    | Line coverage for this file.                    |
| `function_pct`  | `f64`    | Function coverage for this file.                |
| `region_pct`    | `f64`    | Region coverage for this file.                  |
| `total_lines`   | `u64`    | Executable lines in this file.                  |
| `covered_lines` | `u64`    | Lines in this file exercised at least once.     |

---

## `CoverageThreshold`

```rust
pub enum CoverageThreshold {
    MinLinePct(f64),
    MinFunctionPct(f64),
    MinRegionPct(f64),
}
```

Built with `CoverageThreshold::min_line_pct`, `min_function_pct` and
`min_region_pct`. Each fails when the matching percentage is below the
given value.

---

## `CoverageDiff`

| Field                | Type   | Description                                   |
|----------------------|--------|-----------------------------------------------|
| `line_pct_delta`     | `f64`  | Current minus baseline line percentage.       |
| `function_pct_delta` | `f64`  | Current minus baseline function percentage.   |
| `region_pct_delta`   | `f64`  | Current minus baseline region percentage.     |
| `regressed`          | `bool` | `true` when any delta is worse than tolerance. |

---

## Baselines

`Baseline` holds `name`, `line_pct`, `function_pct`, `region_pct`.

`BaselineStore` is the storage trait:

```rust
fn load(&self, scope: &str, name: &str) -> io::Result<Option<Baseline>>;
fn save(&self, scope: &str, baseline: &Baseline) -> io::Result<()>;
```

`JsonFileBaselineStore::new(root)` stores each baseline at
`<root>/<scope>/<name>.json` (see `path_for`). `load` returns `Ok(None)`
when the file does not exist. `save` writes a temp file and renames it
over the target, so readers see either the old or the new baseline.

---

## `CoverageProducer`

```rust
pub fn new(run: CoverageRun, threshold: CoverageThreshold) -> Self
pub fn with_baseline(self, baseline: Baseline, tolerance_pct: f64) -> Self
```

Implements `dev_report::Producer`. A successful run pushes the
threshold check (and, with a baseline, a `coverage::regression::<name>`
check that fails with `Severity::Error` on regression). A failed run
pushes one `coverage::<name>` check that fails with `Severity::Critical`
and is tagged `coverage` and `subprocess`.

---

## `CoverageError`

```rust
pub enum CoverageError {
    ToolNotInstalled,
    SubprocessFailed(String),
    ParseError(String),
    Io(std::io::Error),
}
```

| Variant            | Meaning                                                                 |
|--------------------|-------------------------------------------------------------------------|
| `ToolNotInstalled` | `cargo-llvm-cov` is missing. Fix: `cargo install cargo-llvm-cov`.       |
| `SubprocessFailed` | The run failed, timed out, or could not start; carries exit status and stderr. |
| `ParseError`       | The output was not the expected JSON; carries the parser message.       |
| `Io`               | I/O error (available for callers mapping baseline-store errors).        |
