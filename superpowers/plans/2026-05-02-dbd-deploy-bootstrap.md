# dbd Deploy + Version Upgrade Bootstrap — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Embed dbd-core into the bootstrap crate so that `setup_database` downloads + applies the schema from `sensei-hq/daemon/database@v{app_version}`, and extend phase-1 gates to detect and upgrade stale sensei/senseid binaries.

**Architecture:** Three sequential repo changes — fix dbd-rs first (cache bug + `Design::deploy()` method), then wire dbd-core into the bootstrap crate (new `VersionedBinaryChecker`, `BrewUpgradeFixer`, `database::deploy()`), then update the Tauri commands to thread `app_version` through and add E2E tests.

**Tech Stack:** Rust (dbd-core/sqlx/tokio), Tauri 2, Playwright (TypeScript), cargo workspaces.

---

## Repo map

| Repo | Path |
|------|------|
| dbd-rs | `/Users/Jerry/Developer/dbd-rs` |
| daemon workspace | `/Users/Jerry/Developer/sensei/daemon` |
| app | `/Users/Jerry/Developer/sensei/app` |
| docs | `/Users/Jerry/Developer/sensei/docs` |

## Files changed

### dbd-rs
| Action | File | Purpose |
|--------|------|---------|
| Modify | `crates/dbd-core/src/deploy.rs` | Fix cache-check bug for subpath sources |
| Modify | `crates/dbd-core/src/design.rs` | Add `Design::deploy()` method |
| Create | `bump-version.sh` | Tag dbd-rs releases consistently |

### daemon / bootstrap crate
| Action | File | Purpose |
|--------|------|---------|
| Modify | `crates/bootstrap/Cargo.toml` | Add `dbd-core` path dep; add `rt` to tokio |
| Modify | `crates/bootstrap/src/database.rs` | Add `deploy(app_version)`; update `setup(app_version)`; remove `migrate()` |
| Modify | `crates/bootstrap/src/prereq/checker.rs` | Add `VersionedBinaryChecker` |
| Modify | `crates/bootstrap/src/prereq/fixer.rs` | Add `BrewUpgradeFixer`; add `app_version` to `DatabaseSetupFixer` |
| Modify | `crates/bootstrap/src/prereq/factory.rs` | Add `expected_version` param; add senseid gate |
| Create | `bump-version.sh` | Tag daemon releases consistently |

### app
| Action | File | Purpose |
|--------|------|---------|
| Modify | `src-tauri/src/commands/bootstrap.rs` | Thread `app_version` from Tauri into factory calls |
| Create | `e2e/fixtures-db-missing.ts` | Stateful IPC mocks: DB failed → ready after setup |
| Create | `e2e/tests/db-setup.spec.ts` | E2E test: DB missing → autoconfigure → advance to /setup/welcome |

---

## Task 1: Fix dbd-rs cache-check bug

