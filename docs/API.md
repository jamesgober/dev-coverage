# dev-coverage — API Reference

> Hand-written reference. Mirrors `cargo doc --open` output but with
> curated examples and structure.

## Table of contents

- [`CoverageRun`](#coveragerun)
  - [`CoverageRun::new`](#coveragerunnew)
  - [`CoverageRun::execute`](#coveragerunexecute)
- [`CoverageResult`](#coverageresult)
  - [Fields](#coverageresult-fields)
  - [`CoverageResult::into_check_result`](#coverageresultinto_check_result)
- [`CoverageThreshold`](#coveragethreshold)
  - [`CoverageThreshold::MinLinePct`](#coveragethresholdminlinepct)
  - [`CoverageThreshold::MinFunctionPct`](#coveragethresholdminfunctionpct)
  - [`CoverageThreshold::MinRegionPct`](#coveragethresholdminregionpct)
  - [`CoverageThreshold::min_line_pct`](#coveragethresholdmin_line_pct)
  - [`CoverageThreshold::min_function_pct`](#coveragethresholdmin_function_pct)
  - [`CoverageThreshold::min_region_pct`](#coveragethresholdmin_region_pct)
- [`CoverageError`](#coverageerror)
  - [`CoverageError::ToolNotInstalled`](#coverageerrortoolnotinstalled)
  - [`CoverageError::SubprocessFailed`](#coverageerrorsubprocessfailed)
  - [`CoverageError::ParseError`](#coverageerrorparseerror)

---

## `CoverageRun`

```rust
pub struct CoverageRun { /* private */ }
```

Configuration for a coverage measurement run. Holds the crate name
and version so the resulting `CheckResult` carries identifying
information.

### `CoverageRun::new`

```rust
pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self
```

Begin a new coverage run.

| Parameter | Type                  | Description                          |
|-----------|-----------------------|--------------------------------------|
| `name`    | `impl Into<String>`   | Crate name being measured.           |
| `version` | `impl Into<String>`   | Crate version at measurement time.   |

```rust
use dev_coverage::CoverageRun;

let run = CoverageRun::new("my-crate", "0.1.0");
```

### `CoverageRun::execute`

```rust
pub fn execute(&self) -> Result<CoverageResult, CoverageError>
```

Run coverage measurement against the current project. Invokes
`cargo-llvm-cov` under the hood.

Returns:
- `Ok(CoverageResult)` on success.
- `Err(CoverageError::ToolNotInstalled)` if `cargo-llvm-cov` is
  missing.
- `Err(CoverageError::SubprocessFailed)` if the run fails.
- `Err(CoverageError::ParseError)` if the output is malformed.

```rust
use dev_coverage::CoverageRun;

let run = CoverageRun::new("my-crate", "0.1.0");
let result = run.execute()?;
println!("Line coverage: {:.2}%", result.line_pct);
# Ok::<(), dev_coverage::CoverageError>(())
```

---

## `CoverageResult`

```rust
pub struct CoverageResult {
    pub name: String,
    pub version: String,
    pub line_pct: f64,
    pub function_pct: f64,
    pub region_pct: f64,
    pub total_lines: u64,
    pub covered_lines: u64,
}
```

### CoverageResult fields

| Field            | Type     | Description                                                       |
|------------------|----------|-------------------------------------------------------------------|
| `name`           | `String` | Crate name from `CoverageRun`.                                    |
| `version`        | `String` | Crate version.                                                    |
| `line_pct`       | `f64`    | Percent of executable lines exercised (0.0-100.0).                |
| `function_pct`   | `f64`    | Percent of functions called by at least one test.                 |
| `region_pct`     | `f64`    | Percent of basic blocks / branch points exercised.                |
| `total_lines`    | `u64`    | Total executable lines counted.                                   |
| `covered_lines`  | `u64`    | Lines exercised at least once.                                    |

### `CoverageResult::into_check_result`

```rust
pub fn into_check_result(self, threshold: CoverageThreshold) -> CheckResult
```

Convert this result into a `dev-report::CheckResult` against a
threshold. Returns a passing check if the relevant percentage meets
the threshold; failing (`Severity::Warning`) otherwise.

```rust
use dev_coverage::{CoverageResult, CoverageThreshold};

# let result = CoverageResult {
#     name: "x".into(), version: "0.1.0".into(),
#     line_pct: 85.0, function_pct: 90.0, region_pct: 80.0,
#     total_lines: 100, covered_lines: 85,
# };
let check = result.into_check_result(CoverageThreshold::min_line_pct(80.0));
```

---

## `CoverageThreshold`

```rust
pub enum CoverageThreshold {
    MinLinePct(f64),
    MinFunctionPct(f64),
    MinRegionPct(f64),
}
```

### `CoverageThreshold::MinLinePct`

Fail if `result.line_pct < pct`. Most common threshold type.

### `CoverageThreshold::MinFunctionPct`

Fail if `result.function_pct < pct`. Useful for libraries where you
want every public function exercised.

### `CoverageThreshold::MinRegionPct`

Fail if `result.region_pct < pct`. Strictest type; covers branches.

### `CoverageThreshold::min_line_pct`

```rust
pub fn min_line_pct(pct: f64) -> Self
```

Build a line-coverage threshold.

```rust
use dev_coverage::CoverageThreshold;

let t = CoverageThreshold::min_line_pct(80.0);
```

### `CoverageThreshold::min_function_pct`

```rust
pub fn min_function_pct(pct: f64) -> Self
```

### `CoverageThreshold::min_region_pct`

```rust
pub fn min_region_pct(pct: f64) -> Self
```

---

## `CoverageError`

```rust
pub enum CoverageError {
    ToolNotInstalled,
    SubprocessFailed(String),
    ParseError(String),
}
```

### `CoverageError::ToolNotInstalled`

`cargo-llvm-cov` is not on the PATH.

Remediation: `cargo install cargo-llvm-cov`.

### `CoverageError::SubprocessFailed`

The `cargo llvm-cov` invocation failed. The wrapped `String` contains
stderr output for diagnosis.

### `CoverageError::ParseError`

The tool output couldn't be parsed. Indicates an incompatibility
between this crate and the installed `cargo-llvm-cov` version. The
wrapped `String` contains parser-level diagnostic.
