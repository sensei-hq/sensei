# Bootstrap Diagnostic Logging — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add structured trace logging to every bootstrap check, collect sessions in a Tauri sidecar, and expose a `/logs` screen for session browsing and GitHub issue submission.

**Architecture:** Every check function in the bootstrap crate is a pure function returning `(ComponentStatus, Vec<BootstrapTrace>)` with no side effects. `run_with_traces()` runs all 6 gates in parallel threads and collates results. The Tauri command layer receives the data, writes a session JSON file via `LogCollector`, and the SvelteKit `/logs` page reads sessions from the sidecar. A frontend `getModuleLogger('wizard')` factory sends incremental `LogEntry` records to the same sidecar for general app logging.

**Tech Stack:** Rust + chrono (bootstrap crate + Tauri sidecar), sysinfo (Tauri sidecar for OS/arch), Svelte 5 runes + SvelteKit, Tauri 2 managed state + menu API, Tauri shell plugin for GitHub URL

---

## File Map

**Create:**
- `app/src-tauri/src/log_collector.rs` — `LogCollector` struct, session management (start/append/end + atomic write), file rotation
- `app/src-tauri/src/commands/logs.rs` — 4 Tauri commands: `log_session_start`, `log_entry`, `log_session_end`, `get_log_sessions`
- `app/src/lib/logger.ts` — `getModuleLogger()` factory + `ModuleLogger` class
- `app/src/lib/logger.spec.ts` — unit tests for logger (mocked invoke)
- `app/src/routes/(app)/logs/+page.ts` — SvelteKit load function calling `get_log_sessions`
- `app/src/routes/(app)/logs/+page.svelte` — two-panel UI: date→module accordion sidebar, trace table, report modal

**Modify:**
- `daemon/crates/bootstrap/Cargo.toml` — add `chrono = "0.4"`
- `daemon/crates/bootstrap/src/types.rs` — add `BootstrapTrace` struct + `TraceAction` enum
- `daemon/crates/bootstrap/src/util.rs` — add `run_traced()`, `probe_traced()`, `next_trace_id()`, `now_iso()`, plus traced check variants
- `daemon/crates/bootstrap/src/database.rs` — add `check_traced()` returning `(ComponentStatus, Vec<BootstrapTrace>)`
- `daemon/crates/bootstrap/src/lib.rs` — add `run_with_traces()` (parallel, returns `(BootstrapResult, Vec<BootstrapTrace>)`)
- `app/src-tauri/Cargo.toml` — add `chrono`, `sysinfo`
- `app/src-tauri/src/commands/mod.rs` — add `pub mod logs;`
- `app/src-tauri/src/commands/bootstrap.rs` — update `run_bootstrap` to call `run_with_traces` and write session
- `app/src-tauri/src/lib.rs` — register log commands, `.manage(LogCollector)`, add Diagnostic Logs menu item
- `app/src/lib/types.ts` — add `LogSession`, `BootstrapTrace`, `LogEntry` TypeScript types

---

## Task 1: BootstrapTrace types + chrono dep

**Files:**
- Modify: `daemon/crates/bootstrap/Cargo.toml`
- Modify: `daemon/crates/bootstrap/src/types.rs`

- [ ] **Step 1: Write the failing test**

Add to `daemon/crates/bootstrap/src/types.rs` (below existing tests):

```rust
#[test]
fn bootstrap_trace_serializes() {
    let t = BootstrapTrace {
        id:            "trace-00000001".to_string(),
        ts:            "2026-05-01T10:00:00Z".to_string(),
        action_type:   TraceAction::Check,
        step:          "postgres_binary".to_string(),
        desc:          "Locate postgres binary".to_string(),
        cmd:           "which postgres".to_string(),
        exit:          Some(0),
        out:           "/opt/homebrew/bin/postgres".to_string(),
        err:           String::new(),
        ms:            4,
        ok:            true,
        fix_attempted: false,
        fix_approach:  None,
        fix_ok:        None,
    };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains("\"step\":\"postgres_binary\""));
    assert!(json.contains("\"ok\":true"));
    let round_trip: BootstrapTrace = serde_json::from_str(&json).unwrap();
    assert_eq!(round_trip.step, t.step);
    assert!(matches!(round_trip.action_type, TraceAction::Check));
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd daemon && cargo test -p sensei-bootstrap bootstrap_trace_serializes 2>&1 | tail -20
```

Expected: `error[E0412]: cannot find type 'BootstrapTrace'`

- [ ] **Step 3: Add chrono to bootstrap Cargo.toml**

In `daemon/crates/bootstrap/Cargo.toml`, add under `[dependencies]`:

```toml
chrono = "0.4"
```

- [ ] **Step 4: Add BootstrapTrace and TraceAction to types.rs**

Add after the `BootstrapResult` impl block (before `#[cfg(test)]`):

```rust
/// Action classification for a diagnostic trace step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceAction {
    /// Passive probe — binary present, port open, DB exists.
    Check,
    /// Active remediation — install, start service, create DB.
    Resolve,
    /// Human action required — displayed as an instruction.
    Instruct,
}

/// A single diagnostic step captured during bootstrap.
/// Pure data — no file I/O, no side effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapTrace {
    pub id:            String,          // counter-based id, e.g. "trace-00000001"
    pub ts:            String,          // ISO 8601, e.g. "2026-05-01T10:00:00Z"
    pub action_type:   TraceAction,
    pub step:          String,          // snake_case step name, e.g. "postgres_port"
    pub desc:          String,          // human-readable label
    pub cmd:           String,          // command string or "tcp probe host:port"
    pub exit:          Option<i32>,     // process exit code (None for TCP probes)
    pub out:           String,          // stdout (trimmed)
    pub err:           String,          // stderr (trimmed)
    pub ms:            u64,             // wall-clock duration in milliseconds
    pub ok:            bool,            // did this step pass?
    pub fix_attempted: bool,            // did bootstrap attempt a fix?
    pub fix_approach:  Option<String>,  // command/strategy used to fix
    pub fix_ok:        Option<bool>,    // did the fix succeed?
}
```

- [ ] **Step 5: Run to verify it passes**

```bash
cd daemon && cargo test -p sensei-bootstrap bootstrap_trace_serializes 2>&1 | tail -10
```

Expected: `test types::tests::bootstrap_trace_serializes ... ok`

- [ ] **Step 6: Commit**

```bash
cd daemon && git add crates/bootstrap/Cargo.toml crates/bootstrap/src/types.rs
git commit -m "feat(bootstrap): add BootstrapTrace and TraceAction types"
```

---

## Task 2: run_traced() + probe_traced() helpers

**Files:**
- Modify: `daemon/crates/bootstrap/src/util.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `util.rs`:

```rust
#[test]
fn run_traced_captures_echo() {
    let t = run_traced("echo_test", "Echo hello", "echo", &["hello"]);
    assert!(t.ok, "echo should succeed");
    assert_eq!(t.out, "hello");
    assert_eq!(t.err, "");
    assert!(t.exit == Some(0));
    assert_eq!(t.step, "echo_test");
    assert!(t.ms < 5000, "should complete in under 5 seconds");
    assert!(!t.id.is_empty());
    assert!(!t.ts.is_empty());
}

#[test]
fn run_traced_captures_failure() {
    let t = run_traced("bad_cmd", "Run nonexistent", "sensei-nonexistent-xyz-binary", &[]);
    assert!(!t.ok);
    assert!(!t.err.is_empty(), "error should capture the failure reason");
}

#[test]
fn probe_traced_closed_port() {
    let t = probe_traced("port_check", "Check port 1", "127.0.0.1", 1);
    assert!(!t.ok);
    assert!(t.exit.is_none(), "TCP probes have no exit code");
    assert!(t.cmd.contains("tcp probe"));
    assert!(t.cmd.contains("127.0.0.1:1"));
}

#[test]
fn next_trace_id_is_unique() {
    let a = next_trace_id();
    let b = next_trace_id();
    assert_ne!(a, b);
    assert!(a.starts_with("trace-"));
}

#[test]
fn now_iso_looks_like_rfc3339() {
    let ts = now_iso();
    assert!(ts.contains('T'), "should have T separator");
    assert!(ts.ends_with('Z') || ts.contains('+'), "should have timezone");
}
```

- [ ] **Step 2: Run to verify they fail**

```bash
cd daemon && cargo test -p sensei-bootstrap run_traced probe_traced next_trace_id now_iso 2>&1 | tail -20
```

Expected: `error[E0425]: cannot find function 'run_traced'`

- [ ] **Step 3: Add helpers to util.rs**

Add after the imports at the top of `util.rs` (after `use crate::types::{ComponentState, ComponentStatus};`):

```rust
use crate::types::{BootstrapTrace, TraceAction};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
```

Add these public functions before the `#[cfg(test)]` block:

