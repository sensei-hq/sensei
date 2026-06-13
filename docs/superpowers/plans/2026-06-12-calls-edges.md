# Populate `calls` Edges (Rust, function-granular) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Rust adapter extract function/method call-sites so the indexer writes `calls` edges sourced from the *caller function node*, resurrecting the dead `get_callers`/`get_callees`/`call_flow` tools.

**Architecture:** The full `calls` pipeline (emit → resolve → `call_graph` view → tools) already exists; only call-site extraction (`rust_lang.rs::parse()` hardcodes `edges: vec![]`) and function-granular edge sourcing (process.rs sources every edge from the *file* node) are missing. The adapter owns all extraction + a per-adapter denylist + dedup; process.rs persists the adapter's edge intent generically, resolving each edge's source from a `(name, line_start) → node-id` map built while inserting the file's symbols.

**Tech Stack:** Rust, tree-sitter (`tree_sitter_rust`), sqlx/Postgres (`sensei_test` for DB tests). Daemon crate `senseid` is bin-only — unit tests run via `cargo test -p senseid --bin senseid`.

**Spec:** `docs/superpowers/specs/2026-06-12-calls-edges-design.md`

---

## File structure

| File | Responsibility | Change |
|------|----------------|--------|
| `crates/senseid/src/types.rs` | Adapter output types | add `caller_line: u32` to `ParsedEdge` |
| `crates/senseid/src/languages/rust_lang.rs` | Rust adapter — extraction, denylist, dedup | the real work: extract call-sites into `ParsedFile.edges` |
| `crates/senseid/src/tasks/processors/types.rs` | Pipeline result types | add `caller_line: u32` to `UnresolvedCall` |
| `crates/senseid/src/tasks/processors/code.rs` | Maps `ParsedFile` → `FileProcessResult` | carry `caller_line` through |
| `crates/senseid/src/tasks/handlers/process.rs` | Persists results to PG | function-granular edge sourcing via `(name,line)→id` map |

No DDL, resolve, view, or tool changes — all reused unchanged.

**Adapter state note (verified):** No code anywhere constructs a `ParsedEdge` literal today, so adding the required `caller_line` field breaks no existing call sites. `python.rs::find_calls` (:474) is a stub — it has the caller line-range traversal but an empty `if` block (`// requires source bytes`), so `extract_edges` always returns `vec![]`. `typescript.rs` has an unused `_edges` param. Completing those adapters (Python is closest — it already does line-range caller attribution) is the natural follow-up issue and should adopt this same contract + a per-adapter denylist.

---

## Task 1: Extract call-sites in the Rust adapter

