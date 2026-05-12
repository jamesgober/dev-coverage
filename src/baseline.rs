//! Persisted coverage baselines.
//!
//! A [`Baseline`] captures the three headline percentages
//! (`line_pct`, `function_pct`, `region_pct`) for a single subject so
//! the next run can diff against it. Baselines are stored under a
//! caller-chosen *scope* — typically a git SHA, a branch name, or the
//! literal `"latest"` — via any [`BaselineStore`] implementation.
//!
//! [`JsonFileBaselineStore`] is the default file-system backend. It
//! writes one JSON file per `(scope, name)` pair using write-temp-rename
//! semantics so partial writes never corrupt a comparison.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Persisted coverage baseline for a single subject.
///
/// # Example
///
/// ```
/// use dev_coverage::Baseline;
///
/// let b = Baseline {
///     name: "my-crate".into(),
///     line_pct: 87.5,
///     function_pct: 90.0,
///     region_pct: 80.0,
/// };
/// assert_eq!(b.name, "my-crate");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    /// Subject name. Matches the `name` on the [`CoverageResult`] this
    /// baseline was derived from.
    ///
    /// [`CoverageResult`]: crate::CoverageResult
    pub name: String,
    /// Line coverage percentage at baseline time.
    pub line_pct: f64,
    /// Function coverage percentage at baseline time.
    pub function_pct: f64,
    /// Region coverage percentage at baseline time.
    pub region_pct: f64,
}

/// Storage backend for [`Baseline`] values.
///
/// Implementations MUST treat [`load`](Self::load) as tolerant of
/// missing data (return `Ok(None)`) and SHOULD make
/// [`save`](Self::save) atomic — partial writes that survive a crash
/// corrupt future comparisons.
///
/// `scope` is a free-form key the caller uses to namespace baselines.
/// Implementations MUST treat `(scope, name)` as the identity of a
/// baseline.
pub trait BaselineStore {
    /// Load a baseline if one exists for `(scope, name)`.
    fn load(&self, scope: &str, name: &str) -> io::Result<Option<Baseline>>;

    /// Persist a baseline atomically under the given scope.
    fn save(&self, scope: &str, baseline: &Baseline) -> io::Result<()>;
}

/// Filesystem-backed JSON baseline store.
///
/// Keys baselines as `<root>/<scope>/<name>.json`. Save uses
/// write-temp-rename to remain atomic on the same filesystem.
///
/// # Example
///
/// ```
/// use dev_coverage::{Baseline, BaselineStore, JsonFileBaselineStore};
///
/// let dir = tempfile::tempdir().unwrap();
/// let store = JsonFileBaselineStore::new(dir.path());
///
/// let b = Baseline {
///     name: "my-crate".into(),
///     line_pct: 85.0,
///     function_pct: 90.0,
///     region_pct: 80.0,
/// };
/// store.save("main", &b).unwrap();
/// let back = store.load("main", "my-crate").unwrap().unwrap();
/// assert_eq!(back, b);
/// ```
#[derive(Debug, Clone)]
pub struct JsonFileBaselineStore {
    root: PathBuf,
}

impl JsonFileBaselineStore {
    /// Build a store rooted at `path`. Subdirectories per scope are
    /// created lazily on `save`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Path that would be used to persist `(scope, name)`.
    pub fn path_for(&self, scope: &str, name: &str) -> PathBuf {
        self.root.join(scope).join(format!("{name}.json"))
    }

    fn write_atomic(target: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = target.with_extension("json.tmp");
        fs::write(&tmp, contents)?;
        // On Windows, rename will fail if the target exists; replace by
        // removing first when present.
        if target.exists() {
            fs::remove_file(target)?;
        }
        fs::rename(&tmp, target)
    }
}

impl BaselineStore for JsonFileBaselineStore {
    fn load(&self, scope: &str, name: &str) -> io::Result<Option<Baseline>> {
        let path = self.path_for(scope, name);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path)?;
        let baseline: Baseline = serde_json::from_str(&text).map_err(io::Error::other)?;
        Ok(Some(baseline))
    }

    fn save(&self, scope: &str, baseline: &Baseline) -> io::Result<()> {
        let path = self.path_for(scope, &baseline.name);
        let serialized = serde_json::to_string_pretty(baseline).map_err(io::Error::other)?;
        Self::write_atomic(&path, &serialized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Baseline {
        Baseline {
            name: "my-crate".into(),
            line_pct: 87.5,
            function_pct: 90.0,
            region_pct: 80.0,
        }
    }

    #[test]
    fn load_missing_returns_ok_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonFileBaselineStore::new(dir.path());
        let got = store.load("main", "nothing-here").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonFileBaselineStore::new(dir.path());
        let b = fixture();
        store.save("main", &b).unwrap();
        let back = store.load("main", "my-crate").unwrap();
        assert_eq!(back, Some(b));
    }

    #[test]
    fn save_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonFileBaselineStore::new(dir.path());
        let mut b = fixture();
        store.save("main", &b).unwrap();
        b.line_pct = 99.0;
        store.save("main", &b).unwrap();
        let back = store.load("main", "my-crate").unwrap().unwrap();
        assert_eq!(back.line_pct, 99.0);
    }

    #[test]
    fn scopes_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonFileBaselineStore::new(dir.path());
        let mut main = fixture();
        main.line_pct = 80.0;
        let mut feature = fixture();
        feature.line_pct = 60.0;
        store.save("main", &main).unwrap();
        store.save("feature/x", &feature).unwrap();
        let m = store.load("main", "my-crate").unwrap().unwrap();
        let f = store.load("feature/x", "my-crate").unwrap().unwrap();
        assert_eq!(m.line_pct, 80.0);
        assert_eq!(f.line_pct, 60.0);
    }

    #[test]
    fn path_for_matches_layout() {
        let store = JsonFileBaselineStore::new("/tmp/x");
        let p = store.path_for("main", "my-crate");
        assert!(p.ends_with("main/my-crate.json"));
    }

    #[test]
    fn load_rejects_corrupt_json() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonFileBaselineStore::new(dir.path());
        // Write garbage at the expected path.
        let p = store.path_for("main", "my-crate");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "not json").unwrap();
        let err = store.load("main", "my-crate").err().unwrap();
        // Just confirm it surfaces an error rather than panicking.
        assert!(err.to_string().to_lowercase().contains("expected"));
    }
}
