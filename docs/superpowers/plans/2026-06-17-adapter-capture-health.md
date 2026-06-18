# ACP Adapter Health & Capture Watchdog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the daemon a per-adapter health check + auto-resolver, an hourly capture watchdog with a circuit breaker and system notifications, and a `sensei doctor` section that consumes it — the durable fix for silently-broken capture.

**Architecture:** Pure, unit-testable building blocks (health vocabulary, freshness math, Claude JSON probes) in `assistants/health.rs` + `assistants/claude_code.rs`; a daemon orchestrator (`assistants/watchdog.rs`) that adds the DB-backed events-freshness check, runs the hourly sweep with an in-memory circuit breaker, and fires notifications; HTTP endpoints so the CLI (`sensei doctor`) and app are thin consumers.

**Tech Stack:** Rust, axum, sqlx (Postgres), tokio, `notify-rust` (new), `reqwest` blocking (CLI), `owo-colors` (CLI).

**Spec:** `docs/superpowers/specs/2026-06-17-adapter-capture-health-design.md`

**Conventions to honor:**
- pg_store query style: `sqlx_core::query_as::query_as("SQL").bind(x).fetch_one(&self.pool).await.map_err(|e| e.to_string())?`.
- `assistant_family` is a Postgres enum — bind with a `$1::sensei.assistant_family` cast.
- Run the workspace tests with `cargo test -p senseid` (daemon) and `cargo test -p sensei-cli` (CLI). The pre-commit hook runs `make test-fast`.
- TDD: failing test first, every task ends in a commit. Work on branch `develop`.

---

## File Structure

**Create:**
- `crates/senseid/src/assistants/health.rs` — pure types (`CheckStatus`, `AdapterCheck`, `AdapterHealth`, `AdapterResolveReport`), status aggregation, `business_elapsed_hours`, `capture_freshness`.
- `crates/senseid/src/assistants/watchdog.rs` — `health_report()` orchestrator (reads window config, merges DB freshness), circuit-breaker types, `tick_adapter()` policy, `spawn_watchdog()` loop.
- `crates/senseid/src/notifications.rs` — `Notifier` trait, `NotifyLevel`, `NotifyRustNotifier`, `NoopNotifier`.

**Modify:**
- `crates/senseid/src/assistants/trait_def.rs` — add `config_health()` + `resolve()` trait methods (defaults).
- `crates/senseid/src/assistants/claude_code.rs` — four pure probe fns + `config_health()` override.
- `crates/senseid/src/assistants/mod.rs` — declare modules, re-export public types, expose `health_report`.
- `crates/senseid/src/db/pg_store.rs` — `latest_hook_event_ts(family)`.
- `crates/senseid/src/api/handlers/config.rs` — `assistants_health` + `assistants_resolve` handlers.
- `crates/senseid/src/api/routes.rs` — register two routes.
- `crates/senseid/src/api/server.rs` — construct notifier + spawn watchdog.
- `crates/senseid/src/main.rs` — `mod notifications;`.
- `crates/senseid/Cargo.toml` — add `notify-rust`.
- `crates/cli/src/doctor.rs` — Capture/Assistants section + `--fix`.
- `crates/cli/src/main.rs` — `Doctor { fix: bool }`.

---

## Task 1: Health vocabulary + status aggregation (pure)

**Files:**
- Create: `crates/senseid/src/assistants/health.rs`
- Modify: `crates/senseid/src/assistants/mod.rs` (add `mod health;` + re-export)

- [ ] **Step 1: Write the failing test**

Add to a new file `crates/senseid/src/assistants/health.rs`:

```rust
//! Pure, DB-free health vocabulary + freshness math for ACP adapters.
//! Everything here is unit-testable without a daemon, DB, or network.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus { Ok, Warn, Fail, Unknown }

impl CheckStatus {
    /// Severity for "worst-of" aggregation: Fail > Warn > Unknown > Ok.
    fn rank(self) -> u8 {
        match self { CheckStatus::Ok => 0, CheckStatus::Unknown => 1, CheckStatus::Warn => 2, CheckStatus::Fail => 3 }
    }
    /// The more-severe of two statuses.
    pub fn worse(self, other: CheckStatus) -> CheckStatus {
        if other.rank() > self.rank() { other } else { self }
    }
    /// Aggregate an iterator of statuses to the worst. Empty => Ok.
    pub fn worst_of<'a>(it: impl Iterator<Item = &'a CheckStatus>) -> CheckStatus {
        it.fold(CheckStatus::Ok, |acc, s| acc.worse(*s))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCheck {
    pub id: String,
    pub label: String,
    pub status: CheckStatus,
    pub detail: Option<String>,
}

impl AdapterCheck {
    pub fn new(id: &str, label: &str, status: CheckStatus, detail: Option<String>) -> Self {
        Self { id: id.to_string(), label: label.to_string(), status, detail }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterHealth {
    pub adapter_id: String,
    pub family: String,
    pub status: CheckStatus,
    pub checks: Vec<AdapterCheck>,
    pub resolvable: bool,
}

impl AdapterHealth {
    /// Build with `status` computed as the worst of `checks`.
    pub fn new(adapter_id: &str, family: &str, checks: Vec<AdapterCheck>, resolvable: bool) -> Self {
        let status = CheckStatus::worst_of(checks.iter().map(|c| &c.status));
        Self { adapter_id: adapter_id.to_string(), family: family.to_string(), status, checks, resolvable }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterResolveReport {
    pub adapter_id: String,
    pub ok: bool,
    pub actions: Vec<String>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worst_of_picks_fail_over_warn_over_ok() {
        let s = [CheckStatus::Ok, CheckStatus::Warn, CheckStatus::Fail];
        assert_eq!(CheckStatus::worst_of(s.iter()), CheckStatus::Fail);
        let s2 = [CheckStatus::Ok, CheckStatus::Warn];
        assert_eq!(CheckStatus::worst_of(s2.iter()), CheckStatus::Warn);
        let empty: [CheckStatus; 0] = [];
        assert_eq!(CheckStatus::worst_of(empty.iter()), CheckStatus::Ok);
    }

    #[test]
    fn unknown_is_worse_than_ok_but_better_than_warn() {
        assert_eq!(CheckStatus::Ok.worse(CheckStatus::Unknown), CheckStatus::Unknown);
        assert_eq!(CheckStatus::Unknown.worse(CheckStatus::Warn), CheckStatus::Warn);
    }

    #[test]
    fn adapter_health_status_is_worst_of_checks() {
        let checks = vec![
            AdapterCheck::new("a", "A", CheckStatus::Ok, None),
            AdapterCheck::new("b", "B", CheckStatus::Fail, Some("boom".into())),
        ];
        let h = AdapterHealth::new("claude-code", "claude", checks, true);
        assert_eq!(h.status, CheckStatus::Fail);
        assert_eq!(h.checks.len(), 2);
    }

    #[test]
    fn check_status_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&CheckStatus::Fail).unwrap(), "\"fail\"");
        assert_eq!(serde_json::to_string(&CheckStatus::Ok).unwrap(), "\"ok\"");
    }
}
```

In `crates/senseid/src/assistants/mod.rs`, add after the existing `mod` lines (line ~4):

```rust
mod health;
pub use health::{AdapterCheck, AdapterHealth, AdapterResolveReport, CheckStatus};
```