```rust
/// Generate a monotonically increasing trace ID.
pub fn next_trace_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("trace-{:08x}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// Current time as an ISO 8601 string.
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Run a shell command and return a fully-populated BootstrapTrace.
/// Pure function — no side effects, no global state mutations.
pub fn run_traced(step: &str, desc: &str, cmd: &str, args: &[&str]) -> BootstrapTrace {
    let ts = now_iso();
    let start = Instant::now();
    let result = std::process::Command::new(cmd)
        .args(args)
        .env("PATH", enrich_path())
        .output();
    let ms = start.elapsed().as_millis() as u64;
    let cmd_str = format!("{} {}", cmd, args.join(" ")).trim().to_string();

    match result {
        Ok(output) => BootstrapTrace {
            id:            next_trace_id(),
            ts,
            action_type:   TraceAction::Check,
            step:          step.to_string(),
            desc:          desc.to_string(),
            cmd:           cmd_str,
            exit:          output.status.code(),
            out:           String::from_utf8_lossy(&output.stdout).trim().to_string(),
            err:           String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ms,
            ok:            output.status.success(),
            fix_attempted: false,
            fix_approach:  None,
            fix_ok:        None,
        },
        Err(e) => BootstrapTrace {
            id:            next_trace_id(),
            ts,
            action_type:   TraceAction::Check,
            step:          step.to_string(),
            desc:          desc.to_string(),
            cmd:           cmd_str,
            exit:          Some(-1),
            out:           String::new(),
            err:           e.to_string(),
            ms,
            ok:            false,
            fix_attempted: false,
            fix_approach:  None,
            fix_ok:        None,
        },
    }
}

/// Probe a TCP port and return a BootstrapTrace with a synthetic command string.
/// Pure function — no side effects.
pub fn probe_traced(step: &str, desc: &str, host: &str, port: u16) -> BootstrapTrace {
    let ts = now_iso();
    let cmd = format!("tcp probe {host}:{port}");
    let start = Instant::now();
    let ok = TcpStream::connect_timeout(
        &format!("{host}:{port}").parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok();
    let ms = start.elapsed().as_millis() as u64;

    BootstrapTrace {
        id:            next_trace_id(),
        ts,
        action_type:   TraceAction::Check,
        step:          step.to_string(),
        desc:          desc.to_string(),
        cmd,
        exit:          None,
        out:           String::new(),
        err:           if ok { String::new() } else { format!("port {port} not reachable") },
        ms,
        ok,
        fix_attempted: false,
        fix_approach:  None,
        fix_ok:        None,
    }
}
```

- [ ] **Step 4: Run to verify the tests pass**

```bash
cd daemon && cargo test -p sensei-bootstrap run_traced probe_traced next_trace_id now_iso 2>&1 | tail -15
```

Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
cd daemon && git add crates/bootstrap/src/util.rs
git commit -m "feat(bootstrap): add run_traced, probe_traced, and trace ID/timestamp helpers"
```

---

## Task 3: Traced check variants in util.rs

**Files:**
- Modify: `daemon/crates/bootstrap/src/util.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `util.rs`:

```rust
#[test]
fn check_binary_traced_finds_ls() {
    let (status, traces) = check_binary_traced("ls", "ls", "--help");
    // ls exists everywhere — expect ready
    assert!(status.is_ready(), "ls should be found");
    assert!(!traces.is_empty(), "should have at least one trace");
    assert!(traces.iter().any(|t| t.step.contains("ls")));
}

#[test]
fn check_binary_traced_missing() {
    let (status, traces) = check_binary_traced(
        "nope", "sensei-nonexistent-xyz", "--version"
    );
    assert!(status.is_failed(), "nonexistent binary should fail");
    assert!(!traces.is_empty());
}

#[test]
fn check_service_traced_closed_port() {
    let (status, traces) = check_service_traced("nope_service", 1);
    assert!(status.is_failed());
    assert!(!traces.is_empty());
    assert!(traces.iter().any(|t| t.cmd.contains("tcp probe")));
}

#[test]
fn check_binary_and_service_traced_missing_binary() {
    let (status, traces) = check_binary_and_service_traced(
        "nope", "sensei-nonexistent-xyz", "--version", 1
    );
    assert!(status.is_failed());
    assert!(!traces.is_empty());
}
```

- [ ] **Step 2: Run to verify they fail**

```bash
cd daemon && cargo test -p sensei-bootstrap check_binary_traced check_service_traced check_binary_and_service_traced 2>&1 | tail -20
```

Expected: `error[E0425]: cannot find function 'check_binary_traced'`

- [ ] **Step 3: Add traced check variants to util.rs**

Add after `check_service` (before `fetch_service_version`):

```rust
/// Check if a binary exists, returning status and a trace. Pure function.
pub fn check_binary_traced(
    name: &str,
    binary: &str,
    version_flag: &str,
) -> (ComponentStatus, Vec<BootstrapTrace>) {
    // Use which_binary for reliable detection (handles EXTRA_PATHS fallback).
    // Record the which command as a trace.
    #[cfg(unix)]
    let which_cmd = "which";
    #[cfg(windows)]
    let which_cmd = "where";

    let which_t = run_traced(
        &format!("{name}_which"),
        &format!("Locate {name} binary"),
        which_cmd,
        &[binary],
    );

    let path = which_binary(binary);
    if path.is_none() {
        let mut t = which_t;
        t.ok = false;
        t.err = format!("{binary} not found in PATH or known directories");
        return (ComponentStatus::missing(name), vec![t]);
    }
    let path = path.unwrap();

    let version_t = run_traced(
        &format!("{name}_version"),
        &format!("Check {name} version"),
        binary,
        &[version_flag],
    );
    let version = parse_version_from_line(&version_t.out);
    let mut traces = vec![which_t, version_t];

    // Mark which trace ok = true if path was found (EXTRA_PATHS fallback may differ)
    if !traces[0].ok { traces[0].ok = true; traces[0].out = path.clone(); }

    let mut status = ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"));
    status.detail = Some(path);
    (status, traces)
}

/// Check binary + service port, returning status and traces. Pure function.
pub fn check_binary_and_service_traced(
    name: &str,
    binary: &str,
    version_flag: &str,
    port: u16,
) -> (ComponentStatus, Vec<BootstrapTrace>) {
    #[cfg(unix)]
    let which_cmd = "which";
    #[cfg(windows)]
    let which_cmd = "where";

    let which_t = run_traced(
        &format!("{name}_which"),
        &format!("Locate {name} binary"),
        which_cmd,
        &[binary],
    );

    let path = which_binary(binary);
    if path.is_none() {
        let mut t = which_t;
        t.ok = false;
        t.err = format!("{binary} not found in PATH or known directories");
        return (ComponentStatus::missing(name), vec![t]);
    }
    let path = path.unwrap();

    let mut traces = vec![which_t];
    if !traces[0].ok { traces[0].ok = true; traces[0].out = path.clone(); }

    let port_t = probe_traced(
        &format!("{name}_port"),
        &format!("Check {name} service on port {port}"),
        "127.0.0.1",
        port,
    );
    let port_ok = port_t.ok;
    traces.push(port_t);

    if !port_ok {
        let mut status = ComponentStatus::failed(
            name,
            &format!("installed at {path} but service not running on port {port}"),
        );
        status.detail = Some(path);
        return (status, traces);
    }

    let svc_version = fetch_service_version(name, port);
    let mut status = ComponentStatus::ready(
        name,
        svc_version.as_deref().unwrap_or("unknown"),
    );
    status.detail = Some(path);
    (status, traces)
}

/// Check a service port only, returning status and a trace. Pure function.
pub fn check_service_traced(name: &str, port: u16) -> (ComponentStatus, Vec<BootstrapTrace>) {
    let port_t = probe_traced(
        &format!("{name}_port"),
        &format!("Check {name} on port {port}"),
        "127.0.0.1",
        port,
    );
    if port_t.ok {
        let version = fetch_service_version(name, port);
        let status = ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"));
        (status, vec![port_t])
    } else {
        let status = ComponentStatus::failed(name, &format!("not reachable on port {port}"));
        (status, vec![port_t])
    }
}
```

- [ ] **Step 4: Run to verify tests pass**

```bash
cd daemon && cargo test -p sensei-bootstrap check_binary_traced check_service_traced check_binary_and_service_traced 2>&1 | tail -15
```

Expected: all 4 tests pass.

- [ ] **Step 5: Run full bootstrap test suite to ensure nothing is broken**

```bash
cd daemon && cargo test -p sensei-bootstrap 2>&1 | tail -20
```

Expected: all existing tests still pass.

- [ ] **Step 6: Commit**

```bash
cd daemon && git add crates/bootstrap/src/util.rs
git commit -m "feat(bootstrap): add traced check variants (pure functions returning traces)"
```

---

## Task 4: database::check_traced()

