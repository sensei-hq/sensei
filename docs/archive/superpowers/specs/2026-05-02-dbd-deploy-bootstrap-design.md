---
name: dbd deploy + version upgrade bootstrap
description: Embed dbd-core into bootstrap for schema deploy; add version-aware upgrade gate; E2E test full boot cycle
date: 2026-05-02
status: approved
---

# dbd deploy + Version Upgrade Bootstrap

## Overview

Three goals:

1. **Schema deployment**: Replace the `senseid migrate` stub in `database::setup()` with a real `dbd deploy` using embedded `dbd-core`. On first boot or after a version upgrade, the bootstrap crate downloads the schema from `sensei-hq/sensei/database@v{version}` and applies it (DDL + seed data) to the local PostgreSQL database.

2. **Version upgrade gate**: Extend phase 1 (install prerequisites) to detect when the installed `sensei` CLI or `senseid` daemon binary is behind the app version, and run `brew upgrade` automatically.

3. **E2E test**: Browser-mode Playwright test covering the full cycle — app opens with DB missing, bootstrap shows gate五 failed, setup runs, gate becomes ready, app advances to `/setup/welcome`.

---

## Repos in scope

| Repo | Path |
|------|------|
| dbd-rs | `/Users/Jerry/Developer/dbd-rs` |
| sensei daemon (schema + bootstrap crate) | `/Users/Jerry/Developer/sensei/daemon` |
| sensei app | `/Users/Jerry/Developer/sensei/app` |

---

## dbd-rs changes

### 1. Fix cache-check bug (`crates/dbd-core/src/deploy.rs`)

**Bug**: `resolve_source()` checks `cache.join("design.yaml")` for the cache-hit guard. For a subpath source like `sensei-hq/sensei/database@v0.2.13`, the tarball extracts as `cache/database/design.yaml` — the check always misses and re-downloads on every run.

**Fix**:

```rust
// Before
if cache.join("design.yaml").exists() {
    return Ok(resolve_subpath(&cache, gh.subpath.as_deref()));
}

// After
let resolved = resolve_subpath(&cache, gh.subpath.as_deref());
if resolved.join("design.yaml").exists() {
    return Ok(resolved);
}
```

Add test: `resolve_source` with a pre-populated subpath cache dir returns without downloading.

### 2. Add `Design::deploy()` (`crates/dbd-core/src/design.rs`)

New method that calls `apply()` then `import_data()` — the full deployment pipeline (DDL + seed).

```rust
/// Deploy the full schema: apply DDL then import seed data.
///
/// Equivalent to `dbd apply` followed by `dbd import`.
/// dbd handles fresh / migrate / current automatically — idempotent.
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

Add tests: dry_run returns Ok, method calls both apply and import_data steps.

### 3. Add `bump-version.sh` (repo root)

Script that:
1. Updates `[workspace.package] version` in `Cargo.toml`
2. Runs `cargo check` to update `Cargo.lock`
3. Commits: `chore: bump version to vX.Y.Z`
4. Creates git tag `vX.Y.Z`

```bash
#!/usr/bin/env bash
set -euo pipefail
VERSION=${1:?usage: bump-version.sh X.Y.Z}
# Update Cargo.toml workspace version
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
cargo check -q
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to v$VERSION"
git tag "v$VERSION"
echo "Tagged v$VERSION — push with: git push && git push --tags"
```

---

## daemon repo changes

### 1. Add `bump-version.sh` (repo root)

Tags the commit consistently so `dbd deploy` can pin to it.

```bash
#!/usr/bin/env bash
set -euo pipefail
VERSION=${1:?usage: bump-version.sh X.Y.Z}
git tag "v$VERSION"
echo "Tagged v$VERSION — push with: git push --tags"
```

(The daemon repo version is kept in sync with the app version by running both bump scripts together during a release.)

---

## bootstrap crate changes (`daemon/crates/bootstrap`)

### 1. `Cargo.toml` — add dbd-core dependency

```toml
[dependencies]
# ... existing deps ...
dbd-core = { path = "../../../dbd-rs/crates/dbd-core" }
tokio = { version = "1", features = ["process", "time", "rt"] }  # add "rt"
```

> For release builds, path will be replaced with a crates.io version or git reference once dbd-core is published.

### 2. `database.rs` — new `deploy()`, updated `setup()`

**New function `deploy(app_version: &str) -> Result<ComponentStatus, String>`**:

```rust
pub fn deploy(app_version: &str) -> Result<ComponentStatus, String> {
    let db = db_name();
    let source = format!("sensei-hq/sensei/database@v{app_version}");
    let db_url = format!("postgres://localhost/{db}");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime error: {e}"))?;

    rt.block_on(async {
        // Resolve source (downloads tarball from GitHub if not cached)
        let project_dir = dbd_core::deploy::resolve_source(&source)
            .await
            .map_err(|e| format!("dbd resolve failed: {e}"))?;

        let config_path = project_dir.join("design.yaml");

        let mut design = dbd_core::Design::from_config_with_dir(
            &config_path, "prod", Some(&project_dir),
        )
        .map_err(|e| format!("dbd config failed: {e}"))?;

        let mut adapter =
            dbd_core::adapter::postgres::PostgresAdapter::new(&db_url, "sensei")
                .await
                .map_err(|e| format!("dbd connect failed: {e}"))?;

        design.deploy(&adapter, false)
            .await
            .map_err(|e| format!("dbd deploy failed: {e}"))?;

        Ok::<(), String>(())
    })?;

    let version = schema_version(db);
    Ok(ComponentStatus::ready("database", &format!("schema-{}", version.unwrap_or(0))))
}
```

**Updated `setup(app_version: &str)`** — replace `migrate()` call:

```rust
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