**Files:**
- Modify: `crates/senseid/src/types.rs:255-260` (`ParsedEdge`)
- Modify: `crates/senseid/src/languages/rust_lang.rs` (`parse()` at :16-41, `walk_nodes` at :413-518, new helpers)
- Test: `crates/senseid/src/languages/rust_lang.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing unit tests**

Add to the `tests` module in `rust_lang.rs` (the module already has a `fn parse(src: &str) -> ParsedFile` helper). Append these tests:

```rust
    #[test]
    fn extracts_free_function_call() {
        let pf = parse("pub fn caller() { callee(); }\npub fn callee() {}");
        assert!(pf.edges.iter().any(|e| e.caller_name == "caller" && e.callee_name == "callee"),
            "expected caller→callee edge, got {:?}", pf.edges);
    }

    #[test]
    fn extracts_path_call_last_segment() {
        let pf = parse("pub fn f() { std::mem::swap(); }");
        assert!(pf.edges.iter().any(|e| e.caller_name == "f" && e.callee_name == "swap"),
            "scoped call should yield last segment 'swap', got {:?}", pf.edges);
    }

    #[test]
    fn extracts_method_call() {
        let pf = parse("pub fn f(pg: Pg) { pg.insert_memory(); }");
        assert!(pf.edges.iter().any(|e| e.caller_name == "f" && e.callee_name == "insert_memory"),
            "method call should yield 'insert_memory', got {:?}", pf.edges);
    }

    #[test]
    fn skips_denylisted_methods() {
        let pf = parse("pub fn f(x: String) { let _ = x.clone(); let _ = x.len(); }");
        assert!(!pf.edges.iter().any(|e| e.callee_name == "clone"), "clone denylisted");
        assert!(!pf.edges.iter().any(|e| e.callee_name == "len"), "len denylisted");
    }

    #[test]
    fn skips_macros() {
        let pf = parse("pub fn f() { println!(\"hi\"); }");
        assert!(!pf.edges.iter().any(|e| e.callee_name == "println"), "macros are not call_expressions");
    }

    #[test]
    fn dedups_repeated_calls() {
        let pf = parse("pub fn f() { g(); g(); g(); }\npub fn g() {}");
        let count = pf.edges.iter().filter(|e| e.caller_name == "f" && e.callee_name == "g").count();
        assert_eq!(count, 1, "repeated calls to g dedup to one edge");
    }

    #[test]
    fn captures_calls_inside_closures() {
        let pf = parse("pub fn f(v: Vec<u32>) { v.iter().for_each(|_| helper()); }\npub fn helper() {}");
        assert!(pf.edges.iter().any(|e| e.caller_name == "f" && e.callee_name == "helper"),
            "call inside a closure attributes to the enclosing fn, got {:?}", pf.edges);
    }

    #[test]
    fn same_named_methods_get_distinct_caller_lines() {
        // Two `new` methods in separate impls (line 2 and line 5) each call `setup`.
        let src = "pub struct A;\nimpl A { pub fn new() -> Self { setup(); A } }\n\npub struct B;\nimpl B { pub fn new() -> Self { setup(); B } }\npub fn setup() {}";
        let lines: Vec<u32> = pf_caller_lines(&parse(src), "new", "setup");
        assert_eq!(lines.len(), 2, "two distinct new→setup edges, got lines {:?}", lines);
        assert_ne!(lines[0], lines[1], "the two `new` callers have different caller_line");
    }

    // Helper: collect caller_line for every (caller,callee) edge matching names.
    fn pf_caller_lines(pf: &ParsedFile, caller: &str, callee: &str) -> Vec<u32> {
        pf.edges.iter()
            .filter(|e| e.caller_name == caller && e.callee_name == callee)
            .map(|e| e.caller_line)
            .collect()
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p senseid --bin senseid languages::rust_lang::tests 2>&1 | tail -20`
Expected: FAIL — `pf.edges` is always empty (every new assertion fails), and `e.caller_line` does not compile (`ParsedEdge` has no `caller_line` field).

- [ ] **Step 3: Add `caller_line` to `ParsedEdge`**

In `crates/senseid/src/types.rs`, change `ParsedEdge` (currently lines 255-260):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEdge {
    pub caller_name: String,
    pub caller_line: u32,
    pub callee_name: String,
    pub callee_file: Option<String>,
}
```

- [ ] **Step 4: Add the denylist and extraction helpers in `rust_lang.rs`**

At the top of `rust_lang.rs` (after the `use` lines, before the `impl`), add the per-adapter denylist:

```rust
/// Ubiquitous std/library methods whose call-sites carry no navigation signal.
/// Skipped at extraction to keep unresolvable noise out of `calls` edges.
/// Per-adapter by design — each language owns its own list.
const RUST_CALL_DENYLIST: &[&str] = &[
    "clone", "unwrap", "expect", "into", "to_string", "to_owned",
    "as_str", "as_ref", "iter", "into_iter", "map", "unwrap_or",
    "unwrap_or_default", "ok", "len", "is_empty", "push", "collect",
    "next", "borrow", "borrow_mut", "lock", "read", "write",
];
```

Then add two free functions near `walk_nodes` (e.g. just after it, before `line_at`). `source_text` already exists in this file (`:405`) and returns a node's text:

```rust
/// Extract the bare callee name from a `call_expression`'s `function` field.
/// `foo()` → "foo"; `a::b::c()` → "c"; `recv.method()` → "method".
/// Returns None for unsupported call forms (e.g. calling a closure value).
fn callee_name(call: &Node, src: &[u8]) -> Option<String> {
    let func = call.child_by_field_name("function")?;
    match func.kind() {
        "identifier" => Some(source_text(&func, src)),
        "scoped_identifier" => func
            .child_by_field_name("name")
            .map(|n| source_text(&n, src))
            .or_else(|| source_text(&func, src).rsplit("::").next().map(|s| s.to_string())),
        "field_expression" => func.child_by_field_name("field").map(|n| source_text(&n, src)),
        _ => None,
    }
}

/// Recursively collect call-sites under `node`, attributing each to `caller`.
/// Descends through all children (incl. closures and nested blocks) so calls
/// made anywhere in the function body are attributed to the enclosing fn.
/// Dedups per (caller, caller_line, callee) via `seen`.
fn collect_calls(
    node: &Node,
    src: &[u8],
    caller: &str,
    caller_line: u32,
    edges: &mut Vec<ParsedEdge>,
    seen: &mut std::collections::HashSet<String>,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "call_expression"
            && let Some(name) = callee_name(&child, src)
            && !RUST_CALL_DENYLIST.contains(&name.as_str())
            && seen.insert(format!("{caller}:{caller_line}:{name}"))
        {
            edges.push(ParsedEdge {
                caller_name: caller.to_string(),
                caller_line,
                callee_name: name,
                callee_file: None,
            });
        }
        collect_calls(&child, src, caller, caller_line, edges, seen);
    }
}
```

- [ ] **Step 5: Wire extraction into `walk_nodes` and `parse()`**

Change the `walk_nodes` signature (`:413`) to thread the edges + dedup set:

```rust
fn walk_nodes(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>, edges: &mut Vec<ParsedEdge>, seen: &mut std::collections::HashSet<String>, impl_type: Option<&str>) {
```

In the `"function_item"` arm (`:417-431`), extract calls from the function body before pushing the symbol (so `name` can still be moved into the symbol):

```rust
            "function_item" => {
                let name = field_text(&child, "name", src);
                let caller_line = child.start_position().row as u32 + 1;
                if let Some(body) = child.child_by_field_name("body") {
                    collect_calls(&body, src, &name, caller_line, edges, seen);
                }
                let is_pub = has_child_kind(&child, "visibility_modifier");
                let kind = if impl_type.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                symbols.push(ParsedSymbol {
                    name,
                    kind,
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: is_pub,
                    parent: impl_type.map(|s| s.to_string()),
                });
            }
```

In the `"impl_item"` arm (`:492-499`), pass the new args through the recursive call:

```rust
            "impl_item" => {
                let type_name = field_text(&child, "type", src);
                let type_name_ref = if type_name.is_empty() { None } else { Some(type_name.as_str()) };
                if let Some(body) = child.child_by_field_name("body") {
                    walk_nodes(&body, src, lines, symbols, imports, edges, seen, type_name_ref);
                }
            }
```

In `parse()` (`:30-40`), create the edges vec + dedup set, pass them, and return the edges:

```rust
        let mut symbols = Vec::new();
        let mut imports = Vec::new();
        let mut edges = Vec::new();
        let mut seen = std::collections::HashSet::new();
        walk_nodes(&root, src, &lines, &mut symbols, &mut imports, &mut edges, &mut seen, None);

        ParsedFile {
            file_path: file_path.to_string(),
            language: "rust".to_string(),
            symbols,
            edges,
            imports,
        }
```

Add `use crate::types::ParsedEdge;` to the existing `use crate::types::{...}` import on `:3`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p senseid --bin senseid languages::rust_lang::tests 2>&1 | tail -20`
Expected: PASS — all 8 new tests green, existing rust_lang tests still green.

- [ ] **Step 7: Zero-errors gate + commit**

Run: `cargo clippy -p senseid --bin senseid --all-targets 2>&1 | tail -20`
Expected: no warnings in `rust_lang.rs`/`types.rs`.

```bash
git add crates/senseid/src/types.rs crates/senseid/src/languages/rust_lang.rs
git commit -m "feat(senseid): extract Rust call-sites into ParsedFile.edges (#57)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Source `calls` edges from the caller function node

**Files:**
- Modify: `crates/senseid/src/tasks/processors/types.rs:42-46` (`UnresolvedCall`)
- Modify: `crates/senseid/src/tasks/processors/code.rs:73-77` (mapping)
- Modify: `crates/senseid/src/tasks/handlers/process.rs:618-640` (symbol-id map + source lookup)
- Test: `crates/senseid/src/tasks/handlers/process.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing integration test**

Add to the `tests` module in `process.rs` (it already has `make_ctx()`, and imports `Task`, `TaskKind`). This test writes a Rust file, runs `process_file`, and asserts the `calls` edge's `source_id` is the **caller function node**, not the file node:

```rust
    #[tokio::test]
    async fn calls_edge_sourced_from_caller_function_node() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let file_abs = src_dir.join("lib.rs");
        std::fs::write(&file_abs, "pub fn caller() { callee(); }\npub fn callee() {}").unwrap();

        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&repo_path, "cg", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "cg-repo", &repo_path).await.unwrap();

        let task = Task::new(TaskKind::ProcessFile, &repo_path, &file_abs.to_string_lossy());
        process_file(&ctx, &task).await.unwrap();

        let nodes = ctx.pg().get_nodes_by_folder(&fid).await.unwrap();
        let caller_id = nodes.iter()
            .find(|n| n["name"].as_str() == Some("caller") && n["kind"].as_str() == Some("function"))
            .and_then(|n| crate::api::util::json_uuid(&n["id"]))
            .expect("caller function node exists");
        let file_id = nodes.iter()
            .find(|n| n["kind"].as_str() == Some("file"))
            .and_then(|n| crate::api::util::json_uuid(&n["id"]))
            .expect("file node exists");

        let edges = ctx.pg().get_edges_by_kind(&fid, "calls").await.unwrap();
        let edge = edges.iter()
            .find(|e| e["target_name"].as_str() == Some("callee"))
            .expect("a calls edge to callee exists");
        let source_id = crate::api::util::json_uuid(&edge["source_id"]).unwrap();

        assert_eq!(source_id, caller_id, "edge sourced from the caller fn node");
        assert_ne!(source_id, file_id, "edge NOT sourced from the file node");
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p senseid --bin senseid calls_edge_sourced_from_caller_function_node -- --nocapture 2>&1 | tail -25`
Expected: FAIL — either `UnresolvedCall` has no `caller_line` (compile error once code.rs is touched) or the edge's `source_id` equals the file node id (current file-granular behavior), so `assert_eq!(source_id, caller_id)` fails.

- [ ] **Step 3: Add `caller_line` to `UnresolvedCall`**

In `crates/senseid/src/tasks/processors/types.rs`, change `UnresolvedCall` (lines 42-46):

```rust
#[derive(Debug, Clone, Serialize)]
pub struct UnresolvedCall {
    pub caller_name: String,
    pub caller_line: u32,
    pub callee_name: String,
}
```

- [ ] **Step 4: Carry `caller_line` through the `code.rs` mapping**

In `crates/senseid/src/tasks/processors/code.rs`, change the `unresolved_calls` mapping (lines 73-77):

```rust
    let unresolved_calls: Vec<UnresolvedCall> = parsed.edges.iter()
        .map(|e| UnresolvedCall {
            caller_name: e.caller_name.clone(),
            caller_line: e.caller_line,
            callee_name: e.callee_name.clone(),
        }).collect();