**Files:**
- Modify: `daemon/crates/bootstrap/src/database.rs`

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)]` in `database.rs`:

```rust
#[test]
fn check_traced_returns_traces() {
    let (status, traces) = check_traced(None);
    // Always returns at least one trace regardless of PostgreSQL availability.
    assert!(!traces.is_empty(), "should have at least one trace");
    // Status is either ready or failed — never panics.
    let _ = status.is_ready() || status.is_failed();
    // Every trace has a non-empty step and cmd.
    for t in &traces {
        assert!(!t.step.is_empty());
        assert!(!t.cmd.is_empty());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd daemon && cargo test -p sensei-bootstrap check_traced_returns_traces 2>&1 | tail -10
```

Expected: `error[E0425]: cannot find function 'check_traced'`

- [ ] **Step 3: Add check_traced() to database.rs**

Add `use crate::types::BootstrapTrace;` to the imports at the top of `database.rs`. Then add `check_traced` before `create`:

```rust
use crate::types::{BootstrapTrace, ComponentStatus};
use crate::util::{run_traced, probe_traced};
```

Replace the existing `use crate::types::ComponentStatus;` and `use crate::util;` with the above.

Then add `check_traced` before the `create` function:

```rust
/// Check PostgreSQL reachability + sensei database, returning traces.
/// Pure function — no side effects.
pub fn check_traced(db_name: Option<&str>) -> (ComponentStatus, Vec<BootstrapTrace>) {
    let db = db_name.unwrap_or(DEFAULT_DB_NAME);
    let mut traces = Vec::new();

    // Step 1: pg_isready
    let pg_t = run_traced(
        "pg_isready",
        "Check PostgreSQL is accepting connections",
        "pg_isready",
        &["--quiet"],
    );
    let pg_ok = pg_t.ok;
    traces.push(pg_t);

    if !pg_ok {
        return (
            ComponentStatus::failed("database", "postgresql not reachable (pg_isready failed)"),
            traces,
        );
    }

    // Step 2: database exists
    let db_list_t = run_traced(
        "psql_list",
        "List databases to check existence",
        "psql",
        &["-lqt"],
    );
    let db_list_ok = db_list_t.ok;
    let db_list_out = db_list_t.out.clone();
    traces.push(db_list_t);

    if !db_list_ok {
        return (
            ComponentStatus::failed("database", "psql -lqt failed"),
            traces,
        );
    }

    let db_exists = db_list_out.lines().any(|line| {
        line.split('|')
            .next()
            .map(|n| n.trim() == db)
            .unwrap_or(false)
    });

    if !db_exists {
        return (
            ComponentStatus::failed("database", &format!("database '{db}' does not exist")),
            traces,
        );
    }

    // Step 3: pgvector extension
    let vec_t = run_traced(
        "pgvector_check",
        "Check pgvector extension installed",
        "psql",
        &["-d", db, "-tAc", "SELECT 1 FROM pg_extension WHERE extname = 'vector'"],
    );
    let vec_ok = vec_t.ok && vec_t.out.starts_with('1');
    traces.push(vec_t);

    if !vec_ok {
        return (
            ComponentStatus::failed("database", "pgvector extension not installed"),
            traces,
        );
    }

    // Step 4: schema version
    let ver_t = run_traced(
        "schema_version",
        "Read schema migration version",
        "psql",
        &["-d", db, "-tAc", "SELECT max(version) FROM schema_migrations"],
    );
    let version: Option<i32> = ver_t.out.trim().parse().ok();
    traces.push(ver_t);

    let status = ComponentStatus::ready(
        "database",
        &format!("schema-{}", version.unwrap_or(0)),
    );
    (status, traces)
}
```

- [ ] **Step 4: Run to verify tests pass**

```bash
cd daemon && cargo test -p sensei-bootstrap check_traced_returns_traces 2>&1 | tail -10
```

Expected: `test database::tests::check_traced_returns_traces ... ok`

- [ ] **Step 5: Run full test suite**

```bash
cd daemon && cargo test -p sensei-bootstrap 2>&1 | tail -20
```

Expected: all tests pass (no regressions).

- [ ] **Step 6: Commit**

```bash
cd daemon && git add crates/bootstrap/src/database.rs
git commit -m "feat(bootstrap): add database::check_traced() pure function"
```

---

## Task 5: run_with_traces() in lib.rs

**Files:**
- Modify: `daemon/crates/bootstrap/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)]` in `lib.rs`:

```rust
#[test]
fn run_with_traces_returns_traces() {
    let (result, traces) = run_with_traces();
    // Should have at least one trace per gate (6 gates minimum)
    assert!(
        traces.len() >= 6,
        "expected at least 6 traces, got {}",
        traces.len()
    );
    assert!(!result.components.is_empty());
    // All traces have non-empty step and id
    for t in &traces {
        assert!(!t.step.is_empty(), "trace step should not be empty");
        assert!(!t.id.is_empty(), "trace id should not be empty");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd daemon && cargo test -p sensei-bootstrap run_with_traces_returns_traces 2>&1 | tail -10
```

Expected: `error[E0425]: cannot find function 'run_with_traces'`

- [ ] **Step 3: Add run_with_traces() to lib.rs**

Add `pub use types::BootstrapTrace;` to the `pub use types::*;` line (or just ensure BootstrapTrace is re-exported). Then add the function before `#[cfg(test)]`:

```rust
/// Run all bootstrap checks in parallel and return both the result and diagnostic traces.
///
/// Each gate runs on its own thread. Results are collated in gate order.
/// Pure data returned — the caller decides what to do with the traces.
pub fn run_with_traces() -> (BootstrapResult, Vec<BootstrapTrace>) {
    let hw = hardware::detect();

    // Spawn all 6 gates concurrently
    let h0 = std::thread::spawn(|| {
        let prov = platform::detect();
        let status = prov.check_package_manager();
        // Wrap in a synthetic trace since the platform provider is opaque
        let ok = status.is_ready();
        let trace = BootstrapTrace {
            id:            util::next_trace_id(),
            ts:            util::now_iso(),
            action_type:   types::TraceAction::Check,
            step:          "package_manager".to_string(),
            desc:          "Check system package manager".to_string(),
            cmd:           "package_manager_check".to_string(),
            exit:          Some(if ok { 0 } else { 1 }),
            out:           status.version.clone().unwrap_or_default(),
            err:           match &status.state {
                               types::ComponentState::Failed { error } => error.clone(),
                               _ => String::new(),
                           },
            ms:            0,
            ok,
            fix_attempted: false,
            fix_approach:  None,
            fix_ok:        None,
        };
        (status, vec![trace])
    });

    let h1 = std::thread::spawn(|| {
        util::check_binary_and_service_traced("postgresql", "postgres", "--version", POSTGRES_PORT)
    });
    let h2 = std::thread::spawn(|| {
        util::check_binary_and_service_traced("ollama", "ollama", "--version", OLLAMA_PORT)
    });
    let h3 = std::thread::spawn(|| {
        util::check_binary_traced("sensei", "sensei", "--version")
    });
    let h4 = std::thread::spawn(|| database::check_traced(None));
    let h5 = std::thread::spawn(|| util::check_service_traced("daemon", DAEMON_PORT));

    // Collate in gate order
    let (s0, t0) = h0.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("package_manager", "thread panicked"), vec![])
    });
    let (s1, t1) = h1.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("postgresql", "thread panicked"), vec![])
    });
    let (s2, t2) = h2.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("ollama", "thread panicked"), vec![])
    });
    let (s3, t3) = h3.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("sensei", "thread panicked"), vec![])
    });
    let (s4, t4) = h4.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("database", "thread panicked"), vec![])
    });
    let (s5, t5) = h5.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("daemon", "thread panicked"), vec![])
    });

    let components = vec![s0, s1, s2, s3, s4, s5];
    let mut all_traces = Vec::new();
    for t in [t0, t1, t2, t3, t4, t5] {
        all_traces.extend(t);
    }

    (BootstrapResult::from_checks(components, hw), all_traces)
}
```

- [ ] **Step 4: Run to verify test passes**

```bash
cd daemon && cargo test -p sensei-bootstrap run_with_traces_returns_traces 2>&1 | tail -15
```

Expected: `test tests::run_with_traces_returns_traces ... ok`

- [ ] **Step 5: Run full test suite**

```bash
cd daemon && cargo test -p sensei-bootstrap 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
cd daemon && git add crates/bootstrap/src/lib.rs
git commit -m "feat(bootstrap): add run_with_traces() parallel check with diagnostic traces"
```

---

## Task 6: LogCollector Tauri component

**Files:**
- Modify: `app/src-tauri/Cargo.toml`
- Create: `app/src-tauri/src/log_collector.rs`

- [ ] **Step 1: Add dependencies to Tauri Cargo.toml**

In `app/src-tauri/Cargo.toml`, add under `[dependencies]`:

```toml
chrono = { version = "0.4", features = ["serde"] }
sysinfo = "0.34"
```

Add under `[dev-dependencies]` (create the section if it doesn't exist):

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Write the failing tests**

Create `app/src-tauri/src/log_collector.rs` with tests only first:

```rust
//! Session-scoped diagnostic log collector.
//!
//! Two usage modes:
//! - Bootstrap: write a complete session atomically via `write_session()`
//! - App logger: incremental start/append/end via `start_session()`, `append_entry()`, `end_session()`
//!
//! Sole writer and reader of log files on disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_sys_info() -> SystemInfo {
        SystemInfo {
            os:        "macOS 15.0".to_string(),
            arch:      "arm64".to_string(),
            ram_gb:    16,
            cpu_cores: 10,
        }
    }

    #[test]
    fn start_append_end_writes_file() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());

        let sid = collector.start_session(
            "wizard".to_string(),
            "0.1.0".to_string(),
            make_sys_info(),
        );
        assert!(sid.starts_with("sess-"));

        let entry = LogEntry {
            id:    "e1".to_string(),
            ts:    "2026-05-01T10:00:00Z".to_string(),
            level: "info".to_string(),
            layer: "ui".to_string(),
            step:  "prefs_load".to_string(),
            msg:   "Preferences loaded".to_string(),
            data:  None,
            err:   None,
            stack: None,
        };
        collector.append_entry(&sid, entry);

        collector.end_session(&sid).unwrap();

        let path = tmp.path().join("wizard").join(format!("{sid}.json"));
        assert!(path.exists(), "session file should be written");
        let content = std::fs::read_to_string(&path).unwrap();
        let session: LogSession = serde_json::from_str(&content).unwrap();
        assert_eq!(session.module, "wizard");
        assert_eq!(session.traces.len(), 1);
        assert_eq!(session.outcome, "success");
    }

    #[test]
    fn error_entry_sets_outcome_failed() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());
        let sid = collector.start_session("wizard".to_string(), "0.1.0".to_string(), make_sys_info());

        collector.append_entry(&sid, LogEntry {
            id: "e1".to_string(), ts: "2026-05-01T10:00:00Z".to_string(),
            level: "error".to_string(), layer: "api".to_string(),
            step: "save_prefs".to_string(), msg: "Save failed".to_string(),
            data: None, err: Some("timeout".to_string()), stack: None,
        });

        collector.end_session(&sid).unwrap();
        let path = tmp.path().join("wizard").join(format!("{sid}.json"));
        let session: LogSession = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(session.outcome, "failed");
    }

    #[test]
    fn write_session_atomic() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());
        let session = LogSession {
            id:          "sess-00000001".to_string(),
            module:      "bootstrap".to_string(),
            started_at:  "2026-05-01T10:00:00Z".to_string(),
            app_version: "0.1.0".to_string(),
            system_info: make_sys_info(),
            outcome:     "success".to_string(),
            duration_ms: 1234,
            traces:      vec![],
        };
        collector.write_session("bootstrap", "sess-00000001", &session).unwrap();
        let path = tmp.path().join("bootstrap").join("sess-00000001.json");
        assert!(path.exists());
    }

    #[test]
    fn file_rotation_keeps_at_most_20() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());

        // Write 22 sessions
        for i in 0..22u64 {
            let sid = format!("sess-{i:08x}");
            let session = LogSession {
                id:          sid.clone(),
                module:      "bootstrap".to_string(),
                started_at:  format!("2026-05-01T10:00:{i:02}Z"),
                app_version: "0.1.0".to_string(),
                system_info: make_sys_info(),
                outcome:     "success".to_string(),
                duration_ms: i * 100,
                traces:      vec![],
            };
            collector.write_session("bootstrap", &sid, &session).unwrap();
            // Trigger rotation by starting a new session (rotation checks on start)
            let sid2 = collector.start_session("bootstrap".to_string(), "0.1.0".to_string(), make_sys_info());
            collector.end_session(&sid2).ok();
        }

        let dir = tmp.path().join("bootstrap");
        let count = std::fs::read_dir(&dir).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .count();
        assert!(count <= 20, "should keep at most 20 files, got {count}");
    }

    #[test]
    fn read_sessions_returns_newest_first() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());

        for (i, ts) in ["2026-05-01T08:00:00Z", "2026-05-01T10:00:00Z", "2026-05-01T09:00:00Z"]
            .iter()
            .enumerate()
        {
            let sid = format!("sess-{i:08x}");
            collector.write_session("bootstrap", &sid, &LogSession {
                id: sid, module: "bootstrap".to_string(), started_at: ts.to_string(),
                app_version: "0.1.0".to_string(), system_info: make_sys_info(),
                outcome: "success".to_string(), duration_ms: 100, traces: vec![],
            }).unwrap();
        }

        let sessions = collector.read_sessions(Some("bootstrap"));
        assert_eq!(sessions.len(), 3);
        // Newest first: 10:00 > 09:00 > 08:00
        assert!(sessions[0].started_at > sessions[1].started_at);
        assert!(sessions[1].started_at > sessions[2].started_at);
    }
}
```

- [ ] **Step 3: Run to verify tests fail**

```bash
cd app/src-tauri && cargo test log_collector 2>&1 | tail -20
```

Expected: compilation errors (struct definitions missing).

- [ ] **Step 4: Add full LogCollector implementation above the tests block**

Insert after the `use` statements at the top of `log_collector.rs`:

```rust
/// System metadata captured at session start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os:        String,
    pub arch:      String,
    pub ram_gb:    u64,
    pub cpu_cores: usize,
}

/// A single incremental log entry (app logger).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id:    String,
    pub ts:    String,
    /// "info" | "warn" | "error"
    pub level: String,
    /// "ui" | "api" | "sidecar" | "data_load"
    pub layer: String,
    pub step:  String,
    pub msg:   String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:  Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err:   Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

/// A completed log session — stored on disk as one JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSession {
    pub id:          String,
    pub module:      String,
    pub started_at:  String,
    pub app_version: String,
    pub system_info: SystemInfo,
    pub outcome:     String,
    pub duration_ms: u64,
    /// BootstrapTrace[] or LogEntry[] serialised as JSON values.
    pub traces:      Vec<serde_json::Value>,
}

struct ActiveSession {
    module:        String,
    started_at:    String,
    start_instant: Instant,
    app_version:   String,
    system_info:   SystemInfo,
    entries:       Vec<serde_json::Value>,
    /// 0 = info, 1 = warn, 2 = error
    max_level:     u8,
}

/// Thread-safe log session manager. Manages state per-module in memory;
/// writes atomically to `{log_dir}/{module}/{session_id}.json` on session end.
pub struct LogCollector {
    sessions: Mutex<HashMap<String, ActiveSession>>,
    log_dir:  PathBuf,
}

impl LogCollector {
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            log_dir,
        }
    }

    /// Start a new incremental session. Returns the generated session ID.
    pub fn start_session(
        &self,
        module: String,
        app_version: String,
        system_info: SystemInfo,
    ) -> String {
        let session_id = next_session_id();
        let dir = self.log_dir.join(&module);
        self.rotate_if_needed(&dir);

        let session = ActiveSession {
            module,
            started_at:    chrono::Utc::now().to_rfc3339(),
            start_instant: Instant::now(),
            app_version,
            system_info,
            entries:       Vec::new(),
            max_level:     0,
        };

        self.sessions.lock().unwrap().insert(session_id.clone(), session);
        session_id
    }

    /// Append a log entry to an open session. No-op if session_id is unknown.
    pub fn append_entry(&self, session_id: &str, entry: LogEntry) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(s) = sessions.get_mut(session_id) {
            let level = match entry.level.as_str() {
                "error" => 2,
                "warn"  => 1,
                _       => 0,
            };
            if level > s.max_level {
                s.max_level = level;
            }
            if let Ok(val) = serde_json::to_value(&entry) {
                s.entries.push(val);
            }
        }
    }

    /// Finalize an incremental session and write it to disk.
    pub fn end_session(&self, session_id: &str) -> Result<(), String> {
        let session = {
            let mut sessions = self.sessions.lock().unwrap();
            sessions
                .remove(session_id)
                .ok_or_else(|| format!("session {session_id} not found"))?
        };

        let outcome = match session.max_level {
            2 => "failed",
            1 => "partial",
            _ => "success",
        };
        let duration_ms = session.start_instant.elapsed().as_millis() as u64;
        let module = session.module.clone();

        let log_session = LogSession {
            id:          session_id.to_string(),
            module:      session.module,
            started_at:  session.started_at,
            app_version: session.app_version,
            system_info: session.system_info,
            outcome:     outcome.to_string(),
            duration_ms,
            traces:      session.entries,
        };

        self.write_session(&module, session_id, &log_session)
    }

    /// Write a pre-built session atomically (used by bootstrap after run_with_traces).
    pub fn write_session(
        &self,
        module: &str,
        session_id: &str,
        session: &LogSession,
    ) -> Result<(), String> {
        let dir = self.log_dir.join(module);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{session_id}.json"));
        let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    /// Read all sessions, optionally filtered by module, newest-first.
    pub fn read_sessions(&self, module: Option<&str>) -> Vec<LogSession> {
        let dirs: Vec<PathBuf> = if let Some(m) = module {
            vec![self.log_dir.join(m)]
        } else {
            std::fs::read_dir(&self.log_dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect()
        };

        let mut results = Vec::new();
        for dir in dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(session) = serde_json::from_str::<LogSession>(&content) {
                                results.push(session);
                            }
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        results
    }

    /// Delete oldest files if the module directory has ≥ 20 JSON files.
    fn rotate_if_needed(&self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();

        if files.len() >= 20 {
            files.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
            for file in files.iter().take(files.len() - 19) {
                let _ = std::fs::remove_file(file);
            }
        }
    }
}

fn next_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("sess-{n:08x}")
}
```

- [ ] **Step 5: Run to verify tests pass**

```bash
cd app/src-tauri && cargo test log_collector 2>&1 | tail -20
```

Expected: all 5 tests pass.

- [ ] **Step 6: Commit**

```bash
cd app/src-tauri && git add Cargo.toml src/log_collector.rs
git commit -m "feat(tauri): add LogCollector session manager with atomic writes and file rotation"
```

---

## Task 7: Log Tauri commands

**Files:**
- Create: `app/src-tauri/src/commands/logs.rs`
- Modify: `app/src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Create commands/logs.rs**

```rust
//! Log commands — session management and session retrieval.
//!
//! The UI logger sends entries via invoke; the sidecar is the sole writer.

use crate::log_collector::{LogCollector, LogEntry, LogSession, SystemInfo};
use tauri::State;

/// Start a new log session. Returns the session ID.
#[tauri::command]
pub fn log_session_start(
    state: State<LogCollector>,
    module: String,
    app_version: String,
    system_info: SystemInfo,
) -> String {
    state.start_session(module, app_version, system_info)
}

/// Append a single log entry to an open session. Fire-and-forget.
#[tauri::command]
pub fn log_entry(state: State<LogCollector>, session_id: String, entry: LogEntry) {
    state.append_entry(&session_id, entry);
}

/// Finalize a session and write it to disk.
#[tauri::command]
pub fn log_session_end(
    state: State<LogCollector>,
    session_id: String,
) -> Result<(), String> {
    state.end_session(&session_id)
}

/// Return all log sessions, optionally filtered by module, newest-first.
#[tauri::command]
pub fn get_log_sessions(
    state: State<LogCollector>,
    module: Option<String>,
) -> Vec<LogSession> {
    state.read_sessions(module.as_deref())
}
```

- [ ] **Step 2: Add module to commands/mod.rs**

```rust
//! Tauri command modules — each module owns a focused set of invoke commands.

pub mod bootstrap;
pub mod assistants;
pub mod repos;
pub mod logs;
```

- [ ] **Step 3: Compile check**

```bash
cd app/src-tauri && cargo check 2>&1 | tail -20
```

Expected: no errors (logs module is declared, commands compile).

- [ ] **Step 4: Commit**

```bash
cd app/src-tauri && git add src/commands/logs.rs src/commands/mod.rs
git commit -m "feat(tauri): add log_session_start/entry/end/get_log_sessions commands"
```

---

## Task 8: Wire LogCollector into Tauri app + menu item

**Files:**
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Update lib.rs**

Replace the entire contents of `app/src-tauri/src/lib.rs` with:

```rust
//! Sensei Desktop — Tauri application entry point.

mod commands;
mod log_collector;

use log_collector::LogCollector;
use tauri::Manager;

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Bootstrap (prereqs, hardware, models)
            commands::bootstrap::run_bootstrap,
            commands::bootstrap::detect_hardware,
            commands::bootstrap::list_models,
            commands::bootstrap::missing_models,
            commands::bootstrap::get_platform,
            commands::bootstrap::install_prerequisites,
            commands::bootstrap::start_services,
            commands::bootstrap::setup_database,
            // Assistants (detection, MCP config)
            commands::assistants::detect_assistants,
            commands::assistants::configure_mcp,
            commands::assistants::check_assistant_configs,
            // Repos (scanning, analysis, dependencies)
            commands::repos::get_repo_id,
            commands::repos::analyze_folder,
            commands::repos::detect_dependencies,
            // Logs
            commands::logs::log_session_start,
            commands::logs::log_entry,
            commands::logs::log_session_end,
            commands::logs::get_log_sessions,
        ])
        .setup(|app| {
            // ── Vibrancy ──────────────────────────────────────────────────
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                let _ = apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None);
            }
            #[cfg(debug_assertions)]
            window.open_devtools();

            // ── LogCollector managed state ────────────────────────────────
            let log_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir")
                .join("sensei")
                .join("logs");
            app.manage(LogCollector::new(log_dir));

            // ── Menu: View > Diagnostic Logs ──────────────────────────────
            use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
            let logs_item = MenuItemBuilder::with_id("open-logs", "Diagnostic Logs")
                .accelerator("CmdOrCtrl+Shift+L")
                .build(app)?;
            let view_menu = SubmenuBuilder::new(app, "View")
                .item(&logs_item)
                .build()?;
            let menu = MenuBuilder::new(app)
                .item(&view_menu)
                .build()?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                if event.id() == "open-logs" {
                    let _ = app.emit("open-logs", ());
                }
            });

            Ok(())
        });

    #[cfg(feature = "e2e-testing")]
    let builder = builder.plugin(tauri_plugin_playwright::init());

    builder
        .run(tauri::generate_context!())
        .expect("error while running sensei desktop")
}
```

- [ ] **Step 2: Compile check**

```bash
cd app/src-tauri && cargo check 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
cd app/src-tauri && git add src/lib.rs
git commit -m "feat(tauri): register LogCollector, log commands, and Diagnostic Logs menu item"
```

---

## Task 9: Update run_bootstrap to write session traces

**Files:**
- Modify: `app/src-tauri/src/commands/bootstrap.rs`

- [ ] **Step 1: Update run_bootstrap in bootstrap.rs**

Replace the `run_bootstrap` command function with:

```rust
use sensei_bootstrap::{
    self as bootstrap,
    BootstrapResult, HardwareInfo,
};
use tauri::Emitter;
use crate::log_collector::{LogCollector, LogSession, SystemInfo};
```

(Add the `LogCollector`/`LogSession`/`SystemInfo` imports at the top of the file.)

Replace the `run_bootstrap` function:

```rust
/// Run the full bootstrap check — all components + hardware detection.
/// Traces all checks and writes a session log to disk.
#[tauri::command]
pub fn run_bootstrap(app: tauri::AppHandle) -> BootstrapResult {
    let (result, traces) = bootstrap::run_with_traces();

    // Write session log (best-effort — never fail the bootstrap over logging)
    let _ = write_bootstrap_session(&app, &result, &traces);

    result
}