Remove `migrate()` — it was a stub and is fully replaced by `deploy()`.

### 3. `prereq/checker.rs` — add `VersionedBinaryChecker`

Checks that a binary exists AND its version string contains the expected version. Returns `fail("outdated: …")` if behind so the fixer knows to upgrade rather than install.

```rust
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
        // Extract bare semver from output like "sensei 0.2.13" or "0.2.13"
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

### 4. `prereq/fixer.rs` — add `BrewUpgradeFixer`

Tries `brew upgrade` first (handles version bump), falls back to `brew install` (handles first install). One fixer covers both cases so the factory stays simple.

```rust
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
        // Try upgrade first — works if already installed
        let upgrade = Command::new(&self.brew_path)
            .args(["upgrade", &self.formula])
            .output()
            .map_err(|e| format!("brew upgrade failed to run: {e}"))?;

        if upgrade.status.success() {
            return Ok(FixResult::new(format!("brew upgrade {}", self.formula)));
        }

        // Fall back to install — handles first install case
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

### 5. `prereq/factory.rs` — signature changes

**`install_prerequisites(provider, expected_version: &str)`** — use `VersionedBinaryChecker` + `BrewUpgradeFixer` for sensei and senseid:

```rust
pub fn install_prerequisites(
    provider: Arc<dyn PlatformProvider>,
    expected_version: &str,
) -> Vec<Box<dyn Prerequisite>> {
    match provider.platform() {
        Platform::MacOS | Platform::Linux => {
            let brew = detect_brew_path();
            let mk_fixer = |formula: &str| -> Box<dyn Fixer> {
                match &brew {
                    Some(p) => Box::new(BrewUpgradeFixer::new(p, formula)),
                    None => Box::new(NoopFixer::new("Homebrew not found")),
                }
            };
            vec![
                // postgresql — binary only (version not pinned to app version)
                Box::new(GenericPrerequisite::new(
                    "postgresql", "PostgreSQL",
                    Box::new(BinaryChecker::new("postgres", "--version")),
                    mk_fixer("postgresql@17"),
                    GateKind::Install, None,
                )),
                // ollama — binary only
                Box::new(GenericPrerequisite::new(
                    "ollama", "Ollama",
                    Box::new(BinaryChecker::new("ollama", "--version")),
                    mk_fixer("ollama"),
                    GateKind::Install, None,
                )),
                // sensei CLI — version-pinned to app version
                Box::new(GenericPrerequisite::new(
                    "sensei", "Sensei CLI",
                    Box::new(VersionedBinaryChecker::new("sensei", "--version", expected_version)),
                    mk_fixer("sensei-hq/tap/sensei"),
                    GateKind::Install, None,
                )),
                // senseid daemon — version-pinned to app version
                Box::new(GenericPrerequisite::new(
                    "senseid", "Sensei Daemon",
                    Box::new(VersionedBinaryChecker::new("senseid", "--version", expected_version)),
                    mk_fixer("sensei-hq/tap/senseid"),
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
                Box::new(VersionedBinaryChecker::new("sensei", "--version", expected_version)),
                Box::new(NoopFixer::new("Download sensei from sensei.so/download")),
                GateKind::Install, None,
            )),
            Box::new(GenericPrerequisite::new(
                "senseid", "Sensei Daemon",
                Box::new(VersionedBinaryChecker::new("senseid", "--version", expected_version)),
                Box::new(NoopFixer::new("Download senseid from sensei.so/download")),
                GateKind::Install, None,
            )),
        ],
    }
}
```