```

- [ ] **Step 5: Build the `(name,line)→id` map and source edges from it in `process.rs`**

In `crates/senseid/src/tasks/handlers/process.rs`, replace the symbol-insert loop (lines 618-626) so it captures each symbol's node id:

```rust
            // Write symbol nodes (functions, classes, types, etc.), capturing
            // each id keyed by (name, line_start) so call edges can be sourced
            // from the caller node — not the file. Generic graph-wiring; reused
            // by any symbol-sourced edge kind. Keyed on line because same-named
            // methods across impl blocks are legal in Rust.
            let mut sym_ids: std::collections::HashMap<(String, i32), uuid::Uuid> =
                std::collections::HashMap::new();
            for sym in &result.symbols {
                let parent_uuid = file_node_id; // symbols are children of the file
                if let Ok(id) = ctx.pg().upsert_node(
                    &folder_id, &sym.kind, &sym.name, &result.rel_path,
                    parent_uuid.as_ref(), sym.signature.as_deref(),
                    Some(sym.line as i32), Some(sym.line_end as i32),
                ).await {
                    sym_ids.insert((sym.name.clone(), sym.line as i32), id);
                }
            }
```

Then replace the call-edge loop (lines 635-640):

```rust
            // Write unresolved call edges, sourced from the caller function node
            // (falling back to the file node when the call has no enclosing named
            // symbol). target stays unresolved for resolve_edges to point.
            for call in &result.unresolved_calls {
                let source = sym_ids
                    .get(&(call.caller_name.clone(), call.caller_line as i32))
                    .copied()
                    .or(file_node_id);
                if let Some(src_id) = source {
                    ctx.pg().insert_edge(&folder_id, &src_id, None, Some(&call.callee_name), "calls").await.ok();
                }
            }