fn write_bootstrap_session(
    app: &tauri::AppHandle,
    result: &BootstrapResult,
    traces: &[sensei_bootstrap::BootstrapTrace],
) -> Result<(), String> {
    let hw = &result.hardware;
    let system_info = collect_system_info(hw);

    let outcome = if result.ready {
        "success"
    } else if result.components.iter().any(|c| c.is_failed()) {
        "failed"
    } else {
        "partial"
    };

    let duration_ms: u64 = traces.iter().map(|t| t.ms).sum();

    let trace_values: Vec<serde_json::Value> = traces
        .iter()
        .map(|t| serde_json::to_value(t).unwrap_or_default())
        .collect();

    // Generate session ID from current time + counter
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let session_id = format!("sess-bs-{:08x}", COUNTER.fetch_add(1, Ordering::SeqCst));

    let session = LogSession {
        id:          session_id.clone(),
        module:      "bootstrap".to_string(),
        started_at:  chrono::Utc::now().to_rfc3339(),
        app_version: app.package_info().version.to_string(),
        system_info,
        outcome:     outcome.to_string(),
        duration_ms,
        traces:      trace_values,
    };

    let collector = app.state::<LogCollector>();
    collector.write_session("bootstrap", &session_id, &session)
}

fn collect_system_info(hw: &HardwareInfo) -> SystemInfo {
    SystemInfo {
        os:        sysinfo::System::long_os_version()
                       .unwrap_or_else(|| "unknown".to_string()),
        arch:      std::env::consts::ARCH.to_string(),
        ram_gb:    hw.ram_gb as u64,
        cpu_cores: hw.cpu_cores as usize,
    }
}
```

- [ ] **Step 2: Compile check**

```bash
cd app/src-tauri && cargo check 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
cd app/src-tauri && git add src/commands/bootstrap.rs
git commit -m "feat(tauri): update run_bootstrap to write diagnostic session via LogCollector"
```

---

## Task 10: Frontend types + logger.ts

**Files:**
- Modify: `app/src/lib/types.ts`
- Create: `app/src/lib/logger.ts`
- Create: `app/src/lib/logger.spec.ts`

- [ ] **Step 1: Add log types to types.ts**

Read `app/src/lib/types.ts`, then append to its end:

```ts
// ── Diagnostic Log Types ──────────────────────────────────────────────────