- [ ] **Step 2: Run test to verify it fails (then passes — it's self-contained)**

Run: `cargo test -p senseid assistants::health:: 2>&1 | tail -20`
Expected: compiles and the four tests PASS (this task is pure; the "failing" state is only if a typo breaks it).

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/assistants/health.rs crates/senseid/src/assistants/mod.rs
git commit -m "feat(assistants): health vocabulary + status aggregation"
```

---

## Task 2: Capture freshness + business-time helpers (pure)

**Files:**
- Modify: `crates/senseid/src/assistants/health.rs`

- [ ] **Step 1: Write the failing test**

Append inside `health.rs` (above `#[cfg(test)]`), the implementation:

```rust
use chrono::{Datelike, TimeZone, Utc, Weekday};

/// Hours elapsed between two epoch-millis instants. When `exclude_weekends`,
/// any whole or partial Saturday/Sunday is removed from the elapsed total, so a
/// Friday-afternoon → Monday-morning gap counts only the working hours.
///
/// Implementation: walk the span in UTC and sum only the milliseconds that fall
/// on Mon–Fri. Coarse (1-minute step) on purpose — freshness thresholds are in
/// hours, so minute granularity is precise enough and keeps the loop cheap.
pub fn business_elapsed_hours(from_ms: i64, to_ms: i64, exclude_weekends: bool) -> f64 {
    if to_ms <= from_ms { return 0.0; }
    if !exclude_weekends {
        return (to_ms - from_ms) as f64 / 3_600_000.0;
    }
    const STEP_MS: i64 = 60_000; // 1 minute
    let mut counted_ms: i64 = 0;
    let mut t = from_ms;
    while t < to_ms {
        let dt = Utc.timestamp_millis_opt(t).single();
        let is_weekend = dt.map(|d| matches!(d.weekday(), Weekday::Sat | Weekday::Sun)).unwrap_or(false);
        let next = (t + STEP_MS).min(to_ms);
        if !is_weekend { counted_ms += next - t; }
        t = next;
    }
    counted_ms as f64 / 3_600_000.0
}

/// The capture-freshness check for an assistant family.
/// `last_ts` = newest hook_event ts (epoch ms) for the family, or None if the
/// daemon has never recorded one. `now_ms` = current epoch ms.
pub fn capture_freshness(
    last_ts: Option<i64>,
    now_ms: i64,
    window_hours: f64,
    exclude_weekends: bool,
) -> AdapterCheck {
    match last_ts {
        None => AdapterCheck::new(
            "events", "events flowing", CheckStatus::Warn,
            Some("never captured — no hook events recorded yet".into()),
        ),
        Some(ts) => {
            let elapsed = business_elapsed_hours(ts, now_ms, exclude_weekends);
            if elapsed <= window_hours {
                AdapterCheck::new("events", "events flowing", CheckStatus::Ok,
                    Some(format!("last event {:.1}h ago", elapsed)))
            } else {
                AdapterCheck::new("events", "events flowing", CheckStatus::Fail,
                    Some(format!("stale: last event {:.1}h ago (window {}h)", elapsed, window_hours)))
            }
        }
    }
}
```

Append these tests inside the existing `mod tests`:

```rust
    // 2026-06-12 is a Friday. 16:00Z Fri → 10:00Z Mon.
    fn ms(s: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp_millis()
    }

    #[test]
    fn business_elapsed_excludes_weekend() {
        let fri = ms("2026-06-12T16:00:00Z");
        let mon = ms("2026-06-15T10:00:00Z");
        // Wall-clock ~66h; business time = 8h (Fri 16→24) + 10h (Mon 0→10) = 18h.
        let h = business_elapsed_hours(fri, mon, true);
        assert!((h - 18.0).abs() < 0.5, "expected ~18 business hours, got {h}");
    }

    #[test]
    fn business_elapsed_full_clock_when_not_excluding() {
        let fri = ms("2026-06-12T16:00:00Z");
        let mon = ms("2026-06-15T10:00:00Z");
        let h = business_elapsed_hours(fri, mon, false);
        assert!((h - 66.0).abs() < 0.5, "expected ~66 wall-clock hours, got {h}");
    }

    #[test]
    fn freshness_none_is_warn_never_captured() {
        let c = capture_freshness(None, ms("2026-06-15T10:00:00Z"), 24.0, true);
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.unwrap().contains("never captured"));
    }

    #[test]
    fn freshness_within_window_is_ok() {
        let now = ms("2026-06-15T10:00:00Z");
        let two_h_ago = ms("2026-06-15T08:00:00Z");
        assert_eq!(capture_freshness(Some(two_h_ago), now, 24.0, true).status, CheckStatus::Ok);
    }

    #[test]
    fn freshness_weekend_gap_stays_ok_but_full_clock_fails() {
        let mon = ms("2026-06-15T10:00:00Z");
        let fri = ms("2026-06-12T16:00:00Z");
        // 18 business hours < 24 → Ok; 66 wall-clock hours > 24 → Fail.
        assert_eq!(capture_freshness(Some(fri), mon, 24.0, true).status, CheckStatus::Ok);
        assert_eq!(capture_freshness(Some(fri), mon, 24.0, false).status, CheckStatus::Fail);
    }
```

- [ ] **Step 2: Run test to verify**

Run: `cargo test -p senseid assistants::health::tests:: 2>&1 | tail -20`
Expected: all freshness/business tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/assistants/health.rs
git commit -m "feat(assistants): capture freshness + weekend-aware elapsed time"
```

---

## Task 3: Claude config probes (pure FS)

**Files:**
- Modify: `crates/senseid/src/assistants/claude_code.rs`

These read JSON files (no `claude` CLI). Each takes explicit paths so tests use tempdirs, mirroring the existing `verify_plugin_installed` tests.

- [ ] **Step 1: Write the failing tests**

Add these free functions near the top of `claude_code.rs` (after the existing `verify_plugin_installed`):

```rust
use crate::assistants::{AdapterCheck, CheckStatus};

/// settings.json → enabledPlugins["sensei@sensei-marketplace"] == true.
pub(super) fn check_enabled(settings_path: &Path) -> AdapterCheck {
    let id = "enabled"; let label = "plugin enabled";
    let Some(content) = std::fs::read_to_string(settings_path).ok() else {
        return AdapterCheck::new(id, label, CheckStatus::Fail, Some("settings.json missing".into()));
    };
    let Some(v) = json5::from_str::<serde_json::Value>(&content).ok() else {
        return AdapterCheck::new(id, label, CheckStatus::Unknown, Some("settings.json unparseable".into()));
    };
    let enabled = v.get("enabledPlugins")
        .and_then(|m| m.get("sensei@sensei-marketplace"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    if enabled {
        AdapterCheck::new(id, label, CheckStatus::Ok, None)
    } else {
        AdapterCheck::new(id, label, CheckStatus::Fail, Some("enabledPlugins[sensei@sensei-marketplace] != true".into()))
    }
}

/// settings.json → extraKnownMarketplaces["sensei-marketplace"] present.
pub(super) fn check_marketplace(settings_path: &Path) -> AdapterCheck {
    let id = "marketplace"; let label = "marketplace registered";
    let Some(content) = std::fs::read_to_string(settings_path).ok() else {
        return AdapterCheck::new(id, label, CheckStatus::Fail, Some("settings.json missing".into()));
    };
    let Some(v) = json5::from_str::<serde_json::Value>(&content).ok() else {
        return AdapterCheck::new(id, label, CheckStatus::Unknown, Some("settings.json unparseable".into()));
    };
    let present = v.get("extraKnownMarketplaces")
        .and_then(|m| m.get("sensei-marketplace"))
        .is_some();
    if present {
        AdapterCheck::new(id, label, CheckStatus::Ok, None)
    } else {
        AdapterCheck::new(id, label, CheckStatus::Fail, Some("sensei-marketplace not registered".into()))
    }
}

/// installed_plugins.json records sensei (reuses verify_plugin_installed).
pub(super) fn check_plugin(manifest_path: &Path) -> AdapterCheck {
    if verify_plugin_installed(manifest_path, "sensei") {
        AdapterCheck::new("plugin", "plugin installed", CheckStatus::Ok, None)
    } else {
        AdapterCheck::new("plugin", "plugin installed", CheckStatus::Fail,
            Some("sensei not recorded in installed_plugins.json".into()))
    }
}

/// The plugin's installPath exists and its hooks/hooks.json declares both
/// PreToolUse and PostToolUse. `install_path` is read from installed_plugins.json.
pub(super) fn check_hooks(install_path: Option<&Path>) -> AdapterCheck {
    let id = "hooks"; let label = "hooks registered";
    let Some(dir) = install_path else {
        return AdapterCheck::new(id, label, CheckStatus::Fail, Some("no plugin installPath recorded".into()));
    };
    if !dir.exists() {
        return AdapterCheck::new(id, label, CheckStatus::Fail, Some(format!("installPath missing: {}", dir.display())));
    }
    let hooks_file = dir.join("hooks/hooks.json");
    let Some(content) = std::fs::read_to_string(&hooks_file).ok() else {
        return AdapterCheck::new(id, label, CheckStatus::Fail, Some("hooks/hooks.json missing".into()));
    };
    let Some(v) = json5::from_str::<serde_json::Value>(&content).ok() else {
        return AdapterCheck::new(id, label, CheckStatus::Unknown, Some("hooks.json unparseable".into()));
    };
    let hooks = v.get("hooks");
    let has = |evt: &str| hooks.and_then(|h| h.get(evt)).is_some();
    if has("PreToolUse") && has("PostToolUse") {
        AdapterCheck::new(id, label, CheckStatus::Ok, None)
    } else {
        AdapterCheck::new(id, label, CheckStatus::Fail, Some("PreToolUse/PostToolUse not declared in hooks.json".into()))
    }
}

/// Read the recorded installPath for `sensei@*` from installed_plugins.json.
pub(super) fn plugin_install_path(manifest_path: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let v = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let plugins = v.get("plugins")?.as_object()?;
    let (_k, entries) = plugins.iter().find(|(k, _)| k.starts_with("sensei@"))?;
    let first = entries.as_array()?.first()?;
    first.get("installPath").and_then(|p| p.as_str()).map(PathBuf::from)
}
```

Add tests in the `mod tests` block:

```rust
    // ── config probes ──────────────────────────────────────────────────
    #[test]
    fn check_enabled_true_when_flag_set() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, r#"{"enabledPlugins":{"sensei@sensei-marketplace":true}}"#).unwrap();
        assert_eq!(check_enabled(&s).status, CheckStatus::Ok);
    }
    #[test]
    fn check_enabled_fail_when_false_or_missing() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, r#"{"enabledPlugins":{"sensei@sensei-marketplace":false}}"#).unwrap();
        assert_eq!(check_enabled(&s).status, CheckStatus::Fail);
        let s2 = tmp.path().join("missing.json");
        assert_eq!(check_enabled(&s2).status, CheckStatus::Fail);
    }
    #[test]
    fn check_enabled_unknown_when_malformed() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, "{ not json").unwrap();
        // json5 is lenient; use clearly invalid content.
        std::fs::write(&s, "}{").unwrap();
        assert_eq!(check_enabled(&s).status, CheckStatus::Unknown);
    }
    #[test]
    fn check_marketplace_ok_when_registered() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, r#"{"extraKnownMarketplaces":{"sensei-marketplace":{"source":{}}}}"#).unwrap();
        assert_eq!(check_marketplace(&s).status, CheckStatus::Ok);
    }
    #[test]
    fn check_marketplace_fail_when_absent() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, r#"{"extraKnownMarketplaces":{}}"#).unwrap();
        assert_eq!(check_marketplace(&s).status, CheckStatus::Fail);
    }
    #[test]
    fn check_plugin_reuses_verify() {
        let tmp = make_tmp_home();
        let m = tmp.path().join("installed_plugins.json");
        std::fs::write(&m, r#"{"plugins":{"sensei@sensei-marketplace":[{"installPath":"/x"}]}}"#).unwrap();
        assert_eq!(check_plugin(&m).status, CheckStatus::Ok);
    }
    #[test]
    fn plugin_install_path_reads_first_entry() {
        let tmp = make_tmp_home();
        let m = tmp.path().join("installed_plugins.json");
        std::fs::write(&m, r#"{"plugins":{"sensei@sensei-marketplace":[{"installPath":"/foo/bar"}]}}"#).unwrap();
        assert_eq!(plugin_install_path(&m), Some(PathBuf::from("/foo/bar")));
    }
    #[test]
    fn check_hooks_ok_when_both_events_declared() {
        let tmp = make_tmp_home();
        let dir = tmp.path().join("plugin");
        std::fs::create_dir_all(dir.join("hooks")).unwrap();
        std::fs::write(dir.join("hooks/hooks.json"),
            r#"{"hooks":{"PreToolUse":[{}],"PostToolUse":[{}],"SessionStart":[{}]}}"#).unwrap();
        assert_eq!(check_hooks(Some(&dir)).status, CheckStatus::Ok);
    }
    #[test]
    fn check_hooks_fail_when_path_missing() {
        assert_eq!(check_hooks(None).status, CheckStatus::Fail);
        let tmp = make_tmp_home();
        assert_eq!(check_hooks(Some(&tmp.path().join("nope"))).status, CheckStatus::Fail);
    }
    #[test]
    fn check_hooks_fail_when_events_absent() {
        let tmp = make_tmp_home();
        let dir = tmp.path().join("plugin");
        std::fs::create_dir_all(dir.join("hooks")).unwrap();
        std::fs::write(dir.join("hooks/hooks.json"), r#"{"hooks":{"SessionStart":[{}]}}"#).unwrap();
        assert_eq!(check_hooks(Some(&dir)).status, CheckStatus::Fail);
    }
```

> Note: `make_tmp_home()` and `use tempfile::TempDir` already exist in this test module. `json5` is already a daemon dependency (used in `clean_sensei_from_mcp_file`).

- [ ] **Step 2: Run tests to verify they fail then pass**

Run: `cargo test -p senseid assistants::claude_code::tests::check 2>&1 | tail -25`
Expected: the new `check_*` / `plugin_install_path` tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/assistants/claude_code.rs
git commit -m "feat(claude): pure marketplace/plugin/enabled/hooks probes"
```

---

## Task 4: `Assistant` trait health/resolve + Claude override

**Files:**
- Modify: `crates/senseid/src/assistants/trait_def.rs`
- Modify: `crates/senseid/src/assistants/claude_code.rs`

- [ ] **Step 1: Write the failing test (trait default via a stub) + add trait methods**

In `trait_def.rs`, add imports and two methods to the `Assistant` trait (after `remove`):

```rust
use crate::assistants::health::{AdapterCheck, AdapterHealth, AdapterResolveReport, CheckStatus};
```

```rust
    /// Pure, filesystem-only health probes for this adapter. Default = one
    /// check derived from `is_configured()`. Override for richer adapters.
    fn config_health(&self) -> Vec<AdapterCheck> {
        let status = if self.is_configured() { CheckStatus::Ok } else { CheckStatus::Fail };
        let detail = (status == CheckStatus::Fail).then(|| "sensei not configured in this assistant".to_string());
        vec![AdapterCheck::new("configured", "configured", status, detail)]
    }

    /// Auto-resolve: default = re-run `configure()` (the install/reinstall flow)
    /// and map its result. `mcp_cmd` is resolved by the caller.
    fn resolve(&self, mcp_cmd: &str) -> AdapterResolveReport {
        match self.configure(mcp_cmd) {
            Ok(ok) => AdapterResolveReport {
                adapter_id: self.id().to_string(),
                ok: true,
                actions: {
                    let mut a = vec![format!("configured {}", self.id())];
                    a.extend(ok.warnings);
                    a
                },
                errors: vec![],
            },
            Err(e) => AdapterResolveReport {
                adapter_id: self.id().to_string(),
                ok: false,
                actions: vec![],
                errors: vec![e],
            },
        }
    }

    /// Convenience: the adapter's config-side AdapterHealth (no DB checks).
    fn config_health_report(&self) -> AdapterHealth {
        AdapterHealth::new(self.id(), self.family(), self.config_health(), true)
    }
```

Add a test module at the bottom of `trait_def.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct StubAssistant { configured: bool }
    impl Assistant for StubAssistant {
        fn id(&self) -> &str { "stub" }
        fn name(&self) -> &str { "Stub" }
        fn mcp_key(&self) -> &str { "mcpServers" }
        fn config_path(&self) -> PathBuf { PathBuf::from("/dev/null") }
        fn detect(&self) -> bool { true }
        fn configure(&self, _mcp_cmd: &str) -> Result<AssistantConfigureOk, String> {
            Ok(AssistantConfigureOk { plugin: false, warnings: vec![] })
        }
        fn remove(&self) -> bool { true }
        fn is_configured(&self) -> bool { self.configured }
    }

    #[test]
    fn default_config_health_reflects_is_configured() {
        assert_eq!(StubAssistant { configured: true }.config_health()[0].status, CheckStatus::Ok);
        assert_eq!(StubAssistant { configured: false }.config_health()[0].status, CheckStatus::Fail);
    }

    #[test]
    fn default_resolve_maps_configure_success() {
        let r = StubAssistant { configured: false }.resolve("sensei-mcp");
        assert!(r.ok);
        assert_eq!(r.adapter_id, "stub");
    }
}
```

In `claude_code.rs`, override `config_health()` inside `impl Assistant for ClaudeCodeAssistant` (after `is_configured`):

```rust
    fn config_health(&self) -> Vec<AdapterCheck> {
        let settings = self.config_path();                 // ~/.claude/settings.json
        let manifest = installed_plugins_manifest();       // ~/.claude/plugins/installed_plugins.json
        let install_path = plugin_install_path(&manifest);
        vec![
            check_marketplace(&settings),
            check_plugin(&manifest),
            check_enabled(&settings),
            check_hooks(install_path.as_deref()),
        ]
    }
```

Add the import at the top of `claude_code.rs`:

```rust
use crate::assistants::health::AdapterCheck;
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p senseid assistants:: 2>&1 | tail -25`
Expected: trait stub tests + existing assistant tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/assistants/trait_def.rs crates/senseid/src/assistants/claude_code.rs
git commit -m "feat(assistants): trait config_health/resolve + Claude override"
```

---

## Task 5: `latest_hook_event_ts` on PgStore

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Write the failing test + implementation**

Add after `insert_hook_event` (line ~1793):

```rust
    /// Newest hook_event timestamp (epoch ms) for an assistant family, or None
    /// when the daemon has never recorded one for it. `assistant_family` is a
    /// Postgres enum, so bind with the explicit cast.
    pub async fn latest_hook_event_ts(&self, family: &str) -> Result<Option<i64>, String> {
        let row: (Option<i64>,) = sqlx_core::query_as::query_as(
            "SELECT max(ts) FROM activity.hook_events WHERE assistant_family = $1::sensei.assistant_family"
        )
        .bind(family)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }
```

Add a DB test. Find the existing `#[cfg(test)] mod tests` in `pg_store.rs` (search `connect_test`) and add:

```rust
    #[tokio::test]
    async fn latest_hook_event_ts_returns_max_for_family() {
        let pg = PgStore::connect_test().await.unwrap();
        let base = 1_900_000_000_000_i64; // far-future, won't collide with seeded data
        for (i, off) in [0_i64, 5000, 2000].iter().enumerate() {
            pg.insert_hook_event(
                &format!("sess-test-{i}"), "claude", "PreToolUse", Some("Bash"),
                Some("/tmp"), base + off, Some(true), &serde_json::json!({"t": i}),
            ).await.unwrap();
        }
        let max = pg.latest_hook_event_ts("claude").await.unwrap().unwrap();
        assert!(max >= base + 5000, "expected >= {} got {max}", base + 5000);
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p senseid latest_hook_event_ts 2>&1 | tail -20`
Expected: PASS (requires a reachable test DB — same harness as other `connect_test` tests).

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(db): latest_hook_event_ts(family) for capture freshness"
```

---

## Task 6: `health_report` orchestrator + window config

**Files:**
- Create: `crates/senseid/src/assistants/watchdog.rs`
- Modify: `crates/senseid/src/assistants/mod.rs`

- [ ] **Step 1: Implementation + test**

Create `crates/senseid/src/assistants/watchdog.rs`:

```rust
//! Daemon-side orchestration: merges the DB-backed capture-freshness check into
//! each adapter's pure config_health, runs the hourly watchdog with a circuit
//! breaker, and fires notifications. The pure parts (config keys, tick policy)
//! are unit-tested; the loop is thin glue.

use crate::assistants::health::{capture_freshness, AdapterCheck, AdapterHealth, CheckStatus};
use crate::db::pg_store::PgStore;

pub const DEFAULT_WINDOW_HOURS: f64 = 24.0;
pub const DEFAULT_EXCLUDE_WEEKENDS: bool = true;

#[derive(Debug, Clone, Copy)]
pub struct CaptureWindow { pub hours: f64, pub exclude_weekends: bool }

impl Default for CaptureWindow {
    fn default() -> Self { Self { hours: DEFAULT_WINDOW_HOURS, exclude_weekends: DEFAULT_EXCLUDE_WEEKENDS } }
}

/// Parse the two config strings into a CaptureWindow, falling back to defaults
/// on missing/garbage values. Pure — unit-tested without a DB.
pub fn parse_window(hours: Option<&str>, exclude_weekends: Option<&str>) -> CaptureWindow {
    CaptureWindow {
        hours: hours.and_then(|s| s.trim().parse::<f64>().ok()).filter(|h| *h > 0.0).unwrap_or(DEFAULT_WINDOW_HOURS),
        exclude_weekends: exclude_weekends.and_then(|s| s.trim().parse::<bool>().ok()).unwrap_or(DEFAULT_EXCLUDE_WEEKENDS),
    }
}

/// Load the capture window from sensei.config (keys
/// `capture.max_inactivity_hours`, `capture.exclude_weekends`).
pub async fn load_window(pg: &PgStore) -> CaptureWindow {
    let hours = pg.get_config("capture.max_inactivity_hours").await.ok().flatten();
    let weekends = pg.get_config("capture.exclude_weekends").await.ok().flatten();
    parse_window(hours.as_deref(), weekends.as_deref())
}

/// Compute health for every configured adapter, appending the DB-backed
/// `events` freshness check to the `claude` family.
pub async fn health_report(pg: &PgStore, now_ms: i64) -> Vec<AdapterHealth> {
    let window = load_window(pg).await;
    let mut out = Vec::new();
    for status in crate::assistants::detect() {
        if !status.configured { continue; }
        let mut checks = config_health_for(&status.id);
        if status.family == "claude" {
            let last = pg.latest_hook_event_ts("claude").await.ok().flatten();
            checks.push(capture_freshness(last, now_ms, window.hours, window.exclude_weekends));
        }
        out.push(AdapterHealth::new(&status.id, &status.family, checks, true));
    }
    out
}

/// config_health for an adapter id, via the registry. Returns a single Unknown
/// check if the id is not in the registry (defensive).
fn config_health_for(adapter_id: &str) -> Vec<AdapterCheck> {
    crate::assistants::config_health_for_id(adapter_id)
        .unwrap_or_else(|| vec![AdapterCheck::new("configured", "configured", CheckStatus::Unknown,
            Some(format!("unknown adapter {adapter_id}")))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_window_defaults_on_garbage() {
        let w = parse_window(None, None);
        assert_eq!(w.hours, 24.0);
        assert!(w.exclude_weekends);
        let w2 = parse_window(Some("abc"), Some("nope"));
        assert_eq!(w2.hours, 24.0);
        assert!(w2.exclude_weekends);
    }
    #[test]
    fn parse_window_reads_values() {
        let w = parse_window(Some("6"), Some("false"));
        assert_eq!(w.hours, 6.0);
        assert!(!w.exclude_weekends);
    }
    #[test]
    fn parse_window_rejects_nonpositive_hours() {
        assert_eq!(parse_window(Some("0"), None).hours, 24.0);
        assert_eq!(parse_window(Some("-3"), None).hours, 24.0);
    }
}
```

In `assistants/mod.rs`, add a registry helper so the orchestrator can fetch one adapter's config_health by id (keeps `all_assistants()` private). Add `mod watchdog;` and:

```rust
/// config_health for a single adapter id, or None if not registered.
pub fn config_health_for_id(adapter_id: &str) -> Option<Vec<crate::assistants::AdapterCheck>> {
    all_assistants().iter().find(|a| a.id() == adapter_id).map(|a| a.config_health())
}
```

Also re-export from mod.rs: `pub use watchdog::{health_report, CaptureWindow};`

- [ ] **Step 2: Run tests**

Run: `cargo test -p senseid assistants::watchdog::tests:: 2>&1 | tail -20`
Expected: the three `parse_window` tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/assistants/watchdog.rs crates/senseid/src/assistants/mod.rs
git commit -m "feat(assistants): health_report orchestrator + capture window config"
```

---

## Task 7: Notifications module (`notify-rust`)

**Files:**
- Create: `crates/senseid/src/notifications.rs`
- Modify: `crates/senseid/src/main.rs` (add `pub mod notifications;`)
- Modify: `crates/senseid/Cargo.toml`

- [ ] **Step 1: Add the dependency**

In `crates/senseid/Cargo.toml`, under `[dependencies]` (near the existing `notify = "8"` line), add:

```toml
notify-rust = "4"
```

Run: `cargo build -p senseid 2>&1 | tail -5`
Expected: builds (downloads `notify-rust`).

- [ ] **Step 2: Write the module + tests**

Create `crates/senseid/src/notifications.rs`:

```rust
//! Cross-platform desktop notifications for the daemon, behind a trait so unit
//! tests never fire a real OS notification. NOTE: the module is `notifications`
//! (not `notify`) because the `notify` crate name is taken by the fs-watcher.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyLevel { Info, Warn, Critical }

pub trait Notifier: Send + Sync {
    fn notify(&self, level: NotifyLevel, title: &str, body: &str);
}

/// Production notifier backed by notify-rust. Failures are logged, never
/// propagated — a missing notification daemon must not break the watchdog.
pub struct DesktopNotifier;

impl Notifier for DesktopNotifier {
    fn notify(&self, level: NotifyLevel, title: &str, body: &str) {
        let prefix = match level {
            NotifyLevel::Info => "✓", NotifyLevel::Warn => "⚠", NotifyLevel::Critical => "✗",
        };
        let result = notify_rust::Notification::new()
            .summary(&format!("{prefix} sensei: {title}"))
            .body(body)
            .appname("sensei")
            .show();
        match result {
            Ok(_) => tracing::info!(level = ?level, %title, "desktop notification shown"),
            Err(e) => tracing::warn!(level = ?level, %title, error = %e, "desktop notification failed"),
        }
    }
}

/// No-op notifier for tests.
pub struct NoopNotifier;
impl Notifier for NoopNotifier {
    fn notify(&self, _l: NotifyLevel, _t: &str, _b: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records calls so watchdog tests can assert on what was notified.
    pub struct RecordingNotifier(pub Mutex<Vec<(NotifyLevel, String)>>);
    impl Notifier for RecordingNotifier {
        fn notify(&self, level: NotifyLevel, title: &str, _body: &str) {
            self.0.lock().unwrap().push((level, title.to_string()));
        }
    }

    #[test]
    fn noop_notifier_does_nothing() {
        NoopNotifier.notify(NotifyLevel::Info, "t", "b"); // must not panic
    }

    #[test]
    fn recording_notifier_captures_calls() {
        let r = RecordingNotifier(Mutex::new(vec![]));
        r.notify(NotifyLevel::Critical, "boom", "body");
        assert_eq!(r.0.lock().unwrap()[0], (NotifyLevel::Critical, "boom".to_string()));
    }
}
```

In `crates/senseid/src/main.rs`, add to the module list (near line 16):

```rust
pub mod notifications;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p senseid notifications:: 2>&1 | tail -15`
Expected: both tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/Cargo.toml crates/senseid/src/notifications.rs crates/senseid/src/main.rs Cargo.lock
git commit -m "feat(daemon): notifications module (notify-rust) with injectable Notifier"
```

---

## Task 8: Watchdog tick policy + circuit breaker

**Files:**
- Modify: `crates/senseid/src/assistants/watchdog.rs`

- [ ] **Step 1: Implementation + test**

Append to `watchdog.rs` (above the test module):

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use crate::notifications::{Notifier, NotifyLevel};

/// Per-adapter watchdog state. `suspended` short-circuits future ticks;
/// `stale_notified` dedups the events-stale warning so it fires once per
/// stale episode, not every hour.
#[derive(Default)]
pub struct AdapterWatch { pub suspended: Option<String>, pub stale_notified: bool }

pub type BreakerMap = Mutex<HashMap<String, AdapterWatch>>;

/// The config-side checks whose failure justifies an auto-reinstall.
fn config_side_failing(h: &AdapterHealth) -> bool {
    h.checks.iter().any(|c| c.status == CheckStatus::Fail
        && matches!(c.id.as_str(), "marketplace" | "plugin" | "enabled" | "hooks"))
}

fn events_failing(h: &AdapterHealth) -> bool {
    h.checks.iter().any(|c| c.id == "events" && c.status == CheckStatus::Fail)
}

/// What a tick did — returned so the async caller (`run_sweep`) can write the
/// DB audit trail without `tick_adapter` itself needing to be async.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome { Skipped, Healthy, StaleWarned, StaleAlreadyNotified, Resolved, Suspended }

/// Decide + act for one adapter. `resolve_fn` runs the reinstall and returns
/// whether it succeeded; `recheck_fn` recomputes health afterward. Sync + pure
/// of IO except via the injected closures + notifier, so it's fully
/// unit-testable; all `.await` logging happens in the caller off the return.
pub fn tick_adapter(
    health: &AdapterHealth,
    watch: &mut AdapterWatch,
    notifier: &dyn Notifier,
    resolve_fn: &dyn Fn() -> bool,
    recheck_fn: &dyn Fn() -> AdapterHealth,
) -> TickOutcome {
    if watch.suspended.is_some() { return TickOutcome::Skipped; }

    if config_side_failing(health) {
        let _ran = resolve_fn();
        let after = recheck_fn();
        if config_side_failing(&after) {
            let reason = "auto-repair failed; manual action needed".to_string();
            notifier.notify(NotifyLevel::Critical,
                &format!("{} capture broken", health.family),
                &format!("Config checks still failing after reinstall. Run `sensei doctor --fix`. ({reason})"));
            watch.suspended = Some(reason);
            return TickOutcome::Suspended;
        } else {
            notifier.notify(NotifyLevel::Info,
                &format!("{} capture auto-resolved", health.family),
                "A config check failed and was repaired by reinstalling the plugin.");
            watch.stale_notified = false;
            return TickOutcome::Resolved;
        }
    }

    if events_failing(health) {
        if !watch.stale_notified {
            notifier.notify(NotifyLevel::Warn,
                &format!("{} capture stale", health.family),
                "No hook events within the inactivity window. Config looks fine — a session restart may be needed.");
            watch.stale_notified = true;
            return TickOutcome::StaleWarned;
        }
        return TickOutcome::StaleAlreadyNotified;
    }
    watch.stale_notified = false;
    TickOutcome::Healthy
}
```

Add tests in the `mod tests` block (extend the existing one). Reference the `RecordingNotifier` by making it accessible: in `notifications.rs`, move `RecordingNotifier` out of `#[cfg(test)]` is undesirable — instead define a local one here:

```rust
    use crate::notifications::{Notifier, NotifyLevel};
    use std::sync::Mutex;

    struct Rec(Mutex<Vec<(NotifyLevel, String)>>);
    impl Notifier for Rec {
        fn notify(&self, l: NotifyLevel, t: &str, _b: &str) { self.0.lock().unwrap().push((l, t.into())); }
    }
    fn health(checks: Vec<(&str, CheckStatus)>) -> AdapterHealth {
        let cs = checks.into_iter().map(|(id, s)| AdapterCheck::new(id, id, s, None)).collect();
        AdapterHealth::new("claude-code", "claude", cs, true)
    }

    #[test]
    fn config_fail_then_resolve_ok_notifies_info_and_stays_active() {
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch::default();
        let bad = health(vec![("plugin", CheckStatus::Fail), ("events", CheckStatus::Ok)]);
        let good = health(vec![("plugin", CheckStatus::Ok), ("events", CheckStatus::Ok)]);
        tick_adapter(&bad, &mut w, &rec, &|| true, &|| good.clone());
        assert!(w.suspended.is_none());
        assert_eq!(rec.0.lock().unwrap()[0].0, NotifyLevel::Info);
    }

    #[test]
    fn config_fail_then_still_fail_notifies_critical_and_suspends() {
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch::default();
        let bad = health(vec![("plugin", CheckStatus::Fail)]);
        tick_adapter(&bad, &mut w, &rec, &|| true, &|| bad.clone());
        assert!(w.suspended.is_some());
        assert_eq!(rec.0.lock().unwrap()[0].0, NotifyLevel::Critical);
    }

    #[test]
    fn suspended_adapter_is_skipped() {
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch { suspended: Some("x".into()), stale_notified: false };
        let bad = health(vec![("plugin", CheckStatus::Fail)]);
        tick_adapter(&bad, &mut w, &rec, &|| panic!("must not resolve"), &|| bad.clone());
        assert!(rec.0.lock().unwrap().is_empty());
    }

    #[test]
    fn pure_events_stale_warns_once_never_resolves() {
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch::default();
        let stale = health(vec![("plugin", CheckStatus::Ok), ("events", CheckStatus::Fail)]);
        tick_adapter(&stale, &mut w, &rec, &|| panic!("must not resolve"), &|| stale.clone());
        tick_adapter(&stale, &mut w, &rec, &|| panic!("must not resolve"), &|| stale.clone());
        let calls = rec.0.lock().unwrap();
        assert_eq!(calls.len(), 1, "stale should warn once, not every tick");
        assert_eq!(calls[0].0, NotifyLevel::Warn);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p senseid assistants::watchdog::tests:: 2>&1 | tail -20`
Expected: the four `tick_adapter` tests + `parse_window` tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/assistants/watchdog.rs
git commit -m "feat(watchdog): tick policy + circuit breaker (config-fail→resolve→suspend)"
```

---

## Task 9: HTTP endpoints

**Files:**
- Modify: `crates/senseid/src/api/handlers/config.rs`
- Modify: `crates/senseid/src/api/routes.rs`

- [ ] **Step 1: Add handlers + spawn-loop hook (`run_sweep`)**

First add the sweep entry point to `watchdog.rs` (used by both the loop and the resolve endpoint to recheck). Append to `watchdog.rs`:

```rust
use std::sync::Arc;

/// Resolve a single adapter by id (re-run configure) and return the report.
/// Clears any breaker suspension for that adapter (explicit manual retry).
pub fn resolve_adapter(adapter_id: &str, breaker: &BreakerMap) -> crate::assistants::AdapterResolveReport {
    if let Ok(mut map) = breaker.lock() {
        map.entry(adapter_id.to_string()).or_default().suspended = None;
    }
    crate::assistants::resolve_by_id(adapter_id).unwrap_or_else(|| crate::assistants::AdapterResolveReport {
        adapter_id: adapter_id.to_string(), ok: false, actions: vec![],
        errors: vec![format!("unknown adapter {adapter_id}")],
    })
}

/// One full sweep over configured adapters: compute health, log each verdict
/// to the DB (via `logger` → public.logs), run the tick policy (auto-resolve
/// config-fails, notify, trip breaker). Used by the hourly loop. `breaker`
/// persists circuit-breaker state across ticks.
pub async fn run_sweep(
    pg: &PgStore,
    notifier: &Arc<dyn Notifier>,
    breaker: &Arc<BreakerMap>,
    logger: &sensei_logger::Logger,
    now_ms: i64,
) {
    let report = health_report(pg, now_ms).await;
    for h in report {
        // DB audit trail (the user-requested "log in db" → public.logs via the
        // daemon logger). Warn on any non-Ok verdict so it's queryable; info
        // otherwise. Logger methods are async, so all logging stays here in the
        // async body — `tick_adapter` is sync and returns what it did.
        let summary = format!("adapter {} status={:?} checks=[{}]", h.adapter_id, h.status,
            h.checks.iter().map(|c| format!("{}:{:?}", c.id, c.status)).collect::<Vec<_>>().join(","));
        if h.status == CheckStatus::Ok {
            logger.info(&summary, None).await;
        } else {
            logger.warn(&summary, None).await;
        }

        // Pull state out under the lock, run the (sync) policy, write back.
        let mut watch = {
            let mut map = breaker.lock().unwrap();
            std::mem::take(map.entry(h.adapter_id.clone()).or_default())
        };
        let id = h.adapter_id.clone();
        let recheck_health = AdapterHealth::new(&h.adapter_id, &h.family, config_health_for(&id), true);
        let outcome = tick_adapter(
            &h, &mut watch, notifier.as_ref(),
            &|| crate::assistants::resolve_by_id(&id).map(|r| r.ok).unwrap_or(false),
            &|| recheck_health.clone(),
        );
        breaker.lock().unwrap().insert(h.adapter_id.clone(), watch);

        // Log what the policy decided (resolution / suspension is the important
        // audit signal — it means the daemon mutated the user's config or gave up).
        match outcome {
            TickOutcome::Resolved  => logger.warn(&format!("watchdog auto-resolved {}", h.adapter_id), None).await,
            TickOutcome::Suspended => logger.error(
                &format!("watchdog SUSPENDED {} — auto-repair failed, manual action needed", h.adapter_id),
                None, None).await,
            _ => {}
        }
    }
}
```

In `assistants/mod.rs`, add a resolve-by-id registry helper (resolves `mcp_cmd` like `configure` does):

```rust
/// Resolve a single adapter by id via its `resolve()` (reinstall). None if the
/// id isn't registered or the mcp binary can't be found.
pub fn resolve_by_id(adapter_id: &str) -> Option<crate::assistants::AdapterResolveReport> {
    let mcp_cmd = find_mcp_binary()?.to_string_lossy().to_string();
    all_assistants().iter().find(|a| a.id() == adapter_id).map(|a| a.resolve(&mcp_cmd))
}
```
Re-export `resolve_by_id` and `AdapterResolveReport` is already exported (Task 1). Also export `run_sweep, resolve_adapter, BreakerMap` from mod.rs: `pub use watchdog::{health_report, run_sweep, resolve_adapter, BreakerMap, CaptureWindow};`

Add handlers in `config.rs` (top: confirm `use crate::api::state::AppState;` exists — it does):

```rust
/// GET /api/assistants/health — current per-adapter health (config + freshness).
pub(crate) async fn assistants_health(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let report = crate::assistants::health_report(&state.pg, now_ms).await;
    let overall = report.iter().map(|h| h.status)
        .fold(crate::assistants::CheckStatus::Ok, |acc, s| acc.worse(s));
    Json(serde_json::json!({ "status": overall, "adapters": report }))
}

#[derive(serde::Deserialize)]
pub(crate) struct ResolveBody { pub adapter_id: String }

/// POST /api/assistants/resolve — reinstall one adapter, clear its breaker.
pub(crate) async fn assistants_resolve(
    State(state): State<AppState>,
    Json(body): Json<ResolveBody>,
) -> Json<crate::assistants::AdapterResolveReport> {
    let report = crate::assistants::resolve_adapter(&body.adapter_id, &state.breaker);
    Json(report)
}
```

> `state.breaker` is added in Task 10 (the `SharedState` field). If you implement endpoints before Task 10, the compile will fail on `state.breaker` — that's expected; Task 10 makes it compile.

In `routes.rs`, register next to the other assistants routes (after line 128):

```rust
        .route("/api/assistants/health", get(config::assistants_health))
        .route("/api/assistants/resolve", post(config::assistants_resolve))
```

- [ ] **Step 2: (compile gate)** — defer the full test run to Task 10 (needs `state.breaker`). For now:

Run: `cargo build -p senseid 2>&1 | tail -10`
Expected: fails ONLY on `state.breaker` not existing yet (resolved next task). If any *other* error, fix it now.

- [ ] **Step 3: Commit** (after Task 10 compiles — see Task 10 Step 4). Skip committing a non-compiling tree; this task's diff is committed together with Task 10.

---

## Task 10: Wire breaker into SharedState + spawn the hourly watchdog

**Files:**
- Modify: `crates/senseid/src/api/state.rs`
- Modify: `crates/senseid/src/api/server.rs`

- [ ] **Step 1: Add the breaker to SharedState**

Read `crates/senseid/src/api/state.rs`. Add a field to `SharedState`:

```rust
    /// Per-adapter capture-watchdog circuit-breaker state (in-memory; resets on restart).
    pub breaker: std::sync::Arc<crate::assistants::BreakerMap>,
```

Update every `SharedState { ... }` constructor (in `server.rs` `build_full_app`, and the test harness in `routes.rs` `test_app()`) to include:

```rust
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
```

- [ ] **Step 2: Construct notifier + spawn the loop in `build_full_app`**

In `server.rs`, after the federation `run_pull_loop` line (~197), add:

```rust
    // Capture watchdog: hourly sweep over configured ACP adapters. Auto-resolves
    // config-side failures (reinstall), trips a per-adapter breaker on give-up,
    // and notifies the user (the only signal once an adapter is suspended).
    {
        let pg = state.pg.clone();
        let breaker = state.breaker.clone();
        let notifier: std::sync::Arc<dyn crate::notifications::Notifier> =
            std::sync::Arc::new(crate::notifications::DesktopNotifier);
        // DB audit logger (writes to public.logs), mirrors the task_logger above.
        let watchdog_logger = sensei_logger::Logger::new(
            sensei_logger::LogWriter::pg(state.pg.pool().clone()),
            sensei_logger::LogLevel::Info,
            "daemon",
            "watchdog",
        );
        tokio::spawn(async move {
            // Small initial delay so startup churn settles before the first sweep.
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            loop {
                let now_ms = chrono::Utc::now().timestamp_millis();
                crate::assistants::run_sweep(&pg, &notifier, &breaker, &watchdog_logger, now_ms).await;
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    }
```

- [ ] **Step 3: Run the daemon test suite**

Run: `cargo test -p senseid 2>&1 | tail -30`
Expected: full daemon suite compiles and PASSES, including the `/health` router test and the new endpoint handlers.

- [ ] **Step 4: Add an endpoint smoke test**

In `routes.rs` `mod tests`, add:

```rust
    #[tokio::test]
    async fn assistants_health_endpoint_returns_status_and_adapters() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().uri("/api/assistants/health").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["status"].is_string());
        assert!(json["adapters"].is_array());
    }
```

Run: `cargo test -p senseid assistants_health_endpoint 2>&1 | tail -15`
Expected: PASS.

- [ ] **Step 5: Commit (Tasks 9 + 10 together)**

```bash
git add crates/senseid/src/api/handlers/config.rs crates/senseid/src/api/routes.rs \
        crates/senseid/src/api/state.rs crates/senseid/src/api/server.rs \
        crates/senseid/src/assistants/mod.rs crates/senseid/src/assistants/watchdog.rs
git commit -m "feat(daemon): assistants health/resolve endpoints + hourly watchdog"
```

---

## Task 11: `sensei doctor` capture section + `--fix`

**Files:**
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/doctor.rs`

- [ ] **Step 1: Add the `--fix` flag**

In `crates/cli/src/main.rs`, change the `Doctor` variant (line ~94) to carry a flag, and the dispatch (line ~120):

```rust
    /// Diagnose bootstrap + capture health
    Doctor {
        /// Attempt to auto-resolve failing adapters (reinstall plugin/marketplace).
        #[arg(long)]
        fix: bool,
    },
```
```rust
        Commands::Doctor { fix } => return ExitCode::from(doctor::run(fix) as u8),
```

- [ ] **Step 2: Add the capture section to `doctor.rs`**

Change `pub fn run() -> i32` to `pub fn run(fix: bool) -> i32` and after `print_terminal(&terminal);` (line ~54) add a call to a new section. Append to `doctor.rs`:

```rust
use sensei_bootstrap::SenseiConfig;

/// Render the Capture / Assistants section by querying the daemon. Returns
/// extra exit-code pressure: 1 if any adapter is failing, else 0.
fn print_capture_section(fix: bool) -> i32 {
    let cfg = SenseiConfig::from_env();
    let base = format!("http://127.0.0.1:{}", cfg.daemon_port);
    println!();
    println!("{}", bold("Capture / Assistants"));

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("build http client");

    let health: serde_json::Value = match client.get(format!("{base}/api/assistants/health")).send() {
        Ok(r) => match r.json() { Ok(j) => j, Err(e) => { println!("  {} {}", red("✗"), dim(&format!("bad response: {e}"))); return 1; } },
        Err(_) => {
            println!("  {} {}", red("✗"), dim("daemon unreachable — cannot verify capture. Is senseid running? `sensei start`"));
            return 1;
        }
    };

    let adapters = health["adapters"].as_array().cloned().unwrap_or_default();
    if adapters.is_empty() {
        println!("  {} {}", dim("·"), dim("no configured assistants detected"));
        return 0;
    }

    let mut worst_fail = 0;
    let mut failing_ids: Vec<String> = vec![];
    for a in &adapters {
        let id = a["adapter_id"].as_str().unwrap_or("?");
        let astatus = a["status"].as_str().unwrap_or("unknown");
        let (icon, _) = status_glyph(astatus);
        println!("  {} {}", icon, bold(id));
        for c in a["checks"].as_array().cloned().unwrap_or_default() {
            let cid = c["id"].as_str().unwrap_or("?");
            let cstatus = c["status"].as_str().unwrap_or("unknown");
            let detail = c["detail"].as_str().unwrap_or("");
            let (gi, _) = status_glyph(cstatus);
            println!("    {} {:<12} {}", gi, cid, dim(detail));
        }
        if astatus == "fail" { worst_fail = 1; failing_ids.push(id.to_string()); }
    }

    if fix && !failing_ids.is_empty() {
        println!();
        for id in &failing_ids {
            println!("  {} resolving {}…", yellow("…"), id);
            let body = serde_json::json!({ "adapter_id": id });
            match client.post(format!("{base}/api/assistants/resolve")).json(&body).send()
                .and_then(|r| r.json::<serde_json::Value>())
            {
                Ok(rep) => {
                    let ok = rep["ok"].as_bool().unwrap_or(false);
                    if ok { println!("    {} resolved", green("✓")); }
                    else { println!("    {} failed: {}", red("✗"), dim(&rep["errors"].to_string())); }
                }
                Err(e) => println!("    {} request failed: {}", red("✗"), dim(&e.to_string())),
            }
        }
        println!("{}", dim("Re-run `sensei doctor` to confirm."));
    } else if worst_fail == 1 {
        println!();
        println!("{}", dim("Run `sensei doctor --fix` to auto-resolve, or restart your Claude session if only `events` is stale."));
    }

    worst_fail
}

/// Map a status string to a coloured glyph.
fn status_glyph(s: &str) -> (String, &'static str) {
    match s {
        "ok"   => (green("✓"), "ok"),
        "warn" => (yellow("⚠"), "warn"),
        "fail" => (red("✗"), "fail"),
        _       => (dim("·"), "unknown"),
    }
}
```

Wire it into `run`:

```rust
pub fn run(fix: bool) -> i32 {
    bootstrap::tracing_init::install_console("sensei_bootstrap=warn");
    // ... existing header + check_and_resolve + print_terminal ...
    let capture_pressure = print_capture_section(fix);
    if terminal.status == HealthStatus::Ok && capture_pressure == 0 { 0 } else { 1 }
}
```

- [ ] **Step 3: Build + manual smoke**

Run: `cargo build -p sensei-cli 2>&1 | tail -10`
Expected: builds.

Run (daemon must be running): `cargo run -p sensei-cli -- doctor 2>&1 | tail -30`
Expected: prints the existing bootstrap table, then a "Capture / Assistants" section listing `claude-code` with `marketplace/plugin/enabled/hooks/events` rows.

- [ ] **Step 4: Commit**

```bash
git add crates/cli/src/main.rs crates/cli/src/doctor.rs
git commit -m "feat(cli): sensei doctor capture/assistants section + --fix"
```

---

## Task 12: Full verification + integration smoke

**Files:** none (verification only)

- [ ] **Step 1: Zero-errors gate**

Run: `cargo test -p senseid -p sensei-bootstrap -p sensei-cli 2>&1 | tail -30`
Expected: all PASS.

Run: `cargo clippy -p senseid -p sensei-cli 2>&1 | tail -20`
Expected: no warnings introduced by the new code (fix any).

- [ ] **Step 2: Live end-to-end**

```bash
make install-debug         # overlay debug daemon binary
sensei restart             # pick up the new watchdog + endpoints
curl -s localhost:7744/api/assistants/health | python3 -m json.tool
```
Expected: JSON with `status` + an `adapters[]` entry for `claude-code` whose `checks` include `marketplace/plugin/enabled/hooks` (all `ok` right now) and `events` (`ok`, since capture is live this session).

Negative check (optional): temporarily set `enabledPlugins[sensei@sensei-marketplace]=false` in a COPY, point the probe at it, confirm `enabled` reports `fail`. Do NOT mutate the real settings.json.

- [ ] **Step 3: Final commit + push**

```bash
git push origin develop
```

- [ ] **Step 4: (on user confirmation) merge to main**

Per project rule "merge to main when a logical feature is complete." Confirm with the user, then:

```bash
git checkout main && git merge --no-ff develop && git push origin main && git checkout develop
```

---

## Self-review notes (already applied)

- **Spec coverage:** Layer 1 → Tasks 1,4. Layer 2 → Tasks 3,4. Layer 3 (freshness/DB/orchestrator) → Tasks 2,5,6. Endpoints → Task 9. Watchdog+breaker+notifications → Tasks 7,8,10. CLI → Task 11. Window config → Task 6. **DB logging** is first-class: `run_sweep` takes a `sensei_logger::Logger` (constructed in Task 10 with `LogWriter::pg`, module `"watchdog"`) and writes every verdict + resolution/suspension to `public.logs` — not via `tracing` (which only reaches stdout/file). Logger methods are async, so `tick_adapter` stays sync and returns `TickOutcome`; `run_sweep` does the `.await` logging.
- **Type consistency:** `AdapterHealth`/`AdapterCheck`/`CheckStatus`/`AdapterResolveReport` names are identical across Tasks 1,4,6,8,9. `config_health()`, `resolve(mcp_cmd)`, `resolve_by_id`, `resolve_adapter`, `run_sweep`, `health_report`, `tick_adapter`, `parse_window`, `capture_freshness`, `business_elapsed_hours`, `latest_hook_event_ts` — each defined once and referenced consistently.
- **Silent-error audit** (third deliverable) is intentionally NOT in this plan; it runs separately and starts with `sessions.rs:173`.