```

(The `imports`, `extends`/parent-ref, and doc-ref loops are left exactly as they are — genuinely file-sourced.)

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p senseid --bin senseid calls_edge_sourced_from_caller_function_node -- --nocapture 2>&1 | tail -25`
Expected: PASS — `source_id == caller_id` and `source_id != file_id`.

- [ ] **Step 7: Run the broader fast suite + clippy + commit**

Run: `cargo test -p senseid --bin senseid tasks:: 2>&1 | tail -20`
Expected: PASS — existing process/resolve tests still green.

Run: `cargo clippy -p senseid --bin senseid --all-targets 2>&1 | tail -20`
Expected: no new warnings.

```bash
git add crates/senseid/src/tasks/processors/types.rs crates/senseid/src/tasks/processors/code.rs crates/senseid/src/tasks/handlers/process.rs
git commit -m "feat(senseid): source calls edges from caller fn node, not file (#57)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Live end-to-end verification + close-out

**Files:** none (verification + docs). Requires the built daemon and a running `sensei` Postgres (port 7744).

- [ ] **Step 1: Build + install the debug service binaries**

Run: `make install-debug 2>&1 | tail -15`
Expected: senseid/sensei/sensei-mcp overlaid into the brew prefix.

- [ ] **Step 2: Trigger a rescan of the `sensei` repo**

Re-index so today's code (incl. `federation/mod.rs`) gets `calls` edges. Use the existing rescan path (CLI or API — the daemon must be running):

Run: `sensei scan --path /Users/Jerry/Developer/sensei-hq/sensei 2>&1 | tail -5` (or trigger via the app/API if the CLI flag differs — confirm the actual rescan command before running).
Expected: a scan task is queued; wait for the folder to reach `indexed`.

- [ ] **Step 3: Verify `calls` edges now exist**

Run (via `psql` against the `sensei` DB or the daemon's query endpoint):
```sql
SELECT kind, count(*) FROM sensei.edges GROUP BY 1 ORDER BY 2 DESC;
```
Expected: a `calls` row with count > 0 (previously absent).

- [ ] **Step 4: Verify the navigation tools return rows**

```sql
-- callers of insert_memory should include a function in federation/mod.rs
SELECT source_name, source_kind, source_file, source_line
  FROM sensei.call_graph
 WHERE folder = 'sensei' AND target_name = 'insert_memory' AND edge_kind = 'calls';