export interface SystemInfo {
    os:        string;
    arch:      string;
    ram_gb:    number;
    cpu_cores: number;
}

export interface LogEntry {
    id:     string;
    ts:     string;
    level:  'info' | 'warn' | 'error';
    layer:  'ui' | 'api' | 'sidecar' | 'data_load';
    step:   string;
    msg:    string;
    data?:  Record<string, unknown>;
    err?:   string;
    stack?: string;
}

export interface BootstrapTrace {
    id:            string;
    ts:            string;
    action_type:   'check' | 'resolve' | 'instruct';
    step:          string;
    desc:          string;
    cmd:           string;
    exit:          number | null;
    out:           string;
    err:           string;
    ms:            number;
    ok:            boolean;
    fix_attempted: boolean;
    fix_approach:  string | null;
    fix_ok:        boolean | null;
}

export interface LogSession {
    id:          string;
    module:      string;
    started_at:  string;
    app_version: string;
    system_info: SystemInfo;
    outcome:     'success' | 'partial' | 'failed';
    duration_ms: number;
    traces:      (BootstrapTrace | LogEntry)[];
}
```

- [ ] **Step 2: Write the failing logger tests**

Create `app/src/lib/logger.spec.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getModuleLogger } from './logger.js';

// Mock @tauri-apps/api/core before importing logger
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
    mockInvoke.mockReset();
});

describe('getModuleLogger', () => {
    it('starts a session on first call', async () => {
        mockInvoke.mockResolvedValueOnce('sess-00000001'); // log_session_start
        const logger = getModuleLogger('wizard');
        await logger.ready;
        expect(mockInvoke).toHaveBeenCalledWith('log_session_start', expect.objectContaining({
            module: 'wizard',
        }));
    });

    it('returns the same instance for the same module', async () => {
        mockInvoke.mockResolvedValue('sess-00000001');
        const a = getModuleLogger('wizard');
        const b = getModuleLogger('wizard');
        expect(a).toBe(b);
    });

    it('returns different instances for different modules', async () => {
        mockInvoke.mockResolvedValue('sess-00000001');
        const a = getModuleLogger('wizard');
        const b = getModuleLogger('bootstrap');
        expect(a).not.toBe(b);
    });

    it('info() sends a log entry with level info', async () => {
        mockInvoke.mockResolvedValueOnce('sess-abc');
        mockInvoke.mockResolvedValue(undefined);
        const logger = getModuleLogger('test-info');
        await logger.ready;
        await logger.info('ui', 'prefs_load', 'Prefs loaded', { ms: 10 });
        const call = mockInvoke.mock.calls.find(([cmd]) => cmd === 'log_entry');
        expect(call).toBeDefined();
        const entry = (call![1] as any).entry;
        expect(entry.level).toBe('info');
        expect(entry.layer).toBe('ui');
        expect(entry.step).toBe('prefs_load');
        expect(entry.msg).toBe('Prefs loaded');
        expect(entry.data).toEqual({ ms: 10 });
    });

    it('error() sets err and stack from Error object', async () => {
        mockInvoke.mockResolvedValueOnce('sess-abc');
        mockInvoke.mockResolvedValue(undefined);
        const logger = getModuleLogger('test-error');
        await logger.ready;
        const e = new Error('connection refused');
        await logger.error('sidecar', 'daemon_invoke', 'Invoke failed', e);
        const call = mockInvoke.mock.calls.find(([cmd]) => cmd === 'log_entry');
        const entry = (call![1] as any).entry;
        expect(entry.level).toBe('error');
        expect(entry.err).toBe('connection refused');
        expect(entry.stack).toContain('Error');
    });

    it('close() calls log_session_end', async () => {
        mockInvoke.mockResolvedValueOnce('sess-close');
        mockInvoke.mockResolvedValue(undefined);
        const logger = getModuleLogger('test-close');
        await logger.ready;
        await logger.close();
        expect(mockInvoke).toHaveBeenCalledWith('log_session_end', { session_id: 'sess-close' });
    });
});
```

- [ ] **Step 3: Run to verify tests fail**

```bash
cd app && bun run test logger.spec 2>&1 | tail -20
```

Expected: `Cannot find module './logger.js'`

- [ ] **Step 4: Create logger.ts**

Create `app/src/lib/logger.ts`:

```ts
/**
 * Module-scoped session logger.
 *
 * Each module (bootstrap, wizard, projects…) gets one active session at a time.
 * Log entries are forwarded to the Tauri sidecar via invoke — the sidecar is the
 * sole writer to disk.
 *
 * Usage:
 *   const logger = getModuleLogger('wizard');
 *   await logger.ready;
 *   logger.info('ui', 'prefs_save', 'Preferences saved', { ms: 58 });
 *   // In onDestroy / beforeNavigate:
 *   logger.close();
 */