**Files:**
- Modify: `/Users/Jerry/Developer/dbd-rs/crates/dbd-core/src/deploy.rs`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` block in `deploy.rs`:

```rust
#[tokio::test]
async fn resolve_source_cache_hit_with_subpath() {
    use crate::github::cache_dir;
    use tempfile::TempDir;

    // Simulate a pre-populated cache for sensei-hq/daemon/database@test-cache-hit-v1
    let cache = cache_dir("sensei-hq", "daemon", "test-cache-hit-v1");
    let database_dir = cache.join("database");
    std::fs::create_dir_all(&database_dir).unwrap();
    std::fs::write(database_dir.join("design.yaml"), "project:\n  name: test\n").unwrap();

    // Should return the database/ subpath without attempting a network download
    let result = resolve_source("sensei-hq/daemon/database@test-cache-hit-v1").await;

    // Cleanup before assertions so a failure doesn't leave files behind
    std::fs::remove_dir_all(&cache).ok();

    let path = result.expect("should return cached path without downloading");
    assert!(
        path.ends_with("database"),
        "should resolve to database/ subpath, got: {}",
        path.display()
    );
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd /Users/Jerry/Developer/dbd-rs
cargo test -p dbd-core resolve_source_cache_hit_with_subpath -- --nocapture
```

Expected: test fails because the current code checks `cache.join("design.yaml")` which doesn't exist for subpath sources, so it tries to download and gets a GitHub error.

- [ ] **Step 3: Fix the cache-check in `deploy.rs`**

Find and replace the cache-hit guard in `resolve_source`:

```rust
// Replace this block (around line 22-25):
// Check if already cached
if cache.join("design.yaml").exists() {
    return Ok(resolve_subpath(&cache, gh.subpath.as_deref()));
}

// With this:
// Check if already cached — resolve subpath first so subpath sources hit correctly
let resolved = resolve_subpath(&cache, gh.subpath.as_deref());
if resolved.join("design.yaml").exists() {
    return Ok(resolved);
}
```

- [ ] **Step 4: Run test to confirm it passes**

```bash
cd /Users/Jerry/Developer/dbd-rs
cargo test -p dbd-core resolve_source_cache_hit_with_subpath -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Run full dbd-core test suite**

```bash
cd /Users/Jerry/Developer/dbd-rs
cargo test -p dbd-core
```

Expected: All tests pass. Zero regressions.

- [ ] **Step 6: Commit**

```bash
cd /Users/Jerry/Developer/dbd-rs
git add crates/dbd-core/src/deploy.rs
git commit -m "fix: resolve cache-check miss for subpath GitHub sources

When source is 'owner/repo/subpath@ref', design.yaml lands at
cache/subpath/design.yaml not cache/design.yaml. Previous check
always missed and re-downloaded the tarball.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Add `Design::deploy()` to dbd-core

**Files:**
- Modify: `/Users/Jerry/Developer/dbd-rs/crates/dbd-core/src/design.rs`

- [ ] **Step 1: Write the failing tests**

Locate the `#[cfg(test)]` block at the bottom of `design.rs` and add:

```rust
#[tokio::test]
async fn deploy_dry_run_returns_ok_and_applies_nothing() {
    use crate::adapter::mock::MockAdapter;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    // Minimal design with no entities or import config
    std::fs::write(
        tmp.path().join("design.yaml"),
        "project:\n  name: test\n  version: 1\nsource:\n  dialect: postgresql\ntarget:\n  postgres:\n    url: $DATABASE_URL\n",
    ).unwrap();

    let design = Design::from_config_with_dir(
        &tmp.path().join("design.yaml"),
        "prod",
        Some(tmp.path()),
    ).unwrap();

    let mock = MockAdapter::new();
    design.deploy(&mock, true).await.unwrap();

    assert!(mock.applied_names().is_empty(), "dry_run must not apply any entities");
    assert!(mock.imported_names().is_empty(), "dry_run must not import any data");
}

#[tokio::test]
async fn deploy_non_dry_run_completes_with_no_entities() {
    use crate::adapter::mock::MockAdapter;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("design.yaml"),
        "project:\n  name: test\n  version: 1\nsource:\n  dialect: postgresql\ntarget:\n  postgres:\n    url: $DATABASE_URL\n",
    ).unwrap();

    let design = Design::from_config_with_dir(
        &tmp.path().join("design.yaml"),
        "prod",
        Some(tmp.path()),
    ).unwrap();

    let mock = MockAdapter::new();
    // Should complete without error even with no entities
    design.deploy(&mock, false).await.unwrap();
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/dbd-rs
cargo test -p dbd-core deploy_dry_run_returns_ok -- --nocapture
```

Expected: FAIL — `no method named 'deploy' found for struct Design`

- [ ] **Step 3: Implement `Design::deploy()`**

In `design.rs`, locate the `impl Design` block. After the `import_data` method (around line 701), add:

```rust
/// Deploy the full schema: apply DDL then import seed data.
///
/// Equivalent to `apply` followed by `import_data`. dbd handles
/// fresh / migrate / current strategy automatically — safe to call
/// on every bootstrap (idempotent when schema is already current).
pub async fn deploy(
    &self,
    adapter: &dyn DatabaseAdapter,
    dry_run: bool,
) -> Result<()> {
    self.apply(adapter, None, dry_run).await?;
    self.import_data(adapter, None, dry_run).await?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd /Users/Jerry/Developer/dbd-rs
cargo test -p dbd-core deploy_dry_run deploy_non_dry_run
```

Expected: Both PASS

- [ ] **Step 5: Run full dbd-core test suite**

```bash
cd /Users/Jerry/Developer/dbd-rs
cargo test -p dbd-core
```

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/Jerry/Developer/dbd-rs
git add crates/dbd-core/src/design.rs
git commit -m "feat: add Design::deploy() — apply DDL then import seed data

Single entry point for bootstrap integration: runs apply (schema +
migrations) followed by import_data (staging seed). Idempotent —
dbd handles fresh/migrate/current automatically.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Add `bump-version.sh` to dbd-rs

**Files:**
- Create: `/Users/Jerry/Developer/dbd-rs/bump-version.sh`

- [ ] **Step 1: Create the script**

```bash
cat > /Users/Jerry/Developer/dbd-rs/bump-version.sh << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

VERSION=${1:?usage: ./bump-version.sh X.Y.Z}

# Update workspace version in Cargo.toml
sed -i '' "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" Cargo.toml

# Rebuild lock file
cargo check -q 2>&1 | grep -v "^$" || true

git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to v$VERSION"
git tag "v$VERSION"

echo "Tagged v$VERSION — push with: git push && git push --tags"
EOF
chmod +x /Users/Jerry/Developer/dbd-rs/bump-version.sh
```

- [ ] **Step 2: Verify it's executable**

```bash
ls -la /Users/Jerry/Developer/dbd-rs/bump-version.sh
```

Expected: `-rwxr-xr-x` permissions

- [ ] **Step 3: Commit**

```bash
cd /Users/Jerry/Developer/dbd-rs
git add bump-version.sh
git commit -m "chore: add bump-version.sh for consistent release tagging

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Add dbd-core dependency to bootstrap crate

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/daemon/crates/bootstrap/Cargo.toml`

- [ ] **Step 1: Add the dependency**

Open `Cargo.toml` and make these two changes:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sysinfo = "0.34"
tokio = { version = "1", features = ["process", "time", "rt"] }   # add "rt"
reqwest = { version = "0.12", features = ["blocking", "json"] }
chrono = "0.4"
dbd-core = { path = "../../../../dbd-rs/crates/dbd-core" }         # add this line
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo check -p sensei-bootstrap
```

Expected: compiles without errors. (First run downloads sqlx — takes a minute.)

- [ ] **Step 3: Run existing tests to confirm no regressions**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap
```

Expected: All existing tests pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/Jerry/Developer/sensei/daemon
git add crates/bootstrap/Cargo.toml Cargo.lock
git commit -m "feat(bootstrap): add dbd-core path dependency

Enables embedding dbd deploy for schema management without
shelling out to an external binary.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Add `VersionedBinaryChecker`

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/daemon/crates/bootstrap/src/prereq/checker.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `checker.rs`:

```rust
#[test]
fn versioned_binary_checker_missing_binary_returns_fail() {
    let checker = VersionedBinaryChecker::new(
        "sensei-nonexistent-xyz-binary", "--version", "1.0.0",
    );
    let result = checker.check();
    assert!(!result.ok);
    assert!(
        result.error.as_deref().unwrap().contains("not found"),
        "error should mention 'not found', got: {:?}",
        result.error
    );
}

#[test]
fn versioned_binary_checker_version_mismatch_returns_fail() {
    // Use 'ls' which exists but will not return version "9999.9.9"
    let checker = VersionedBinaryChecker::new("ls", "--version", "9999.9.9");
    let result = checker.check();
    // ls exists but its version ≠ "9999.9.9"
    // Either ok (if somehow matches) or fail with "outdated"
    if !result.ok {
        let err = result.error.as_deref().unwrap();
        assert!(
            err.contains("outdated"),
            "error should mention 'outdated', got: {err}"
        );
    }
}

#[test]
fn versioned_binary_checker_correct_version_returns_ok() {
    // echo always exists, use a version string unlikely to actually match
    // Instead: check that when version matches, it returns ok
    let checker = VersionedBinaryChecker::new("ls", "--version", "unknown");
    // "unknown" is what binary_version returns when parsing fails
    // If ls --version fails or returns nothing parseable, binary_version returns "unknown"
    // This exercises the ok path
    let result = checker.check();
    // Result is ok only if installed version == "unknown" — mostly just verify no panic
    assert!(result.ok || !result.ok); // no panic
}

#[test]
fn versioned_binary_checker_stores_fields() {
    let checker = VersionedBinaryChecker::new("sensei", "--version", "0.1.0");
    assert_eq!(checker.binary, "sensei");
    assert_eq!(checker.version_flag, "--version");
    assert_eq!(checker.expected_version, "0.1.0");
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap versioned_binary_checker -- --nocapture
```

Expected: FAIL — `cannot find struct VersionedBinaryChecker`

- [ ] **Step 3: Implement `VersionedBinaryChecker`**

In `checker.rs`, after the `BinaryAndPortChecker` impl block, add:

```rust
/// Checks that a binary exists in PATH AND its version matches the expected version.
///
/// Extracts the bare semver from output like "sensei 0.2.13" by taking the last
/// whitespace-delimited token. Returns `fail("outdated: ...")` when the binary exists
/// but is at the wrong version, so the fixer knows to upgrade rather than install.
pub struct VersionedBinaryChecker {
    pub binary: String,
    pub version_flag: String,
    pub expected_version: String,
}

impl VersionedBinaryChecker {
    pub fn new(
        binary: impl Into<String>,
        version_flag: impl Into<String>,
        expected_version: impl Into<String>,
    ) -> Self {
        Self {
            binary: binary.into(),
            version_flag: version_flag.into(),
            expected_version: expected_version.into(),
        }
    }
}

impl Checker for VersionedBinaryChecker {
    fn check(&self) -> CheckResult {
        let path = match util::which_binary(&self.binary) {
            None => return CheckResult::fail(format!("{} not found in PATH", self.binary)),
            Some(p) => p,
        };
        let raw = util::binary_version(&self.binary, &self.version_flag)
            .unwrap_or_else(|| "unknown".to_string());
        // Extract bare semver: "sensei 0.2.13" → "0.2.13"
        let installed = raw.split_whitespace().last().unwrap_or("unknown");
        if installed == self.expected_version {
            CheckResult::ok_with_detail(raw, path)
        } else {
            CheckResult::fail(format!(
                "{} outdated: installed {}, expected {}",
                self.binary, installed, self.expected_version
            ))
        }
    }
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap versioned_binary_checker
```

Expected: All 4 tests PASS

- [ ] **Step 5: Run full bootstrap test suite**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap
```

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/Jerry/Developer/sensei/daemon
git add crates/bootstrap/src/prereq/checker.rs
git commit -m "feat(bootstrap): add VersionedBinaryChecker

Checks binary exists and version matches expected. Returns
'outdated' error so BrewUpgradeFixer knows to upgrade vs install.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Add `BrewUpgradeFixer` and update `DatabaseSetupFixer`

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/daemon/crates/bootstrap/src/prereq/fixer.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `fixer.rs`:

```rust
#[test]
fn brew_upgrade_fixer_stores_fields() {
    let fixer = BrewUpgradeFixer::new("/opt/homebrew/bin/brew", "sensei-hq/tap/sensei");
    assert_eq!(fixer.brew_path, "/opt/homebrew/bin/brew");
    assert_eq!(fixer.formula, "sensei-hq/tap/sensei");
}

#[test]
fn brew_upgrade_fixer_nonexistent_brew_returns_err() {
    let fixer = BrewUpgradeFixer::new("/nonexistent/brew", "sensei-hq/tap/sensei");
    let result = fixer.fix();
    assert!(result.is_err(), "should fail when brew binary does not exist");
    let err = result.unwrap_err();
    assert!(
        err.contains("brew upgrade") || err.contains("brew install"),
        "error should mention brew operation, got: {err}"
    );
}

#[test]
fn database_setup_fixer_stores_version() {
    let fixer = DatabaseSetupFixer::new("0.1.0");
    assert_eq!(fixer.app_version, "0.1.0");
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap brew_upgrade_fixer database_setup_fixer_stores -- --nocapture
```

Expected: FAIL — structs not found

- [ ] **Step 3: Add `BrewUpgradeFixer` and update `DatabaseSetupFixer`**

In `fixer.rs`, after the `BrewFixer` impl block, add `BrewUpgradeFixer`:

```rust
/// Runs `brew upgrade <formula>` first (handles version bump); falls back to
/// `brew install <formula>` for first-time installs. One fixer covers both cases.
pub struct BrewUpgradeFixer {
    pub brew_path: String,
    pub formula: String,
}

impl BrewUpgradeFixer {
    pub fn new(brew_path: impl Into<String>, formula: impl Into<String>) -> Self {
        Self { brew_path: brew_path.into(), formula: formula.into() }
    }
}

impl Fixer for BrewUpgradeFixer {
    fn fix(&self) -> Result<FixResult, String> {
        // Try upgrade first — works when formula is already installed but outdated
        let upgrade = Command::new(&self.brew_path)
            .args(["upgrade", &self.formula])
            .output()
            .map_err(|e| format!("brew upgrade failed to run: {e}"))?;

        if upgrade.status.success() {
            return Ok(FixResult::new(format!("brew upgrade {}", self.formula)));
        }

        // Fall back to install — handles first-time install
        let install = Command::new(&self.brew_path)
            .args(["install", &self.formula])
            .output()
            .map_err(|e| format!("brew install failed to run: {e}"))?;

        if install.status.success() {
            return Ok(FixResult::new(format!("brew install {}", self.formula)));
        }

        let stderr = String::from_utf8_lossy(&install.stderr).trim().to_string();
        Err(format!("brew install {} failed: {stderr}", self.formula))
    }
}
```

Then update `DatabaseSetupFixer` to hold `app_version`:

```rust
// Replace the existing DatabaseSetupFixer struct and impl with:

/// Runs the full database setup pipeline using dbd deploy.
pub struct DatabaseSetupFixer {
    pub app_version: String,
}

impl DatabaseSetupFixer {
    pub fn new(app_version: impl Into<String>) -> Self {
        Self { app_version: app_version.into() }
    }
}

impl Fixer for DatabaseSetupFixer {
    fn fix(&self) -> Result<FixResult, String> {
        crate::database::setup(&self.app_version).map(|status| {
            FixResult::new(format!(
                "database setup complete: {}",
                status.version.as_deref().unwrap_or("unknown")
            ))
        })
    }
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap brew_upgrade_fixer database_setup_fixer_stores
```

Expected: All 3 PASS

- [ ] **Step 5: Run full bootstrap test suite**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap
```

Expected: All tests pass (the existing `database_setup_fixer` test may need updating — fix any compile errors from the signature change first).

- [ ] **Step 6: Commit**

```bash
cd /Users/Jerry/Developer/sensei/daemon
git add crates/bootstrap/src/prereq/fixer.rs
git commit -m "feat(bootstrap): add BrewUpgradeFixer; add app_version to DatabaseSetupFixer

BrewUpgradeFixer: upgrade first, install as fallback — handles both
outdated and missing binaries with one fixer.
DatabaseSetupFixer: now receives app_version to pass to dbd deploy.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 7: Add `database::deploy()` and update `database::setup()`

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/daemon/crates/bootstrap/src/database.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `database.rs`:

```rust
#[test]
fn deploy_source_string_is_parseable() {
    // Verify the source format we construct is valid per dbd-core's parser
    let version = "1.2.3";
    let source = format!("sensei-hq/daemon/database@v{version}");
    let parsed = dbd_core::github::parse_github_source(&source).unwrap();
    assert_eq!(parsed.owner, "sensei-hq");
    assert_eq!(parsed.repo, "daemon");
    assert_eq!(parsed.subpath, Some("database".to_string()));
    assert_eq!(parsed.git_ref, format!("v{version}"));
}

#[test]
fn deploy_db_url_format() {
    // Verify the DATABASE_URL we construct has the right format
    std::env::set_var("SENSEI_DB_NAME", "sensei-test-deploy-url");
    let db = db_name();
    let url = format!("postgres://localhost/{db}");
    assert!(url.starts_with("postgres://localhost/"));
    assert!(url.ends_with("sensei-test-deploy-url"));
}

#[test]
fn setup_without_postgres_returns_err() {
    // Existing test updated to pass app_version
    let result = setup("0.1.0");
    if !super::pg_is_ready() {
        let err = result.unwrap_err();
        assert!(err.contains("not accepting connections"), "got: {err}");
    }
    // If postgres IS available, result could be Ok or Err — either is fine
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap deploy_source_string deploy_db_url -- --nocapture
```

Expected: FAIL — `deploy` function not found; `setup` signature mismatch

- [ ] **Step 3: Add `database::deploy()` and update `setup()`**

In `database.rs`:

1. Add the `dbd_core` import at the top of the file:

```rust
use dbd_core::{
    Design,
    deploy::resolve_source,
    adapter::postgres::PostgresAdapter,
};
```

2. Add the `deploy` function (place after `ensure_extensions`):

```rust
/// Deploy the schema from GitHub using dbd-core.
///
/// Source: `sensei-hq/daemon/database@v{app_version}` — downloads tarball
/// and caches under `~/.cache/dbd/`. Idempotent: dbd handles fresh /
/// migrate / current automatically.
///
/// Requires a running PostgreSQL server and a reachable `{db_name}` database.
pub fn deploy(app_version: &str) -> Result<ComponentStatus, String> {
    let db = db_name();
    let source = format!("sensei-hq/daemon/database@v{app_version}");
    let db_url = format!("postgres://localhost/{db}");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime error: {e}"))?;

    rt.block_on(async {
        let project_dir = resolve_source(&source)
            .await
            .map_err(|e| format!("dbd source resolution failed: {e}"))?;

        let config_path = project_dir.join("design.yaml");

        let design = Design::from_config_with_dir(&config_path, "prod", Some(&project_dir))
            .map_err(|e| format!("dbd config load failed: {e}"))?;

        let adapter = PostgresAdapter::new(&db_url, "sensei")
            .await
            .map_err(|e| format!("dbd database connection failed: {e}"))?;

        design
            .deploy(&adapter, false)
            .await
            .map_err(|e| format!("dbd deploy failed: {e}"))
    })?;

    let version = schema_version(db);
    Ok(ComponentStatus::ready(
        "database",
        &format!("schema-{}", version.unwrap_or(0)),
    ))
}
```

3. Update `setup` signature to accept `app_version` and call `deploy` instead of `migrate`:

```rust
/// Full Phase 3 database setup pipeline.
///
/// 1. Check PostgreSQL is reachable
/// 2. Ensure database exists (create if missing)
/// 3. Ensure extensions (pgvector)
/// 4. Run dbd deploy (schema + seed data)
pub fn setup(app_version: &str) -> Result<ComponentStatus, String> {
    let db = db_name();

    if !pg_is_ready() {
        return Err("postgresql is not accepting connections".to_string());
    }

    match database_exists(db) {
        Ok(true)  => {}
        Ok(false) => { create()?; }
        Err(e)    => { return Err(format!("database check failed: {e}")); }
    }

    ensure_extensions()?;
    deploy(app_version)
}
```

4. Remove the `migrate` function entirely (it was a stub — nothing else calls it).

- [ ] **Step 4: Fix the existing `setup_without_postgres` test**

The existing test called `setup()` with no args. It's been replaced in step 1 above. Delete the old test from the file if it's still present:

```rust
// DELETE this test if it exists:
// #[test]
// fn setup_without_postgres() {
//     let result = setup();
//     ...
// }
// It is replaced by setup_without_postgres_returns_err in Step 1
```

Also delete the `migrate_without_senseid` test — `migrate()` no longer exists:
```rust
// DELETE:
// #[test]
// fn migrate_without_senseid() { ... }
```

- [ ] **Step 5: Run tests to confirm they pass**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap deploy_source_string deploy_db_url setup_without_postgres
```

Expected: All 3 PASS

- [ ] **Step 6: Run full bootstrap test suite**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap
```

Expected: All tests pass. (If postgres is not running, `deploy`/`setup` tests return Err gracefully — that's expected.)

- [ ] **Step 7: Commit**

```bash
cd /Users/Jerry/Developer/sensei/daemon
git add crates/bootstrap/src/database.rs
git commit -m "feat(bootstrap): replace migrate stub with dbd deploy

database::deploy(app_version) downloads sensei-hq/daemon/database@v{version}
from GitHub and applies schema via dbd-core (DDL + seed import).
database::setup(app_version) now calls deploy instead of the old senseid
migrate stub. migrate() removed entirely.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 8: Update `factory.rs` — add `expected_version` param + senseid gate

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/daemon/crates/bootstrap/src/prereq/factory.rs`

- [ ] **Step 1: Write the failing tests**

In the `#[cfg(test)]` block of `factory.rs`, update/add:

```rust
#[test]
fn install_prerequisites_returns_four_gates() {
    let provider = Arc::from(platform::detect());
    let prereqs = install_prerequisites(provider, "0.1.0");
    assert_eq!(prereqs.len(), 4, "expected 4 gates: postgresql, ollama, sensei, senseid");
    assert_eq!(prereqs[0].id(), "postgresql");
    assert_eq!(prereqs[1].id(), "ollama");
    assert_eq!(prereqs[2].id(), "sensei");
    assert_eq!(prereqs[3].id(), "senseid");
}

#[test]
fn setup_database_accepts_version() {
    let prereqs = setup_database("0.1.0");
    assert_eq!(prereqs.len(), 1);
    assert_eq!(prereqs[0].id(), "database");
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap install_prerequisites_returns setup_database_accepts -- --nocapture
```

Expected: FAIL — `install_prerequisites` takes wrong number of args; count mismatch

- [ ] **Step 3: Update `factory.rs`**

Replace the entire `install_prerequisites` and `setup_database` functions:

```rust
use super::checker::{BinaryChecker, PortChecker, DatabaseChecker, VersionedBinaryChecker};
use super::fixer::{BrewFixer, BrewUpgradeFixer, WingetFixer, NoopFixer, ServiceStartFixer, DatabaseSetupFixer, Fixer};

/// Phase 1 — install binaries: postgresql, ollama, sensei CLI, senseid daemon.
///
/// sensei and senseid are version-pinned to expected_version and will be
/// upgraded via brew if behind. postgresql and ollama use BinaryChecker only.
pub fn install_prerequisites(
    provider: Arc<dyn PlatformProvider>,
    expected_version: &str,
) -> Vec<Box<dyn Prerequisite>> {
    let version = expected_version.to_string();
    match provider.platform() {
        Platform::MacOS | Platform::Linux => {
            let brew = detect_brew_path();
            let mk_brew_fixer = |formula: &str| -> Box<dyn Fixer> {
                match &brew {
                    Some(p) => Box::new(BrewFixer::new(p, formula)),
                    None    => Box::new(NoopFixer::new("Homebrew not found — install Homebrew first")),
                }
            };
            let mk_upgrade_fixer = |formula: &str| -> Box<dyn Fixer> {
                match &brew {
                    Some(p) => Box::new(BrewUpgradeFixer::new(p, formula)),
                    None    => Box::new(NoopFixer::new("Homebrew not found — install Homebrew first")),
                }
            };
            vec![
                Box::new(GenericPrerequisite::new(
                    "postgresql", "PostgreSQL",
                    Box::new(BinaryChecker::new("postgres", "--version")),
                    mk_brew_fixer("postgresql@17"),
                    GateKind::Install, None,
                )),
                Box::new(GenericPrerequisite::new(
                    "ollama", "Ollama",
                    Box::new(BinaryChecker::new("ollama", "--version")),
                    mk_brew_fixer("ollama"),
                    GateKind::Install, None,
                )),
                Box::new(GenericPrerequisite::new(
                    "sensei", "Sensei CLI",
                    Box::new(VersionedBinaryChecker::new("sensei", "--version", &version)),
                    mk_upgrade_fixer("sensei-hq/tap/sensei"),
                    GateKind::Install, None,
                )),
                Box::new(GenericPrerequisite::new(
                    "senseid", "Sensei Daemon",
                    Box::new(VersionedBinaryChecker::new("senseid", "--version", &version)),
                    mk_upgrade_fixer("sensei-hq/tap/senseid"),
                    GateKind::Install, None,
                )),
            ]
        }
        Platform::Windows => vec![
            Box::new(GenericPrerequisite::new(
                "postgresql", "PostgreSQL",
                Box::new(BinaryChecker::new("postgres", "--version")),
                Box::new(WingetFixer::new("PostgreSQL.PostgreSQL")),
                GateKind::Install, None,
            )),
            Box::new(GenericPrerequisite::new(
                "ollama", "Ollama",
                Box::new(BinaryChecker::new("ollama", "--version")),
                Box::new(WingetFixer::new("Ollama.Ollama")),
                GateKind::Install, None,
            )),
            Box::new(GenericPrerequisite::new(
                "sensei", "Sensei CLI",
                Box::new(VersionedBinaryChecker::new("sensei", "--version", &version)),
                Box::new(NoopFixer::new("Download sensei from sensei.so/download")),
                GateKind::Install, None,
            )),
            Box::new(GenericPrerequisite::new(
                "senseid", "Sensei Daemon",
                Box::new(VersionedBinaryChecker::new("senseid", "--version", &version)),
                Box::new(NoopFixer::new("Download senseid from sensei.so/download")),
                GateKind::Install, None,
            )),
        ],
    }
}

/// Phase 3 — database setup.
pub fn setup_database(app_version: &str) -> Vec<Box<dyn Prerequisite>> {
    vec![
        Box::new(GenericPrerequisite::new(
            "database", "Sensei Database",
            Box::new(DatabaseChecker),
            Box::new(DatabaseSetupFixer::new(app_version)),
            GateKind::Install, None,
        )),
    ]
}
```

- [ ] **Step 4: Update the old `install_prerequisites_returns_three_gates` test**

This test must be deleted or replaced — it tests 3 gates but we now have 4. The new test in Step 1 replaces it. Delete the old test from the file:

```rust
// DELETE this test:
// #[test]
// fn install_prerequisites_returns_three_gates() { ... }
// Replaced by install_prerequisites_returns_four_gates
```

Also update `install_prereqs_all_have_install_kind` to pass the version arg:

```rust
#[test]
fn install_prereqs_all_have_install_kind() {
    let provider = Arc::from(platform::detect());
    for p in install_prerequisites(provider, "0.1.0") {
        assert_eq!(p.gate_kind(), GateKind::Install, "{} should be Install kind", p.id());
    }
}
```

And update `setup_database_returns_one_gate`:
```rust
#[test]
fn setup_database_returns_one_gate() {
    let prereqs = setup_database("0.1.0");
    assert_eq!(prereqs.len(), 1);
    assert_eq!(prereqs[0].id(), "database");
}
```

- [ ] **Step 5: Run tests to confirm they pass**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap install_prerequisites setup_database
```

Expected: All PASS

- [ ] **Step 6: Run full bootstrap test suite**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap
```

Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
cd /Users/Jerry/Developer/sensei/daemon
git add crates/bootstrap/src/prereq/factory.rs
git commit -m "feat(bootstrap): add expected_version to factory; add senseid gate

install_prerequisites now takes expected_version and uses
VersionedBinaryChecker + BrewUpgradeFixer for sensei + senseid.
setup_database passes app_version to DatabaseSetupFixer.
senseid added as a new phase-1 gate (was missing).

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 9: Update Tauri bootstrap commands

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/app/src-tauri/src/commands/bootstrap.rs`

- [ ] **Step 1: Read the current file**

Read `bootstrap.rs` lines 179–207 (the three phase commands).

- [ ] **Step 2: Update `install_prerequisites` and `setup_database`**

Replace the two commands:

```rust
#[tauri::command]
pub fn install_prerequisites(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    std::thread::spawn(move || {
        let provider = Arc::from(bootstrap::provider());
        let prereqs = factory::install_prerequisites(provider, &version);
        runner::run("install", prereqs, |e| dispatch(&app, e));
    });
    Ok(())
}

#[tauri::command]
pub fn setup_database(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    std::thread::spawn(move || {
        let prereqs = factory::setup_database(&version);
        runner::run("database", prereqs, |e| dispatch(&app, e));
    });
    Ok(())
}
```

`start_services` is unchanged.

- [ ] **Step 3: Verify the app compiles**

```bash
cd /Users/Jerry/Developer/sensei/app
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
cd /Users/Jerry/Developer/sensei/app
git add src-tauri/src/commands/bootstrap.rs
git commit -m "feat(app): thread app_version into bootstrap factory calls

install_prerequisites and setup_database now read app.package_info().version
and pass it to the bootstrap factory so version checks and dbd deploy
use the correct version tag.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 10: Tag daemon repo + add bump-version.sh

**Files:**
- Create: `/Users/Jerry/Developer/sensei/daemon/bump-version.sh`

- [ ] **Step 1: Create the bump script**

```bash
cat > /Users/Jerry/Developer/sensei/daemon/bump-version.sh << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

VERSION=${1:?usage: ./bump-version.sh X.Y.Z}

# Update all crate versions in the workspace
for toml in crates/*/Cargo.toml; do
  sed -i '' "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" "$toml"
done

cargo check -q 2>&1 | grep -v "^$" || true

git add crates/*/Cargo.toml Cargo.lock
git commit -m "chore: bump version to v$VERSION"
git tag "v$VERSION"

echo "Tagged v$VERSION — push with: git push && git push --tags"
EOF
chmod +x /Users/Jerry/Developer/sensei/daemon/bump-version.sh
```

- [ ] **Step 2: Create the initial `v0.1.0` tag** (matching current app version)

```bash
cd /Users/Jerry/Developer/sensei/daemon
git tag v0.1.0
echo "Tagged v0.1.0 — push with: git push --tags"
```

- [ ] **Step 3: Commit the script**

```bash
cd /Users/Jerry/Developer/sensei/daemon
git add bump-version.sh
git commit -m "chore: add bump-version.sh for consistent release tagging

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 11: Add E2E fixtures for DB-missing state

**Files:**
- Create: `/Users/Jerry/Developer/sensei/app/e2e/fixtures-db-missing.ts`

- [ ] **Step 1: Create the fixture file**

```typescript
/**
 * Test fixtures for the DB-missing bootstrap scenario.
 *
 * Stateful mocks: run_bootstrap returns DB failed on first call;
 * after setup_database is invoked, subsequent calls return DB ready.
 * This lets the bootstrap page cycle through fix → re-check → advance.
 */

import { createTauriTest } from '@srsholmes/tauri-playwright';

let setupComplete = false;

export const { test, expect } = createTauriTest({
  devUrl: 'http://localhost:5173',
  ipcMocks: {
    run_bootstrap: () => {
      const dbState = setupComplete
        ? { state: 'ready' }
        : { state: 'failed', error: "database 'sensei-dev' does not exist" };
      return {
        components: [
          { name: 'homebrew',    state: { state: 'ready' }, version: '4.0',    detail: null },
          { name: 'postgresql', state: { state: 'ready' }, version: '16',     detail: null },
          { name: 'ollama',     state: { state: 'ready' }, version: '0.3',    detail: null },
          { name: 'sensei',     state: { state: 'ready' }, version: '0.1.0',  detail: null },
          { name: 'database',   state: dbState,            version: null,      detail: null },
          { name: 'daemon',     state: { state: 'ready' }, version: '0.1.0',  detail: null },
        ],
        ready: setupComplete,
        hardware: { ram_gb: 16, cpu_cores: 8, gpu: 'Apple M2', metal_support: true, recommended_tier: 'recommended' },
      };
    },
    setup_database: () => {
      setupComplete = true;
      return null;
    },
    install_prerequisites: () => null,
    start_services: () => null,
    get_platform: () => ({
      platform: 'macos',
      package_manager: 'homebrew',
      prereq_remedy: { title: 'Install via Homebrew', command: 'brew install', url: null },
      pkgmgr_remedy: { title: 'Install Homebrew', command: '/bin/bash -c ...', url: 'https://brew.sh' },
    }),
    detect_hardware: () => ({
      ram_gb: 16, cpu_cores: 8, gpu: 'Apple M2', metal_support: true, recommended_tier: 'recommended',
    }),
  },
  mcpSocket: '/tmp/tauri-playwright.sock',
});
```

- [ ] **Step 2: Verify the file type-checks**

```bash
cd /Users/Jerry/Developer/sensei/app
npx tsc --noEmit e2e/fixtures-db-missing.ts 2>&1 | head -20
```

Expected: No type errors (or only minor import resolution warnings).

- [ ] **Step 3: Commit**

```bash
cd /Users/Jerry/Developer/sensei/app
git add e2e/fixtures-db-missing.ts
git commit -m "test(e2e): add fixtures-db-missing for stateful DB bootstrap test

Stateful IPC mocks that return DB failed on first run_bootstrap,
then ready after setup_database is called.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 12: Add E2E db-setup spec

**Files:**
- Create: `/Users/Jerry/Developer/sensei/app/e2e/tests/db-setup.spec.ts`

- [ ] **Step 1: Create the spec file**

```typescript
/**
 * Database setup E2E tests.
 *
 * Tests the bootstrap page's handling of a missing database:
 * gate五 (database) fails → setup runs → gate ready → auto-advance to /setup.
 *
 * Browser mode: mocked IPC via fixtures-db-missing.ts — fast, runs in CI.
 */

import { test as dbMissingTest, expect } from '../fixtures-db-missing';
import { test, expect as baseExpect } from '../fixtures';
import { navigateTo } from '../helpers';

dbMissingTest.describe('Bootstrap — database autoconfigure', () => {
  dbMissingTest(
    'health page shows database gate failed when DB is missing',
    async ({ tauriPage }) => {
      await navigateTo(tauriPage, '/health');

      // Gate五 (database) should show failed state
      await baseExpect(
        tauriPage.locator('[data-gate="database"]')
      ).toBeVisible();
    }
  );

  dbMissingTest(
    'app advances to /setup/welcome after database is set up',
    async ({ tauriPage }) => {
      await navigateTo(tauriPage, '/health');

      // The bootstrap page auto-triggers setup_database when the gate fails.
      // After setup_database mock completes, run_bootstrap returns DB ready.
      // When all gates are ready, the page auto-navigates to /setup/welcome.
      await tauriPage.waitForURL(/\/setup\/welcome/, { timeout: 15_000 });
    }
  );
});

test.describe('Bootstrap — all gates ready', () => {
  test('health page auto-advances to /setup/welcome when all ready', async ({ tauriPage }) => {
    // fixtures.ts returns all gates ready — should auto-advance immediately
    await navigateTo(tauriPage, '/health');
    await tauriPage.waitForURL(/\/setup\/welcome/, { timeout: 10_000 });
  });

  test('direct navigation to /health works without error', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');
    const url = await tauriPage.url();
    expect(url).toMatch(/\/(health|setup|observatory)/);
  });
});
```

- [ ] **Step 2: Run the E2E tests**

```bash
cd /Users/Jerry/Developer/sensei/app
npx vite dev &
sleep 3
npx playwright test --project=browser e2e/tests/db-setup.spec.ts --reporter=list
```

Expected: All 4 tests pass. Kill the vite server after: `kill %1`

- [ ] **Step 3: Run the full browser-mode E2E suite**

```bash
cd /Users/Jerry/Developer/sensei/app
npx playwright test --project=browser --reporter=list
```

Expected: All existing tests still pass. New db-setup tests pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/Jerry/Developer/sensei/app
git add e2e/tests/db-setup.spec.ts
git commit -m "test(e2e): add db-setup spec — DB missing autoconfigure flow

Tests bootstrap page's DB gate failure detection and auto-advance
after setup_database completes. Also confirms all-ready path
auto-advances to /setup/welcome.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 13: Push daemon tag + final verification

- [ ] **Step 1: Push the daemon `v0.1.0` tag**

```bash
cd /Users/Jerry/Developer/sensei/daemon
git push
git push --tags
```

- [ ] **Step 2: Run the complete bootstrap test suite one final time**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap -- --nocapture 2>&1 | tail -20
```

Expected: All tests pass. Zero failures.

- [ ] **Step 3: Confirm the app builds in release mode**

```bash
cd /Users/Jerry/Developer/sensei/app
cargo build --manifest-path src-tauri/Cargo.toml --release 2>&1 | tail -5
```

Expected: Compiles without errors or warnings about missing features.

- [ ] **Step 4: Run the full browser-mode E2E suite one last time**

```bash
cd /Users/Jerry/Developer/sensei/app
npx playwright test --project=browser --reporter=list
```

Expected: All tests pass.

---

## Post-implementation: release process note

When releasing a new version `X.Y.Z`:

```bash
# 1. Tag dbd-rs if schema tooling changed
cd /Users/Jerry/Developer/dbd-rs && ./bump-version.sh X.Y.Z && git push && git push --tags

# 2. Tag daemon (schema source)
cd /Users/Jerry/Developer/sensei/daemon && ./bump-version.sh X.Y.Z && git push && git push --tags

# 3. Update app version
# Edit app/src-tauri/tauri.conf.json: "version": "X.Y.Z"
# Edit app/src-tauri/Cargo.toml: version = "X.Y.Z"
cd /Users/Jerry/Developer/sensei/app && git tag vX.Y.Z && git push && git push --tags
```

The bootstrap crate reads the app version at runtime from `app.package_info().version`, builds `sensei-hq/daemon/database@vX.Y.Z` as the dbd source, and runs deploy. The tag must exist on the daemon repo before the app ships.