**`setup_database(app_version: &str)`** — pass version to `DatabaseSetupFixer`:

```rust
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

`DatabaseSetupFixer` gains a `app_version` field and passes it to `database::setup(app_version)`.

---

## Tauri commands (`app/src-tauri/src/commands/bootstrap.rs`)

Read app version once and thread it through:

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

---

## E2E tests (`app/e2e/tests/db-setup.spec.ts`)

Browser-mode test using stateful IPC mocks — simulates the DB-missing → setup → ready → advance cycle.

Tests use a separate `fixtures-db-missing.ts` that overrides the `run_bootstrap` and `setup_database` mocks. The `createTauriTest` fixture is called with per-test `ipcMocks`, same pattern as `fixtures.ts`.

**`fixtures-db-missing.ts`** — DB failed state then ready after setup:

```typescript
import { createTauriTest } from '@srsholmes/tauri-playwright';

// Shared call tracker — setup_database sets this so run_bootstrap can return ready
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
          { name: 'sensei',     state: { state: 'ready' }, version: '0.2.13', detail: null },
          { name: 'database',   state: dbState,            version: null,      detail: null },
          { name: 'daemon',     state: { state: 'ready' }, version: '0.2.13', detail: null },
        ],
      };
    },
    setup_database: () => { setupComplete = true; return null; },
    get_platform: () => ({ platform: 'macos', package_manager: 'homebrew',
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

**Test: `db missing → autoconfigure → advance to setup`**

```typescript
import { test, expect } from '../fixtures-db-missing';
import { navigateTo } from '../helpers';

test('bootstrap autoconfigures missing database and advances to /setup/welcome', async ({ tauriPage }) => {
  await navigateTo(tauriPage, '/health');

  // Gate五 (database) shows failed initially
  await expect(tauriPage.locator('[data-gate="database"] [data-status]'))
    .toHaveAttribute('data-status', 'failed');

  // Page triggers setup_database (auto or via "Fix" button) — mock marks DB ready
  // Bootstrap re-checks and all gates pass → auto-advance
  await tauriPage.waitForURL('/setup/welcome', { timeout: 10_000 });
});
```

Additional browser-mode tests (use existing `fixtures.ts`):
- `health page loads with all gates ready → auto-advances to /setup/welcome`
- `health page with sensei outdated → gate shows "outdated" status`

---

## Testing strategy

| Layer | What | How |
|-------|------|-----|
| `dbd-core` unit | `Design::deploy()` dry_run | Rust `#[tokio::test]` |
| `dbd-core` unit | cache-hit with subpath | `#[tokio::test]` with pre-populated tmp dir |
| `bootstrap` unit | `VersionedBinaryChecker` — outdated/current/missing | `#[test]` with fake binaries |
| `bootstrap` unit | `BrewUpgradeFixer` — no brew path → err | `#[test]` |
| `bootstrap` unit | `database::deploy` — no postgres → err | `#[test]` (pg not running) |
| `bootstrap` unit | `factory::setup_database` gate count | `#[test]` |
| `app/e2e` | DB missing → autoconfigure → /setup/welcome | Playwright browser-mode |
| `app/e2e` | All ready on load → auto-advance | Playwright browser-mode |

---

## Release process

All three repos must be tagged consistently for version pinning to work.

### Release checklist

```
1. dbd-rs:    ./bump-version.sh X.Y.Z && git push && git push --tags
2. daemon:    ./bump-version.sh X.Y.Z && git push && git push --tags
3. app:       update version in tauri.conf.json + Cargo.toml
              git tag vX.Y.Z && git push && git push --tags
```

`make bump v=X.Y.Z` from the monorepo root creates the tags that bootstrap uses as `sensei-hq/sensei/database@vX.Y.Z`. The app reads its own version from `tauri.conf.json` at runtime and threads it through to the bootstrap factory.

> **Future**: A single top-level release script (`release.sh X.Y.Z`) should automate all three steps and verify tags exist before building the app.

---

## Out of scope (future)

- `DATABASE_URL` with username/password (currently trusts local pg_hba.conf)
- Windows upgrade path (brew not available — needs separate tooling)
- `sensei doctor` CLI triggering dbd deploy (needs bootstrap feature flag or separate deploy crate)
- Automated release script across repos