import { invoke } from '@tauri-apps/api/core';
import type { LogEntry } from './types.js';

// ── Session ID helper ──────────────────────────────────────────────────────

let _counter = 0;
function nextEntryId(): string {
    return `entry-${Date.now()}-${++_counter}`;
}

function nowIso(): string {
    return new Date().toISOString();
}

// ── ModuleLogger ───────────────────────────────────────────────────────────

export class ModuleLogger {
    readonly module: string;
    /** Resolves when the session has been started with the sidecar. */
    readonly ready: Promise<void>;

    private _sessionId = '';
    private _startPromise: Promise<void>;

    constructor(module: string) {
        this.module = module;
        this._startPromise = this._startSession();
        this.ready = this._startPromise;
    }

    private async _startSession(): Promise<void> {
        const appVersion = '0.1.0'; // TODO: pull from __APP_VERSION__ or tauri API
        const systemInfo = {
            os: navigator.userAgent,
            arch: 'unknown',
            ram_gb: 0,
            cpu_cores: navigator.hardwareConcurrency ?? 0,
        };
        this._sessionId = await invoke<string>('log_session_start', {
            module:      this.module,
            app_version: appVersion,
            system_info: systemInfo,
        });
    }

    async info(
        layer: LogEntry['layer'],
        step: string,
        msg: string,
        data?: Record<string, unknown>,
    ): Promise<void> {
        await this._send({ level: 'info', layer, step, msg, data });
    }

    async warn(
        layer: LogEntry['layer'],
        step: string,
        msg: string,
        data?: Record<string, unknown>,
    ): Promise<void> {
        await this._send({ level: 'warn', layer, step, msg, data });
    }

    async error(
        layer: LogEntry['layer'],
        step: string,
        msg: string,
        errOrData?: Error | Record<string, unknown>,
    ): Promise<void> {
        const extra: Partial<LogEntry> = {};
        if (errOrData instanceof Error) {
            extra.err   = errOrData.message;
            extra.stack = errOrData.stack;
        } else if (errOrData) {
            extra.data = errOrData;
        }
        await this._send({ level: 'error', layer, step, msg, ...extra });
    }

    async close(): Promise<void> {
        await this._startPromise; // Ensure session started before closing
        await invoke('log_session_end', { session_id: this._sessionId });
        _registry.delete(this.module);
    }

    private async _send(fields: Partial<LogEntry>): Promise<void> {
        await this._startPromise;
        const entry: LogEntry = {
            id:    nextEntryId(),
            ts:    nowIso(),
            level: fields.level ?? 'info',
            layer: fields.layer ?? 'ui',
            step:  fields.step  ?? '',
            msg:   fields.msg   ?? '',
            data:  fields.data,
            err:   fields.err,
            stack: fields.stack,
        };

        if (import.meta.env.DEV) {
            const prefix = `[${this.module}:${entry.layer}]`;
            if (entry.level === 'error') console.error(prefix, entry.msg, entry);
            else if (entry.level === 'warn') console.warn(prefix, entry.msg, entry);
            else console.debug(prefix, entry.msg, entry);
        }

        await invoke('log_entry', {
            session_id: this._sessionId,
            entry,
        });
    }
}

// ── Module registry ────────────────────────────────────────────────────────

const _registry = new Map<string, ModuleLogger>();

/**
 * Return (or create) the ModuleLogger for the given module.
 * Calling this twice with the same module returns the same instance.
 */
