# Codebase Audit — Issues Catalog

> Generated: 2026-05-11  
> Scope: All Rust crates (bootstrap, cli, mcp, senseid, gateway) + app/src-tauri  
> Method: Four parallel static analysis agents covering constants/config, code duplication, error handling, and modularity

---

## Status

| ID | Tier | Status | File(s) |
|----|------|--------|---------|
| [B1](#b1--removalrs-reads-wrong-json-key-removes-nothing) | Bug | ✅ Fixed | `senseid/src/installer/removal.rs:77` |
| [B2](#b2--start_daemon-hardcodes-port-7744-and-discards-spawn-error) | Bug | ✅ Fixed | `cli/src/main.rs:182–195` |
| [B3](#b3--mcp-server-panics-on-unexpected-daemon-json) | Bug | ✅ Fixed | `mcp/src/main.rs:478,486,493,507` |
| [B4](#b4--upsert_node-silently-duplicates-nodes-on-every-rescan) | Bug | ✅ Fixed | `senseid/src/db/pg_store.rs:344`, `database/ddl/table/sensei/nodes.ddl` |
| [P1](#p1--unwrap-in-health-http-handler) | Panic | ✅ Fixed | `senseid/src/api/handlers/health.rs:73` |
| [P2](#p2--mutexlockunwrap-in-circuit-breaker-crashes-all-inference) | Panic | ✅ Fixed | `gateway/src/circuit_breaker.rs:72,98,124,151,168,174` |
| [P3](#p3--current_direxpect-panics-cli) | Panic | ✅ Fixed | `cli/src/main.rs:460` |
| [A1](#a1--daemon-porturl-computed-four-incompatible-ways) | Architecture | ✅ Fixed (mcp) | `cli`, `mcp`, `app`, `bootstrap` |
| [A2](#a2--which_binary--which_exists--3-independent-implementations) | Architecture | ✅ Fixed | `bootstrap/util.rs`, `senseid/assistants/helpers.rs`, `cli/main.rs` |
| [A3](#a3--home--3-implementations-with-different-semantics) | Architecture | ✅ Fixed | `bootstrap`, `senseid/paths.rs`, `cli/main.rs` |
| [A4](#a4--is_dev-binary-name-detection-duplicated) | Architecture | ✅ Fixed | `bootstrap/config.rs`, `cli/main.rs`, `senseid/main.rs` |
| [A5](#a5--modesensei_mode-enum-defined-twice) | Architecture | ✅ Fixed | `bootstrap/config.rs`, `senseid/paths.rs` |
| [A6](#a6--assistantstatus-struct-defined-twice) | Architecture | ✅ Fixed | `senseid/assistants/mod.rs:92`, `app/src-tauri/commands/assistants.rs:15` |
| [A7](#a7--constants-scattered-across-crates) | Architecture | ✅ Fixed | Multiple |
| [A8](#a8--senseiconfig-json-readwrite-pattern-in-3-places) | Architecture | ✅ Fixed | `cli/main.rs`, `installer/catalog.rs`, `assistants/mod.rs` |
| [S1](#s1--mark_user_scope_configured-discards-write-failure) | Silent Failure | ✅ Fixed | `cli/main.rs:281` |
| [S2](#s2--config-write-uses-unwrap-before-ok) | Silent Failure | ✅ Fixed | `cli/main.rs:281` |
| [S3](#s3--all-pgstore-errors-erased-to-string) | Silent Failure | ⏳ Pending | `senseid/db/pg_store.rs` (110 sites) |
| [S4](#s4--resolve_project-falls-back-to-raw-hint-as-repo_id) | Silent Failure | ✅ Fixed | `mcp/main.rs:511` |
| [S5](#s5--watcher_status-creates-throwaway-taskqueue-per-http-call) | Silent Failure | ✅ Fixed | `senseid/api/handlers/health.rs` |
| [M1](#m1--boxleak-on-every-file-processed) | Memory Leak | ✅ Fixed | `senseid/languages/mod.rs:60` |
| [M2](#m2--boxleak-in-formula_for-for-unknown-service-names) | Memory Leak | ✅ Fixed | `bootstrap/platform/macos.rs:42` |
| [E1](#e1--adding-a-language-requires-editing-two-parallel-match-arms) | Extensibility | ✅ Fixed | `senseid/languages/mod.rs` |
| [E2](#e2--acp-registry-and-cli-acp-map-are-separate-hardcoded-lists) | Extensibility | ✅ Fixed | `cli/main.rs` |
| [E3](#e3--bootstrap-component-list-has-hardcoded-count-assertion) | Extensibility | ✅ Fixed | `bootstrap/prereq/registry.rs:241` |
| [PL1](#pl1--extra_paths-and-path-separator-are-unix-only-in-utilrs) | Platform | ✅ Fixed | `bootstrap/platform/macos.rs`, `bootstrap/platform/windows.rs` |
| [PL2](#pl2--hardware-detect_gpu-has-cfg-blocks-instead-of-delegating-to-platform) | Platform | ✅ Fixed | `bootstrap/hardware.rs`, `bootstrap/platform/macos.rs`, `bootstrap/platform/windows.rs` |
| [D1](#dead-code) | Dead Code | ✅ Fixed | `senseid/config/detector.rs:13,59` |
| [D2](#dead-code) | Dead Code | ✅ Fixed | `senseid/types.rs:272,317,354,367,388,398,412` |
| [N1](#n1--mcp-duplicates-binary-is-dev--port-logic) | Architecture | ⏳ Pending | `mcp/src/main.rs:14–19` |
| [N2](#n2--assistantsmod-hardcodes-sensei-breaks-in-dev-mode) | Architecture | ⏳ Pending | `senseid/assistants/mod.rs:190` |
| [N3](#n3--external_linksrs-hardcodes-senseirules-path) | Architecture | ⏳ Pending | `senseid/tasks/processors/metadata/external_links.rs:34` |
| [N4](#n4--pg_store-test-urls-hardcode-postgres-port) | Architecture | ⏳ Pending | `senseid/db/pg_store.rs:30,1413` |
| [N5](#n5--tauri-lib-hardcodes-github-url) | Architecture | ⏳ Pending | `app/src-tauri/src/lib.rs:156` |

---

## Tier 1: Confirmed Bugs

### B1 — `removal.rs` reads wrong JSON key, removes nothing

**File:** `crates/senseid/src/installer/removal.rs:72–81`  
**Status:** ✅ Fixed 2026-05-11

**Problem:**  
`remove_registered_projects()` reads `projects.json` as a `serde_json::Value` then accesses
`v["projects"].as_array()`. But `cli/src/main.rs:286` writes the projects list as a **root JSON
array** (`Vec<String>` serialized directly), not `{ "projects": [...] }`. The `v["projects"]`
lookup always returns `null`, so `remove_registered_projects` always produces an empty list and
silently removes nothing from registered project directories.

```rust
// BEFORE (broken) — reads a key that doesn't exist
.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
.and_then(|v| v["projects"].as_array().cloned())

// AFTER (fixed) — parse directly as the type that was written
.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
```

**Impact:** `sensei remove all --purge` does not clean up `.sensei/` directories in registered
project repos.

---

### B2 — `start_daemon` hardcodes port 7744 and discards spawn error

**File:** `crates/cli/src/main.rs:182–195`  
**Status:** ✅ Fixed 2026-05-11

**Problem (two defects, one function):**

1. Port `7744` is hardcoded as a string literal. `default_port()` exists on line 23 and returns
   the correct port (7745 in dev mode), but `start_daemon()` ignores it. In dev mode, `daemon_url()`
   points to `7745` while the spawned daemon listens on `7744` — `ensure_daemon()` polls for 5
   seconds and fails with no cause.

2. `let _ = spawn()` discards the spawn result entirely. If the binary isn't found, the error is
   invisible — the user sees only "Could not start daemon. Run: brew services start sensei" with
   no explanation.

```rust
// BEFORE
let _ = std::process::Command::new(&bin)
    .args(["start", "--port", "7744"])  // wrong port in dev
    .spawn();                            // error discarded

// AFTER
let port = default_port();
match std::process::Command::new(&bin)
    .args(["start", "--port", &port.to_string()])
    .spawn()
{
    Ok(_) => {}
    Err(e) => {
        eprintln!("Failed to spawn daemon ({}): {}", bin.display(), e);
        return;
    }
}
```

---

### B3 — MCP server panics on unexpected daemon JSON

**File:** `crates/mcp/src/main.rs:478, 486, 493, 507`  
**Status:** ✅ Fixed 2026-05-11

**Problem:**  
`resolve_project()` calls `.as_str().unwrap()` on JSON fields from daemon responses at four sites.
Any unexpected response structure (null field, non-string value, network error returning partial
JSON, schema change) causes an unrecovered panic that **kills the entire MCP server process**,
severing the LLM's tool connection. The MCP binary is a long-running stdio server and does not
restart automatically.

```rust
// BEFORE — panics if repo_id is null or non-string
return p["repo_id"].as_str().unwrap().to_string();

// AFTER — returns empty string; caller falls through to hint fallback
return p["repo_id"].as_str().unwrap_or("").to_string();
```

Applied at all 4 sites in `resolve_project()`.

---

### B4 — `upsert_node` silently duplicates nodes on every rescan

**File:** `crates/senseid/src/db/pg_store.rs:344–351`  
**Status:** ⏳ Needs DDL migration

**Problem:**  
`upsert_node` uses `INSERT ... ON CONFLICT DO NOTHING RETURNING id`. The `nodes` table
(see `database/ddl/table/sensei/nodes.ddl`) has **no UNIQUE constraint** beyond the primary key
`id uuid default gen_random_uuid()`. Because `id` is always a new UUID, the conflict clause
**never fires** — every call inserts a new duplicate row.

On every rescan, all nodes for a file are re-inserted, resulting in unbounded growth of the `nodes`
table and incorrect graph data (edges point to stale duplicate nodes).

**Fix requires two steps:**

1. **DDL migration:** Add a unique constraint on `(folder_id, kind, name, file_path)` (or the
   appropriate business key). This requires a `dbd` migration.

2. **Code fix:** Change `fetch_one` to `fetch_optional` and handle the conflict case:
   ```rust
   // When conflict fires (None returned), SELECT the existing id
   let row: Option<(uuid::Uuid,)> = query_as("INSERT ... ON CONFLICT (folder_id, kind, name, file_path) DO NOTHING RETURNING id")
       .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
   match row {
       Some((id,)) => Ok(id),
       None => {
           // Row existed — fetch its id
           let (id,): (uuid::Uuid,) = query_as(
               "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND kind=$2::sensei.node_kind AND name=$3 AND file_path=$4"
           ).bind(folder_id).bind(kind).bind(name).bind(file_path)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
           Ok(id)
       }
   }
   ```

**Tracked separately** — requires schema decision and migration coordination.

---

## Tier 2: Panics in Production Request Paths

### P1 — `.unwrap()` in health HTTP handler

**File:** `crates/senseid/src/api/handlers/health.rs:73`  
**Status:** ✅ Fixed 2026-05-11

**Problem:**  
`spawn_blocking(...).await.unwrap()` inside the `GET /health` handler. If the blocking task panics
(e.g., a database check deadlocks) or the JoinHandle is aborted by Tokio's task scheduler, this
`.unwrap()` propagates the panic to the Axum handler, crashing the worker task.

**Fix:** Match on the `JoinError` and return a degraded but valid response:
```rust
let (pg, ollama, db, models) = match tokio::task::spawn_blocking(...).await {
    Ok(data) => data,
    Err(e) => {
        tracing::error!("health check task panicked: {e}");
        return Json(HealthResponse { status: "error", ... });
    }
};
```

---

### P2 — `Mutex::lock().unwrap()` in circuit breaker crashes all inference

**File:** `crates/gateway/src/circuit_breaker.rs:72, 98, 124, 151, 168, 174` (6 sites)  
**Status:** ✅ Fixed 2026-05-11

**Problem:**  
All public methods call `.lock().unwrap()` on the shared `Mutex`. A panic anywhere that holds this
lock (in any thread) poisons the mutex permanently. Every subsequent call to `can_execute`,
`record_success`, `record_failure`, or `get_state` then also panics, making the circuit breaker
permanently broken and crashing all inference requests.

**Fix:** Recover from poisoned mutexes using `.unwrap_or_else(|e| e.into_inner())`. A poisoned
mutex means some thread panicked while updating the state — the state may be partially updated but
is recoverable. Continuing with the recovered guard is safer than cascading panics.

```rust
// BEFORE
let mut states = self.states.lock().unwrap();

// AFTER
let mut states = self.states.lock().unwrap_or_else(|e| e.into_inner());
```

---

### P3 — `current_dir().expect()` panics CLI

**File:** `crates/cli/src/main.rs:460`  
**Status:** ✅ Fixed 2026-05-11

**Problem:**  
`init_project_scope()` calls `std::env::current_dir().expect("Cannot determine current directory")`.
This panics with a Rust panic message rather than a user-friendly error when the CWD is unavailable
(e.g., deleted directory, network mount gone away, restricted container environment).

**Fix:** Exit cleanly with a human-readable message:
```rust
let repo_root = match std::env::current_dir() {
    Ok(p) => p,
    Err(e) => {
        eprintln!("Error: cannot determine current directory: {e}");
        std::process::exit(1);
    }
};
```

---

## Tier 3: Single Source of Truth Violations

### A1 — Daemon port/URL computed four incompatible ways

**Severity:** High — MCP always connects to wrong port in dev mode

| Location | Mechanism | Dev port | Prod port |
|---|---|---|---|
| `bootstrap/src/config.rs` | `SENSEI_MODE` env var | 7745 | 7744 |
| `cli/src/main.rs:19–25` | binary name ends with `-dev` | 7745 | 7744 |
| `mcp/src/main.rs:4` | **hardcoded const** | ❌ 7744 | 7744 |
| `app/src-tauri/src/commands/assistants.rs:7` | `cfg!(debug_assertions)` | 7745 | 7744 |

`SenseiConfig::from_env()` in `bootstrap` is the canonical source. `cli` bypasses it to avoid
heavy transitive deps (documented). `mcp` and `app` bypass it with no justification.

**Fix:** 
- `mcp`: implement `fn daemon_url() -> String` using binary-name detection (same as `cli:11-17`)
  or read `SENSEI_MODE` env var
- `app`: use `SENSEI_MODE` env var at runtime rather than `cfg!(debug_assertions)` at compile time
- Long-term: extract `bootstrap-constants` as a zero-dep crate so `cli` and `mcp` can import the
  port values without pulling in tokio/sysinfo

---

### A2 — `which_binary` / `which_exists` — 3 independent implementations

| Location | Checks Homebrew paths | Checks `~/.local/bin` |
|---|---|---|
| `bootstrap/src/util.rs:45` (`which_binary`) | ✅ Yes | ✅ Yes |
| `senseid/src/assistants/helpers.rs:5` (`which_exists`) | ❌ No | ❌ No |
| `cli/src/main.rs:148` (`which_exists`) | ❌ No (intentional) | ❌ No |

`senseid` already depends on `bootstrap`. The copy in `senseid/src/assistants/helpers.rs` should
call `sensei_bootstrap::util::which_binary`. The `cli` copy is intentionally limited to avoid
heavy deps — document this clearly.

---

### A3 — `home()` — 3 implementations with different semantics

| Location | Strategy | Divergence risk |
|---|---|---|
| `bootstrap/src/config.rs:188` | Raw `$HOME` env var, fallback `/tmp` | Sandboxed envs where `$HOME` ≠ `dirs::home_dir()` |
| `senseid/src/paths.rs:55` | `dirs::home_dir()`, fallback `/tmp` | Same |
| `cli/src/main.rs:129` | `dirs::home_dir()`, fallback `/tmp` | Same |

All three fall back to `/tmp` silently. A missing `$HOME` should warn the user, not silently
write ephemeral config to `/tmp` that vanishes on reboot.

**Fix:** One canonical `home()` function (in `senseid::paths` or `bootstrap::config`), with a
warn-on-fallback log. All three call sites reference the same function.

---

### A4 — `is_dev()` binary-name detection duplicated

**Files:** `cli/src/main.rs:11–17` and `senseid/src/main.rs:62–66`

Word-for-word identical 5-line block. `SenseiConfig::is_dev()` already exists in bootstrap.
Both should call it (or at minimum share the logic via a named `pub fn binary_is_dev() -> bool`
added to `bootstrap::config`).

---

### A5 — `Mode`/`SenseiMode` enum defined twice

`bootstrap/src/config.rs:58` defines `SenseiMode { Prod, Dev }`.  
`senseid/src/paths.rs:21` defines `Mode { Prod, Dev }` and immediately mirrors it from
`SenseiConfig` via `init_from_env()`. The local `Mode` enum adds no logic — remove it and use
`bootstrap::SenseiMode` directly throughout `senseid`.

---

### A6 — `AssistantStatus` struct defined twice

`senseid/src/assistants/mod.rs:92` (authoritative definition, used in all daemon code).  
`app/src-tauri/src/commands/assistants.rs:15` (silent copy that deserializes the same JSON).

Both are in the same workspace. The Tauri copy will silently diverge as daemon fields evolve.
The Tauri crate should either import from `senseid` or deserialize to `serde_json::Value` at
the boundary and convert explicitly.

---

### A7 — Constants scattered across crates

| Constant | Canonical location | Duplicated in |
|---|---|---|
| `BREW_TAP = "sensei-hq/tap/sensei"` | `bootstrap/src/config.rs:21` | `cli/src/main.rs:8` |
| Port 7744/7745 | `bootstrap/src/config.rs:49–56` | `cli/main.rs:23–25`, `mcp/main.rs:4`, `senseid/paths.rs:41` |
| Ollama URL `http://localhost:11434` | should use `OLLAMA_PORT` constant | `senseid/api/gateway_init.rs:109` (inline) |
| PostgreSQL formula `"postgresql@17"` | should be in `config.rs` | `bootstrap/platform/macos.rs:39` (match arm) |
| MCP server version `"0.1.0"` | should be `env!("CARGO_PKG_VERSION")` | `mcp/main.rs:63` |
| MCP protocol version `"2024-11-05"` | should be named constant | `mcp/main.rs:62` |

---

### A8 — `~/.sensei/config.json` read/write pattern in 3 places

`cli/src/main.rs:270`, `senseid/src/installer/catalog.rs:74`,
`senseid/src/assistants/mod.rs:191` — each independently implements the read-parse-mutate-write
loop on the same file. No locking between processes. A shared `SenseiLocalConfig` struct with
`load()` / `save()` methods would eliminate all three copies and prevent partial-write corruption.

---

## Tier 4: Silent Failures That Hide Errors From Users

### S1 — `mark_user_scope_configured` discards write failure

**File:** `crates/cli/src/main.rs:281`

```rust
fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap()).ok();
```

`.ok()` silently swallows disk-full and permissions errors. If this fails, every subsequent CLI
run re-runs first-time setup (user sees the welcome prompt on every command).

---

### S2 — Config write uses `unwrap()` before `.ok()`

**File:** `crates/cli/src/main.rs:281`

`serde_json::to_string_pretty(&config).unwrap()` on the same line. While this specific JSON value
is not serialization-fail-able, `.unwrap()` in non-test code is a smell — and the outer `.ok()`
means even a panic-recovery path would lose the error.

---

### S3 — All `PgStore` errors erased to `String` (~110 sites)

**File:** `crates/senseid/src/db/pg_store.rs`

Every `.map_err(|e| e.to_string())` discards `sqlx::Error`'s structured information: constraint
name, error code, column, detail. Callers cannot programmatically distinguish:
- Unique constraint violation (expected on rescan)
- Connection failure (retry)
- Type mismatch (bug)
- Deadlock (retry with backoff)

**Fix:** Define a `StoreError` enum with variants for the cases callers need to handle, and match
on `sqlx::Error` discriminants in `map_err`.

---

### S4 — `resolve_project` falls back to raw hint as repo_id

**File:** `crates/mcp/src/main.rs:511`

```rust
hint.to_string() // fallback: use hint as-is
```

An arbitrary user-typed string propagates as a `repo_id` to every daemon API call, returning empty
data with no error surface to the LLM. The LLM has no way to know the project wasn't found.

**Fix:** Return an explicit error result:
```rust
return json!({
    "content": [{"type": "text", "text": format!("Project not found: '{hint}'. Use sensei status to list available projects.")}],
    "isError": true
});
```

---

### S5 — `watcher_status` creates throwaway `TaskQueue` per HTTP call

**File:** `crates/senseid/src/api/handlers/health.rs:92`

```rust
let queue = std::sync::Arc::new(crate::tasks::queue::TaskQueue::new());
let watcher = crate::watcher::root_watcher::RootWatcher::instance(queue);
```

Creates a fresh disconnected queue on every `/watcher/status` call just to pass to the singleton
watcher. The queue is discarded immediately. If `RootWatcher::instance()` stores the passed queue,
this reinjects a disconnected queue on every health check, potentially breaking task routing.

**Fix:** Pass the shared `TaskQueue` from `AppState` rather than constructing a throwaway one.

---

## Tier 5: Memory Leaks

### M1 — `Box::leak` on every file processed (accumulating)

**File:** `crates/senseid/src/languages/mod.rs:60`

```rust
let lang = a.language().to_string();
(a, Box::leak(lang.into_boxed_str()) as &str)
```

`adapter_for_filename` leaks a `&'static str` on every call. With 11 language adapters the total
leaked memory is bounded by adapter count × scan frequency — but the same strings ("rust",
"typescript", etc.) are leaked repeatedly rather than interned once.

**Fix:** Use a `OnceLock<HashMap<&'static str, &'static str>>` intern table initialized at startup,
or change the return type from `&'static str` to `String`.

---

### M2 — `Box::leak` in `formula_for` for unknown service names

**File:** `crates/bootstrap/src/platform/macos.rs:42`

```rust
_ => Box::leak(name.to_string().into_boxed_str())
```

Bounded for the current single-call-per-service pattern, but leaks permanently if called with
arbitrary input (e.g., from a future REST API).

**Fix:** Change return type to `Cow<'static, str>` — known names return `Cow::Borrowed`, unknown
names return `Cow::Owned` without leaking.

---

## Tier 6: Extensibility Issues

### E1 — Adding a language requires editing two parallel match arms

**File:** `crates/senseid/src/languages/mod.rs:24–80`

`adapter_for_ext` and `parse_to_ir_for_ext` are separate match arms with no shared registry.
Adding Go, Ruby, Elixir, etc. requires editing two functions in `mod.rs` plus two filename
handlers. A language registration table (`HashMap<&str, Box<dyn LanguageAdapter>>`) would make
each new language a single self-contained module addition.

---

### E2 — ACP registry and CLI ACP map are separate hardcoded lists

`senseid/src/assistants/mod.rs:14–88` — hardcoded `vec![...]` with a count assertion (`== 8`).  
`cli/src/main.rs:624–643` — independent `match` arm mapping ACP names to IDs.

Adding a new ACP requires editing both files. The CLI should query the daemon's ACP list rather
than maintaining its own copy.

---

### E3 — Bootstrap component list has hardcoded count assertion

**File:** `crates/bootstrap/src/prereq/registry.rs:241`

`assert!(COMPONENTS.len() == 10)` means adding any new bootstrap component requires updating this
number. Replace with a uniqueness assertion (no duplicate IDs) which is the actual invariant that
matters.

---

## Tier 7: Platform Code in Wrong Module

### PL1 — `EXTRA_PATHS` and PATH separator are Unix-only in `util.rs`

**File:** `crates/bootstrap/src/util.rs:30–36`

`/opt/homebrew/bin` paths and `:` separator are meaningless and invalid on Windows. `enrich_path()`
should be `#[cfg(unix)]` or moved into `platform/macos.rs` with the Windows provider using its
own equivalent.

---

### PL2 — `hardware.rs` `detect_gpu()` has `#[cfg]` blocks instead of delegating to platform

**File:** `crates/bootstrap/src/hardware.rs:21–46`

GPU detection calls `sysctl` (macOS) and `lspci` (Linux) directly from within the cross-platform
`hardware.rs` module. Should delegate to a `PlatformProvider::detect_gpu()` method, consistent
with how `start_service` and `check_package_manager` are platform-abstracted.

---

## Dead Code

### D1 — Two dead public functions in `senseid/src/config/detector.rs`

`detect_stack` (line 13) and `detect_config_files` (line 59) are annotated `#[allow(dead_code)]`
and never called from any live code path. Both have test coverage, which is misleading — the
tests verify the functions work but not that they are used. Either wire these into the scan
pipeline or delete them.

### D2 — Multiple `#[allow(dead_code)]` types in `senseid/src/types.rs`

`Repo` (272), `Project` (317), `IndexError` (354), `GraphNode` (367), `GraphEdge` (388),
`FunctionDetail` (398), `TypeDetail` (412), `default_status()` (304), `default_role()` (309),
`default_category()` (335).

These appear to be planned types for a graph query layer that has not yet been implemented. They
should either be wired to the live database query results or removed until needed.

---

## Tier 8: Bootstrap Consolidation (New — 2026-05-11)

### N1 — MCP duplicates `binary_is_dev` + port logic

**File:** `crates/mcp/src/main.rs:14–19`

Word-for-word copy of the binary-name detection pattern that was consolidated into
`sensei_bootstrap::binary_is_dev()` and `SenseiConfig::detect()`. MCP never received
the same cleanup as cli and senseid.

**Fix:** `SenseiConfig::detect().daemon_url()` — one call replaces the 7-line block.

---

### N2 — `assistants/mod.rs` hardcodes `.sensei`, breaks in dev mode

**File:** `crates/senseid/src/assistants/mod.rs:190`

```rust
let sensei_dir = h.join(".sensei");
```

In dev mode (`senseid-dev`) the data directory is `~/.sensei-dev/`, so this always
points to the wrong directory, silently reading/writing prod config during dev runs.

**Fix:** `crate::paths::sensei_dir()` or `SenseiConfig::detect().sensei_dir()`.

---

### N3 — `external_links.rs` hardcodes `.sensei/rules.md` path

**File:** `crates/senseid/src/tasks/processors/metadata/external_links.rs:34`

```rust
".sensei/rules.md",
```

The metadata scanner looks for project rules in `.sensei/rules.md`. In dev mode the
directory is `.sensei-dev/`, so the scanner silently skips rules files during dev indexing.

**Fix:** Build the path from `SenseiConfig::detect().dir_suffix` at scan time.

---

### N4 — `pg_store` test URLs hardcode PostgreSQL port

**Files:** `crates/senseid/src/db/pg_store.rs:30, 1413`

```rust
"postgresql://localhost:5432/sensei_test"
"postgresql://localhost:5432/sensei"
```

Both are test-only fallback URLs. The port `5432` is already `sensei_bootstrap::POSTGRES_PORT`.
If the port constant changes, these will silently diverge.

**Fix:** `format!("postgresql://localhost:{}/sensei_test", sensei_bootstrap::POSTGRES_PORT)`.

---

### N5 — Tauri lib hardcodes GitHub issues URL

**File:** `app/src-tauri/src/lib.rs:156`

```rust
"https://github.com/sensei-hq/sensei/issues"
```

`GITHUB_ORG` and `GITHUB_REPO` are already constants in `sensei-bootstrap`. If the repo
is ever renamed or moved, this will be the only place that doesn't update automatically.

**Fix:** `format!("https://github.com/{}/{}/issues", sensei_bootstrap::GITHUB_ORG, sensei_bootstrap::GITHUB_REPO)`.

---

## Dependency Graph for Fixes

```
B1 (removal JSON key)          — standalone, no deps
B2 (start_daemon port)         — standalone
B3 (MCP unwrap)                — standalone
B4 (node dedup)                — BLOCKS: DDL migration required first
P1 (health unwrap)             — standalone
P2 (circuit breaker mutex)     — standalone
P3 (CLI current_dir)           — standalone
A1 (daemon port SSOT)          — BLOCKS A4, A7
A2 (which_binary SSOT)        — depends on A1 (bootstrap dep decision)
A8 (config.json accessor)      — standalone
S4 (MCP project not found)     — relates to B3 (already partially fixed)
M1 (Box::leak languages)       — standalone
M2 (Box::leak formula_for)     — standalone
```