```
Expected: ≥1 row; at least one `source_file` = `crates/senseid/src/federation/mod.rs`, `source_kind` = `function`/`method` (not `file`).

```sql
-- callees of that caller (substitute the source_name found above)
SELECT target_name, unresolved_target FROM sensei.call_graph
 WHERE folder = 'sensei' AND source_name = '<caller from above>' AND edge_kind = 'calls';
```
Expected: ≥1 callee row.

If the `sensei` MCP plugin is live this session, dogfood the same checks via `get_callers(insert_memory)` / `get_callees(...)` instead of raw SQL.

- [ ] **Step 5: Close out**

- Comment on / close issue #57 with the before/after `edges` group-by counts and a sample `get_callers(insert_memory)` result.
- Mark the relevant `docs/backlog.md` item done (per project rules).
- Update memory pickup note: #57 shipped; next candidates are per-language adapters (Python/Svelte/TS) following the cross-adapter contract, and revisiting the `get_callers` unresolved quick-win if needed.

---

## Self-review

**Spec coverage:**
- Extract free/path/method calls → Task 1 (callee_name handles identifier/scoped_identifier/field_expression). ✓
- Per-adapter denylist (decision B) → Task 1, `RUST_CALL_DENYLIST` const. ✓
- Dedup per (caller, line, callee) → Task 1, `seen` set. ✓
- Function-granular sourcing (decision A) → Task 2, `(name,line)→id` map + file fallback. ✓
- Caller attribution by (name, line_start) → Tasks 1+2; `caller_line` threaded ParsedEdge → UnresolvedCall → process map. ✓
- Bare/last-segment callee names → Task 1 `callee_name`. ✓
- Defer get_callers unresolved quick-win (decision A) → no change made. ✓
- Cross-adapter contract preserved → all extraction in adapter; process.rs change is generic. ✓
- Macros skipped → Task 1 `skips_macros` test (macros aren't `call_expression`). ✓
- End-to-end verification (rescan, counts, tools) → Task 3. ✓

**Placeholder scan:** No TBD/TODO. Task 3 Step 2 flags "confirm the actual rescan command" — a deliberate runtime check, not a code placeholder.

**Type consistency:** `caller_line: u32` on both `ParsedEdge` and `UnresolvedCall`; map keyed `(String, i32)` with `sym.line as i32` and `call.caller_line as i32` (both cast consistently); `callee_name`/`caller_name` field names match across `ParsedEdge`, `UnresolvedCall`, and the `code.rs` mapping; `source_text`/`field_text`/`has_child_kind`/`collect_doc_comments`/`line_at` are all existing helpers in `rust_lang.rs`.