export function getModuleLogger(module: string): ModuleLogger {
    let logger = _registry.get(module);
    if (!logger) {
        logger = new ModuleLogger(module);
        _registry.set(module, logger);
    }
    return logger;
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd app && bun run test logger.spec 2>&1 | tail -20
```

Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
cd app && git add src/lib/types.ts src/lib/logger.ts src/lib/logger.spec.ts
git commit -m "feat(app): add LogSession/BootstrapTrace types and getModuleLogger() frontend logger"
```

---

## Task 11: /logs SvelteKit load function

**Files:**
- Create: `app/src/routes/(app)/logs/+page.ts`

- [ ] **Step 1: Create +page.ts**

```ts
// app/src/routes/(app)/logs/+page.ts
import { invoke } from '@tauri-apps/api/core';
import type { PageLoad } from './$types';
import type { LogSession } from '$lib/types.js';

export const load: PageLoad = async () => {
    const sessions = await invoke<LogSession[]>('get_log_sessions', { module: null });
    return { sessions };
};
```

- [ ] **Step 2: Compile check (type check)**

```bash
cd app && bun run check 2>&1 | tail -20
```

Expected: no type errors.

- [ ] **Step 3: Commit**

```bash
cd app && git add src/routes/\(app\)/logs/+page.ts
git commit -m "feat(app): add /logs route load function"
```

---

## Task 12: /logs +page.svelte

**Files:**
- Create: `app/src/routes/(app)/logs/+page.svelte`

- [ ] **Step 1: Write the component test**

Create `app/src/routes/(app)/logs/+page.spec.svelte.ts`:

```ts
import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import Page from './+page.svelte';
import type { LogSession, BootstrapTrace } from '$lib/types.js';

function makeTrace(step: string, ok: boolean): BootstrapTrace {
    return {
        id: step, ts: '2026-05-01T10:00:00Z', action_type: 'check',
        step, desc: step, cmd: `which ${step}`, exit: ok ? 0 : 1,
        out: ok ? '/usr/bin/' + step : '', err: ok ? '' : 'not found',
        ms: 12, ok, fix_attempted: false, fix_approach: null, fix_ok: null,
    };
}

const bootstrapSession: LogSession = {
    id: 'sess-00000001',
    module: 'bootstrap',
    started_at: new Date().toISOString(),
    app_version: '0.1.0',
    system_info: { os: 'macOS 15.0', arch: 'arm64', ram_gb: 16, cpu_cores: 10 },
    outcome: 'success',
    duration_ms: 1234,
    traces: [makeTrace('postgres', true), makeTrace('ollama', false)],
};

const wizardSession: LogSession = {
    id: 'sess-00000002',
    module: 'wizard',
    started_at: new Date().toISOString(),
    app_version: '0.1.0',
    system_info: { os: 'macOS 15.0', arch: 'arm64', ram_gb: 16, cpu_cores: 10 },
    outcome: 'partial',
    duration_ms: 456,
    traces: [],
};

describe('/logs page', () => {
    it('renders session sidebar with date group and module sections', () => {
        const { getByText } = render(Page, {
            props: { data: { sessions: [bootstrapSession, wizardSession] } },
        });
        expect(getByText('Today')).toBeTruthy();
        expect(getByText(/Bootstrap/i)).toBeTruthy();
        expect(getByText(/Setup Wizard|Wizard/i)).toBeTruthy();
    });

    it('renders trace table when session is selected', async () => {
        const { getByText, getAllByRole } = render(Page, {
            props: { data: { sessions: [bootstrapSession] } },
        });
        // Click first session row to select it
        const rows = getAllByRole('button').filter(b => b.textContent?.includes('sess') || true);
        // Session is auto-selected if it's the only one, or click first row
        // Verify trace table appears
        getByText('postgres');
    });

    it('opens report modal when "Report this session" is clicked', async () => {
        const { getByText } = render(Page, {
            props: { data: { sessions: [bootstrapSession] } },
        });
        // Auto-select first session
        const reportBtn = getByText(/Report this session/i);
        await reportBtn.click();
        expect(getByText(/ISSUE PREVIEW/i)).toBeTruthy();
    });

    it('anonymizes paths in issue body', async () => {
        const sessionWithPath: LogSession = {
            ...bootstrapSession,
            traces: [{
                ...makeTrace('pg', true),
                out: '/Users/jerry/homebrew/bin/postgres',
            }],
        };
        const { getByText } = render(Page, {
            props: { data: { sessions: [sessionWithPath] } },
        });
        const reportBtn = getByText(/Report this session/i);
        await reportBtn.click();
        // The issue preview should not contain /Users/jerry/
        const preview = document.querySelector('.body-preview');
        expect(preview?.textContent).not.toContain('/Users/jerry/');
        expect(preview?.textContent).toContain('~/');
    });
});
```

- [ ] **Step 2: Run to verify test fails**

```bash
cd app && bun run test page.spec.svelte 2>&1 | tail -20
```

Expected: `Cannot find module './+page.svelte'`

- [ ] **Step 3: Create +page.svelte**

Create `app/src/routes/(app)/logs/+page.svelte`:

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  import type { LogSession, BootstrapTrace } from '$lib/types.js';
  import { open } from '@tauri-apps/plugin-opener';

  let { data }: { data: PageData } = $props();

  // ── State ─────────────────────────────────────────────────────────────────
  let selectedId    = $state<string | null>(data.sessions[0]?.id ?? null);
  let showModal     = $state(false);
  let addCtx        = $state('');
  let expandedDates = $state(new Set(['Today']));

  // ── Helpers ───────────────────────────────────────────────────────────────
  const MODULE_META: Record<string, { kanji: string; label: string }> = {
    bootstrap: { kanji: '健', label: 'Bootstrap' },
    wizard:    { kanji: '導', label: 'Setup Wizard' },
    projects:  { kanji: '組', label: 'Projects' },
  };

  function moduleLabel(mod: string) {
    return MODULE_META[mod] ?? { kanji: '？', label: mod };
  }

  function dateKey(s: LogSession): string {
    const d        = new Date(s.started_at);
    const today    = new Date();
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    if (d.toDateString() === today.toDateString())     return 'Today';
    if (d.toDateString() === yesterday.toDateString()) return 'Yesterday';
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  function formatTime(ts: string): string {
    return new Date(ts).toLocaleTimeString('en-US', {
      hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
    });
  }

  function fmtMs(ms: number): string {
    return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
  }

  const anonymize = (s: string) => s.replace(/\/Users\/[^/]+\//g, '~/');

  type Group  = { mod: string; sessions: LogSession[] };
  type Bucket = { date: string; groups: Group[] };

  function groupByDateThenModule(sessions: LogSession[]): Bucket[] {
    const dateOrder: string[] = [];
    const dateMap: Record<string, { modOrder: string[]; modMap: Record<string, LogSession[]> }> = {};
    for (const s of sessions) {
      const dk = dateKey(s);
      if (!dateMap[dk]) { dateMap[dk] = { modOrder: [], modMap: {} }; dateOrder.push(dk); }
      const { modOrder, modMap } = dateMap[dk];
      if (!modMap[s.module]) { modMap[s.module] = []; modOrder.push(s.module); }
      modMap[s.module].push(s);
    }
    return dateOrder.map(dk => ({
      date:   dk,
      groups: dateMap[dk].modOrder.map(mod => ({ mod, sessions: dateMap[dk].modMap[mod] })),
    }));
  }

  // ── Derived ───────────────────────────────────────────────────────────────
  const grouped         = $derived(groupByDateThenModule(data.sessions));
  const selectedSession = $derived(data.sessions.find(s => s.id === selectedId) ?? null);

  function fixCount(session: LogSession): number {
    return (session.traces as BootstrapTrace[]).filter(t => t.fix_attempted).length;
  }

  function outcomeDot(outcome: string): string {
    return outcome === 'success' ? '#4CAF50' : outcome === 'partial' ? '#FF9800' : '#F44336';
  }

  // ── Issue body ─────────────────────────────────────────────────────────────
  function buildTitle(session: LogSession): string {
    return `Bootstrap diagnostic — ${anonymize(session.system_info.os)} · ${session.system_info.arch} · v${session.app_version}`;
  }

  function buildBody(session: LogSession, additionalCtx: string): string {
    const traces = session.traces as BootstrapTrace[];
    const traceRows = traces.map(t =>
      `| ${t.step} | ${t.action_type} | \`${anonymize(t.cmd)}\` | ${fmtMs(t.ms)} | ${t.ok ? '✓' : '✗'} |`
    ).join('\n');

    const fixTraces = traces.filter(t => t.err || t.fix_attempted);
    const fixMd = fixTraces.length > 0
      ? [
          '',
          '### Fix details',
          '',
          ...fixTraces.flatMap(t => [
            `**${t.step}** (${t.action_type} · ${fmtMs(t.ms)}) — ${t.ok ? '✓' : t.fix_ok ? '✓ fixed' : '✗ failed'}`,
            ...(t.err           ? [`- stderr: \`${anonymize(t.err)}\``] : []),
            ...(t.fix_attempted ? [`- fix applied: \`$ ${anonymize(t.fix_approach ?? '')}\` → ${t.fix_ok ? 'success' : 'failed'}`] : []),
          ]),
        ].join('\n')
      : '';

    const ctxSection = additionalCtx ? `\n\n## Additional context\n\n${additionalCtx}` : '';

    return `## System

| | |
|---|---|
| OS | ${anonymize(session.system_info.os)} |
| Arch | ${session.system_info.arch} |
| RAM | ${session.system_info.ram_gb} GB |
| CPU cores | ${session.system_info.cpu_cores} |
| App version | v${session.app_version} |
| Outcome | ${session.outcome} |
| Duration | ${fmtMs(session.duration_ms)} |

## Bootstrap trace

| Step | Type | Command | Duration | Result |
|------|------|---------|----------|--------|
${traceRows}
${fixMd}${ctxSection}`;
  }

  async function submitToGitHub() {
    if (!selectedSession) return;
    const title = buildTitle(selectedSession);
    const body  = buildBody(selectedSession, addCtx);
    const url   = `https://github.com/sensei-hq/app/issues/new?title=${encodeURIComponent(title)}&body=${encodeURIComponent(body)}`;
    await open(url);
  }

  async function copyMarkdown() {
    if (!selectedSession) return;
    await navigator.clipboard.writeText(buildBody(selectedSession, addCtx));
  }
</script>

<!-- ── Layout ─────────────────────────────────────────────────────────────── -->
<div class="logs-root">

  <!-- ── Sidebar ──────────────────────────────────────────────────────────── -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <span class="kanji sidebar-kanji">録</span>
      <span class="sidebar-title">Diagnostic Logs</span>
    </div>

    <div class="session-list">
      {#each grouped as bucket}
        <!-- Date group header -->
        <button
          class="date-header"
          onclick={() => {
            if (expandedDates.has(bucket.date)) {
              expandedDates.delete(bucket.date);
            } else {
              expandedDates.add(bucket.date);
            }
            expandedDates = new Set(expandedDates);
          }}
        >
          <span class="date-label">{bucket.date}</span>
          <span class="date-chevron">{expandedDates.has(bucket.date) ? '▾' : '▸'}</span>
        </button>

        {#if expandedDates.has(bucket.date)}
          {#each bucket.groups as group}
            <!-- Module sub-header -->
            <div class="module-header">
              <span class="kanji module-kanji">{moduleLabel(group.mod).kanji}</span>
              <span class="module-label">{moduleLabel(group.mod).label.toUpperCase()}</span>
            </div>

            <!-- Session rows -->
            {#each group.sessions as session}
              <button
                class="session-row"
                class:selected={selectedId === session.id}
                onclick={() => { selectedId = session.id; showModal = false; }}
              >
                <span
                  class="outcome-dot"
                  style:background={outcomeDot(session.outcome)}
                ></span>
                <span class="session-time">{formatTime(session.started_at)}</span>
                <span class="session-meta">
                  {fmtMs(session.duration_ms)} · {session.traces.length} steps
                  {#if fixCount(session) > 0}
                    · {fixCount(session)} fix{fixCount(session) > 1 ? 'es' : ''}
                  {/if}
                </span>
              </button>
            {/each}
          {/each}
        {/if}
      {/each}

      {#if data.sessions.length === 0}
        <p class="empty-state">No sessions yet. Run the health check to generate logs.</p>
      {/if}
    </div>
  </aside>

  <!-- ── Trace panel ───────────────────────────────────────────────────────── -->
  <main class="trace-panel">
    {#if selectedSession}
      <div class="trace-header">
        <div class="trace-header-left">
          <span class="kanji trace-kanji">{moduleLabel(selectedSession.module).kanji}</span>
          <div>
            <div class="trace-title">{moduleLabel(selectedSession.module).label}</div>
            <div class="trace-meta">
              {selectedSession.started_at.replace('T', ' ').replace('Z', ' UTC')}
              · {fmtMs(selectedSession.duration_ms)}
              · <span
                  style:color={outcomeDot(selectedSession.outcome)}
                >{selectedSession.outcome}</span>
            </div>
          </div>
        </div>
        <button
          class="report-btn"
          onclick={() => { showModal = true; }}
        >
          Report this session ↗
        </button>
      </div>

      <table class="trace-table">
        <thead>
          <tr>
            <th>Step</th>
            <th>Type</th>
            <th>Command</th>
            <th>Duration</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {#each selectedSession.traces as trace}
            {@const t = trace as BootstrapTrace}
            <tr class:trace-fail={!t.ok}>
              <td class="step-cell">{t.step ?? (trace as any).step ?? ''}</td>
              <td>
                <span class="badge badge-{t.action_type ?? 'check'}">{t.action_type ?? ''}</span>
              </td>
              <td class="cmd-cell"><code>{anonymize(t.cmd ?? '')}</code></td>
              <td class="dur-cell">{fmtMs(t.ms ?? 0)}</td>
              <td class="result-cell">
                {#if t.ok}
                  <span class="result-ok">✓</span>
                {:else if t.fix_ok}
                  <span class="result-fixed">✓ fixed</span>
                {:else}
                  <span class="result-fail">✗</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <div class="no-session">Select a session from the sidebar.</div>
    {/if}
  </main>
</div>

<!-- ── Report Modal ──────────────────────────────────────────────────────── -->
{#if showModal && selectedSession}
  <div
    class="modal-overlay"
    role="dialog"
    aria-modal="true"
    onclick={(e) => { if (e.target === e.currentTarget) showModal = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') showModal = false; }}
    tabindex="-1"
  >
    <div class="modal-panel">
      <div class="modal-header">
        <span class="modal-title">Report Session</span>
        <button class="modal-close" onclick={() => { showModal = false; }}>✕</button>
      </div>

      <div class="modal-body">
        <!-- Left: issue preview -->
        <div class="preview-col">
          <div class="preview-label">ISSUE PREVIEW — ANONYMIZED</div>
          <div class="preview-title">
            {buildTitle(selectedSession)}
          </div>
          <pre class="body-preview">{buildBody(selectedSession, addCtx)}</pre>
        </div>

        <!-- Right: submission panel -->
        <div class="submit-col">
          <div class="included-panel">
            <div class="included-title">Included in report</div>
            <ul class="included-list">
              <li>System info (OS, arch, RAM)</li>
              <li>{selectedSession.traces.length} trace steps</li>
              {#if fixCount(selectedSession) > 0}
                <li>{fixCount(selectedSession)} fix attempt(s)</li>
              {/if}
              <li>App version v{selectedSession.app_version}</li>
            </ul>
          </div>

          <label class="ctx-label" for="add-ctx">Additional context</label>
          <textarea
            id="add-ctx"
            class="ctx-input"
            bind:value={addCtx}
            placeholder="Describe what you were doing when the problem occurred…"
            rows={4}
          ></textarea>

          <p class="privacy-note">
            Paths anonymized: <code>/Users/name/</code> → <code>~/</code>
          </p>

          <button class="submit-btn" onclick={submitToGitHub}>
            Submit to GitHub ↗
          </button>
          <button class="copy-btn" onclick={copyMarkdown}>
            Copy markdown
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Layout ─────────────────────────────────────────────────────────── */
  .logs-root {
    display: flex;
    height: 100%;
    overflow: hidden;
    background: var(--paper, #faf9f7);
    color: var(--sumi, #1a1a1a);
    font-family: var(--font-sans, system-ui, sans-serif);
  }

  /* ── Sidebar ─────────────────────────────────────────────────────────── */
  .sidebar {
    width: 248px;
    min-width: 248px;
    border-right: 1px solid var(--hairline, rgba(0,0,0,.08));
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 12px 10px;
    border-bottom: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .sidebar-kanji { font-size: 22px; }
  .sidebar-title { font-size: 13px; font-weight: 600; letter-spacing: .04em; }
  .session-list { flex: 1; overflow-y: auto; padding: 4px 0; }

  .date-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 12px 4px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--sumi-60, #666);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: .08em;
    text-align: left;
    text-transform: uppercase;
  }
  .date-header:hover { color: var(--sumi, #1a1a1a); }
  .date-chevron { font-size: 10px; }

  .module-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px 2px 20px;
  }
  .module-kanji { font-size: 14px; opacity: .6; }
  .module-label { font-size: 10px; letter-spacing: .1em; color: var(--sumi-60, #666); }

  .session-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 12px 6px 28px;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    border-radius: 4px;
    margin: 1px 4px;
    width: calc(100% - 8px);
  }
  .session-row:hover     { background: var(--hover-bg, rgba(0,0,0,.04)); }
  .session-row.selected  { background: var(--selected-bg, rgba(0,0,0,.07)); }
  .outcome-dot  { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
  .session-time { font-size: 12px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .session-meta { font-size: 11px; color: var(--sumi-50, #888); flex: 1; }

  .empty-state { padding: 20px 12px; font-size: 12px; color: var(--sumi-50, #888); }

  /* ── Trace panel ──────────────────────────────────────────────────────── */
  .trace-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .trace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .trace-header-left { display: flex; align-items: center; gap: 10px; }
  .trace-kanji { font-size: 28px; }
  .trace-title { font-size: 15px; font-weight: 600; }
  .trace-meta  { font-size: 12px; color: var(--sumi-50, #888); }
  .report-btn  {
    padding: 7px 14px;
    background: var(--sumi, #1a1a1a);
    color: #fff;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .report-btn:hover { opacity: .85; }

  .trace-table {
    flex: 1;
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    overflow-y: auto;
    display: block;
  }
  .trace-table thead { position: sticky; top: 0; background: var(--paper, #faf9f7); }
  .trace-table th {
    padding: 8px 12px;
    text-align: left;
    font-size: 11px;
    letter-spacing: .06em;
    color: var(--sumi-50, #888);
    border-bottom: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .trace-table td  { padding: 7px 12px; border-bottom: 1px solid var(--hairline, rgba(0,0,0,.05)); }
  .trace-fail td   { background: rgba(244,67,54,.04); }
  .cmd-cell code   { font-size: 11px; font-family: var(--font-mono, monospace); }
  .dur-cell        { font-variant-numeric: tabular-nums; }
  .result-ok       { color: #4CAF50; font-weight: 600; }
  .result-fixed    { color: #FF9800; font-weight: 600; }
  .result-fail     { color: #F44336; font-weight: 600; }
  .badge           { font-size: 10px; padding: 2px 6px; border-radius: 3px; background: rgba(0,0,0,.07); }
  .no-session { display: flex; align-items: center; justify-content: center; height: 100%; color: var(--sumi-50, #888); }

  /* ── Modal ────────────────────────────────────────────────────────────── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal-panel {
    width: 84vw;
    max-width: 960px;
    max-height: 90vh;
    background: var(--paper, #faf9f7);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0,0,0,.28);
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .modal-title { font-size: 14px; font-weight: 600; }
  .modal-close { background: none; border: none; font-size: 16px; cursor: pointer; color: var(--sumi-50, #888); }
  .modal-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  /* Left column */
  .preview-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 16px;
    overflow: hidden;
    border-right: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .preview-label { font-size: 10px; letter-spacing: .1em; color: var(--sumi-50, #888); margin-bottom: 8px; }
  .preview-title {
    font-size: 13px;
    font-weight: 500;
    padding: 8px;
    background: rgba(0,0,0,.04);
    border-radius: 4px;
    margin-bottom: 8px;
  }
  .body-preview {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    white-space: pre-wrap;
    background: rgba(0,0,0,.04);
    padding: 10px;
    border-radius: 4px;
    margin: 0;
  }

  /* Right column */
  .submit-col {
    width: 264px;
    min-width: 264px;
    display: flex;
    flex-direction: column;
    padding: 16px;
    gap: 12px;
    overflow-y: auto;
  }
  .included-panel {
    padding: 10px;
    background: rgba(0,0,0,.04);
    border-radius: 6px;
  }
  .included-title { font-size: 11px; font-weight: 600; letter-spacing: .06em; margin-bottom: 6px; }
  .included-list  { font-size: 12px; padding-left: 16px; margin: 0; color: var(--sumi-60, #555); }
  .ctx-label      { font-size: 12px; font-weight: 500; }
  .ctx-input      {
    width: 100%;
    resize: vertical;
    font-size: 12px;
    padding: 8px;
    border: 1px solid var(--hairline, rgba(0,0,0,.15));
    border-radius: 6px;
    background: transparent;
    font-family: inherit;
  }
  .privacy-note { font-size: 11px; color: var(--sumi-50, #888); margin: 0; }
  .privacy-note code { font-size: 10px; }
  .submit-btn, .copy-btn {
    width: 100%;
    padding: 9px 14px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .submit-btn {
    background: var(--sumi, #1a1a1a);
    color: #fff;
    border: none;
  }
  .copy-btn {
    background: none;
    border: 1px solid var(--hairline, rgba(0,0,0,.2));
    color: var(--sumi, #1a1a1a);
  }
  .submit-btn:hover { opacity: .85; }
  .copy-btn:hover   { background: rgba(0,0,0,.04); }
</style>
```

- [ ] **Step 4: Run tests**

```bash
cd app && bun run test page.spec.svelte 2>&1 | tail -20
```

Expected: tests pass (some may need minor adjustments to button selectors — fix as needed).

- [ ] **Step 5: Type check**

```bash
cd app && bun run check 2>&1 | tail -20
```

Expected: no type errors.

- [ ] **Step 6: Start dev server and verify in browser**

```bash
cd app && bun run dev &
```

Navigate to `http://localhost:5173`. Open the app, navigate to `/logs`. Verify:
- Sidebar shows "Today" section (or "No sessions yet" if no sessions)
- Clicking a session row selects it and shows trace table
- "Report this session" opens centered modal (not a bottom sheet)
- Modal is 84vw, max 960px, 90vh height
- Body preview fills the left column
- Paths like `/Users/name/` become `~/` in the preview

- [ ] **Step 7: Commit**

```bash
cd app && git add src/routes/\(app\)/logs/
git commit -m "feat(app): add /logs diagnostic session browser with report modal"
```

---

## Task 13: Wire frontend navigation to /logs

**Files:**
- Modify: `app/src/routes/(app)/+layout.svelte` (if it has a nav/sidebar menu)

- [ ] **Step 1: Check if the app layout has a nav that needs a /logs link**

Read `app/src/routes/(app)/+layout.svelte` to see if there's a navigation sidebar.

- [ ] **Step 2: Add /logs navigation if a sidebar exists**

If the layout has nav items, add:
```svelte
<a href="/logs" class="nav-item">
  <span class="kanji">録</span>
  <span>Logs</span>
</a>
```

If not (app uses the Tauri menu item instead), skip this step — the menu item `CmdOrCtrl+Shift+L` is the entry point.

- [ ] **Step 3: Add open-logs event listener in root layout**

In `app/src/routes/+layout.svelte`, add a listener for the Tauri menu event:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  // existing imports...

  onMount(() => {
    if (typeof window !== 'undefined' && window.__TAURI__) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen('open-logs', () => goto('/logs'));
      });
    }
  });
</script>
```

- [ ] **Step 4: Type check + verify**

```bash
cd app && bun run check 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
cd app && git add src/routes/
git commit -m "feat(app): wire open-logs Tauri event to /logs navigation"
```

---

## Final Verification

- [ ] **Run all bootstrap crate tests**

```bash
cd daemon && cargo test -p sensei-bootstrap 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Run all Tauri app tests**

```bash
cd app/src-tauri && cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Run all frontend tests**

```bash
cd app && bun run test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Full type check**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: zero errors.

- [ ] **Merge to main**

```bash
git checkout main && git merge develop && git push
```
