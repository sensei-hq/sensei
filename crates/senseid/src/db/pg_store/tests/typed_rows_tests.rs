//! #161: a persistence function's positional parameter list is a hand-copied
//! enumeration of a row's fields, and a field added to the producer goes silent
//! at that boundary — no compile error, no failing test, the value just stops.
//! A typed row destructured EXHAUSTIVELY makes the same addition a compile error.
//!
//! This pins the functions that still take five or more positional parameters.
//! A new one fails here; a converted one must come off the list. So the list
//! only shrinks, and nothing reaches eight again.

/// Every `pg_store` function that still takes five or more positional
/// parameters, as `file::name`. Convert one to a typed row, then delete its line.
const STILL_POSITIONAL: &[&str] = &[
    "governance.rs::assemble_context",
    "graph.rs::semantic_search_nodes",
    "logs.rs::query_logs",
    "playbook.rs::playbook_combo_trust",
];

/// The non-test source files of `db/pg_store`.
fn store_sources() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/db/pg_store");
    let mut out = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if p.is_dir()
            || !name.ends_with(".rs")
            || name.ends_with("_tests.rs")
            || name == "graph_seed.rs"
        {
            continue;
        }
        out.push((name, without_test_modules(&std::fs::read_to_string(&p).unwrap())));
    }
    out.sort();
    out
}

/// The source up to its first `#[cfg(test)]` module, line comments removed.
fn without_test_modules(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let next = lines[i + 1..].iter().find(|l| !l.trim().is_empty());
        if line.trim() == "#[cfg(test)]" && next.is_some_and(|l| l.trim_start().starts_with("mod "))
        {
            break;
        }
        out.push_str(line.split("//").next().unwrap_or(""));
        out.push('\n');
    }
    out
}

/// Each `fn` in `src` with the number of positional (non-`self`) parameters.
fn positional_counts(src: &str) -> Vec<(String, usize)> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(at) = src[i..].find("fn ") {
        let start = i + at;
        i = start + 3;
        if start > 0 && (b[start - 1].is_ascii_alphanumeric() || b[start - 1] == b'_') {
            continue;
        }
        let name: String =
            src[i..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if name.is_empty() {
            continue;
        }
        let mut j = i + name.len();
        let mut depth = 0i32;
        // Skip generics, if any, to the opening parenthesis.
        while j < b.len() {
            match b[j] {
                b'<' => depth += 1,
                b'>' => depth -= 1,
                b'(' if depth == 0 => break,
                _ => {}
            }
            j += 1;
        }
        let (mut params, mut cur, mut d) = (Vec::new(), String::new(), 0i32);
        j += 1;
        while j < b.len() {
            let c = b[j] as char;
            match c {
                '(' | '[' | '<' | '{' => d += 1,
                ')' if d == 0 => break,
                ')' | ']' | '>' | '}' => d -= 1,
                _ => {}
            }
            if c == ',' && d == 0 {
                params.push(std::mem::take(&mut cur));
            } else {
                cur.push(c);
            }
            j += 1;
        }
        params.push(cur);
        let n = params
            .iter()
            .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|p| {
                !p.is_empty() && !matches!(p.as_str(), "self" | "&self" | "&mut self" | "mut self")
            })
            .count();
        out.push((name, n));
        i = j;
    }
    out
}

fn still_positional() -> Vec<String> {
    let mut found = Vec::new();
    for (file, src) in store_sources() {
        for (name, n) in positional_counts(&src) {
            if n >= 5 {
                found.push(format!("{file}::{name}"));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

#[test]
fn no_new_persistence_function_takes_five_positional_parameters() {
    let found = still_positional();
    let allowed: Vec<String> = STILL_POSITIONAL.iter().map(|s| s.to_string()).collect();
    let new: Vec<&String> = found.iter().filter(|f| !allowed.contains(f)).collect();
    let done: Vec<&String> = allowed.iter().filter(|a| !found.contains(a)).collect();
    assert!(new.is_empty(), "take a typed row instead (#161): {new:#?}");
    assert!(done.is_empty(), "converted — delete from STILL_POSITIONAL: {done:#?}");
}

#[test]
fn no_persistence_function_takes_eight_positional_parameters() {
    let mut eight = Vec::new();
    for (file, src) in store_sources() {
        for (name, n) in positional_counts(&src) {
            if n >= 8 {
                eight.push(format!("{file}::{name} ({n})"));
            }
        }
    }
    assert!(eight.is_empty(), "take a typed row (#161): {eight:#?}");
}

#[test]
fn the_scanner_counts_positional_parameters() {
    let src = "pub async fn a(&self, x: Option<(i32, i32)>, y: &[u8]) -> R {}\n\
               fn b<T: Into<String>>(mut self, p: T, q: fn(i32) -> i32, r: HashMap<K, V>) {}\n";
    assert_eq!(positional_counts(src), vec![("a".to_string(), 2), ("b".to_string(), 3)]);
}

// ── Each typed row lands field for field (#161) ────────────────────
//
// A row moves the field list out of the signature; the binds still map fields
// to columns by position. Every field gets a distinct value here, so a swapped
// bind reads back in the wrong column.

#[tokio::test]
async fn an_mcp_server_row_lands_field_for_field() {
    use crate::db::pg_store::McpServerRow;
    let s = super::pg_store().await;
    let args = serde_json::json!(["--a"]);
    let env = serde_json::json!({ "K": "v" });
    let row = McpServerRow {
        acp_family: "claude",
        mcp_key: "probe-key",
        workspace_id: None,
        config_source: "/cfg/mcp.json",
        command: "probe-cmd",
        args: &args,
        env: &env,
    };
    let id = s.upsert_mcp_server(&row).await.unwrap();
    let got: (
        String,
        String,
        String,
        Option<uuid::Uuid>,
        String,
        String,
        serde_json::Value,
        serde_json::Value,
    ) = sqlx_core::query_as::query_as(
        "SELECT acp_family, mcp_key, scope, workspace_id, config_source, command, args, env
               FROM sensei.mcp_servers WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        (
            "claude".into(),
            "probe-key".into(),
            "user".into(),
            None,
            "/cfg/mcp.json".into(),
            "probe-cmd".into(),
            args.clone(),
            env.clone()
        )
    );
    // A re-scan updates the same row in place.
    let again = s.upsert_mcp_server(&McpServerRow { command: "probe-cmd-2", ..row }).await.unwrap();
    assert_eq!(again, id);
}

#[tokio::test]
async fn an_mcp_manifest_row_lands_field_for_field() {
    use crate::db::pg_store::{McpManifestRow, McpServerRow};
    let s = super::pg_store().await;
    let (args, env) = (serde_json::json!([]), serde_json::json!({}));
    let id = s
        .upsert_mcp_server(&McpServerRow {
            acp_family: "claude",
            mcp_key: "manifest-key",
            workspace_id: None,
            config_source: "/cfg",
            command: "c",
            args: &args,
            env: &env,
        })
        .await
        .unwrap();
    let tools = serde_json::json!([{ "name": "t1" }]);
    s.upsert_mcp_tool_manifest(&McpManifestRow {
        server_id: &id,
        tools: &tools,
        tool_count: 3,
        protocol_version: Some("pv"),
        server_name: Some("sn"),
        server_version: Some("sv"),
        error: Some("err"),
    })
    .await
    .unwrap();
    type Manifest =
        (serde_json::Value, i32, Option<String>, Option<String>, Option<String>, Option<String>);
    let read = async || -> Manifest {
        sqlx_core::query_as::query_as(
            "SELECT tools, tool_count, protocol_version, server_name, server_version, error
               FROM sensei.mcp_tool_manifests WHERE server_id = $1",
        )
        .bind(id)
        .fetch_one(s.pool())
        .await
        .unwrap()
    };
    assert_eq!(
        read().await,
        (tools, 3, Some("pv".into()), Some("sn".into()), Some("sv".into()), Some("err".into()))
    );
    let none = serde_json::json!([]);
    s.upsert_mcp_tool_manifest(&McpManifestRow::failed(&id, &none, "down")).await.unwrap();
    assert_eq!(read().await, (none, 0, None, None, None, Some("down".into())));
}

#[tokio::test]
async fn an_assistant_tool_row_lands_field_for_field() {
    use crate::db::pg_store::AssistantToolRow;
    let s = super::pg_store().await;
    s.upsert_assistant_tool(&AssistantToolRow {
        assistant_family: "fam",
        source_type: "builtin",
        source_key: "skey",
        tool_name: "tname",
        invoked_name: "iname",
        description: Some("desc"),
        server_id: None,
    })
    .await
    .unwrap();
    let got: (String, String, String, String, String, Option<String>, Option<uuid::Uuid>) =
        sqlx_core::query_as::query_as(
            "SELECT assistant_family, source_type, source_key, tool_name, invoked_name, description, server_id
               FROM sensei.assistant_tools WHERE tool_name = 'tname'",
        )
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(
        got,
        (
            "fam".into(),
            "builtin".into(),
            "skey".into(),
            "tname".into(),
            "iname".into(),
            Some("desc".into()),
            None
        )
    );
}

#[tokio::test]
async fn a_check_run_row_lands_field_for_field() {
    use crate::db::pg_store::CheckRunRow;
    let s = super::pg_store().await;
    let root = s.add_watch_root("/_t161", "t161", &serde_json::json!([])).await.unwrap();
    let folder = s.upsert_repo_kind(&root, "git", "t161", "/_t161/repo").await.unwrap();
    s.insert_check_run(&CheckRunRow {
        folder_id: &folder,
        rule_statement: "stmt",
        checker_ref: "cref",
        command: "cmd",
        verdict: "fail",
        exit_code: Some(3),
        output_tail: "tail",
    })
    .await
    .unwrap();
    let got: (String, String, String, String, Option<i32>, String) = sqlx_core::query_as::query_as(
        "SELECT rule_statement, checker_ref, command, verdict::text, exit_code, output_tail
           FROM sensei.rule_check_runs WHERE folder_id = $1",
    )
    .bind(folder)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        ("stmt".into(), "cref".into(), "cmd".into(), "fail".into(), Some(3), "tail".into())
    );
}

#[tokio::test]
async fn a_consolidated_ruleset_row_lands_field_for_field() {
    use crate::db::pg_store::ConsolidatedRulesetRow;
    let s = super::pg_store().await;
    let conflicts = serde_json::json!([{ "a": 1 }]);
    let id = s
        .insert_consolidated_ruleset(&ConsolidatedRulesetRow {
            scope: "scope-161",
            version: 7,
            content: "body",
            conflicts: &conflicts,
            model: Some("mdl"),
            source_hash: "hash",
            status: "proposed",
        })
        .await
        .unwrap();
    let got: (String, i32, String, serde_json::Value, Option<String>, String, String) =
        sqlx_core::query_as::query_as(
            "SELECT scope, version, content, conflicts, model, source_hash, status
               FROM sensei.consolidated_rulesets WHERE id = $1",
        )
        .bind(id)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(
        got,
        (
            "scope-161".into(),
            7,
            "body".into(),
            conflicts,
            Some("mdl".into()),
            "hash".into(),
            "proposed".into()
        )
    );
}

#[tokio::test]
async fn a_workflow_state_patch_lands_field_for_field() {
    use crate::db::pg_store::WorkflowStatePatch;
    let s = super::pg_store().await;
    s.upsert_workflow_state(
        "ws-161",
        &WorkflowStatePatch {
            phase: Some("build"),
            plan: Some("plan.md"),
            task: Some("task"),
            issue: Some(161),
            checkpoint: Some("ckpt"),
            rules_hash: Some("rh"),
        },
    )
    .await
    .unwrap();
    type State = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i32>,
        Option<String>,
        Option<String>,
    );
    let got: State = sqlx_core::query_as::query_as(
        "SELECT active_phase, active_plan, active_task, active_issue, last_checkpoint, rules_hash
           FROM sensei.workflow_state WHERE workspace = 'ws-161'",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    let want = |s: &str| Some(s.to_string());
    assert_eq!(
        got,
        (want("build"), want("plan.md"), want("task"), Some(161), want("ckpt"), want("rh"))
    );
}

#[tokio::test]
async fn an_edge_row_lands_on_its_target_columns() {
    use crate::db::pg_store::NodeRow;
    use crate::db::pg_store::{EdgeRow, EdgeTarget};
    let s = super::pg_store().await;
    let root = s.add_watch_root("/_t161e", "t161e", &serde_json::json!([])).await.unwrap();
    let folder = s.upsert_repo_kind(&root, "git", "t161e", "/_t161e/repo").await.unwrap();
    s.upsert_file_row(
        &crate::db::pg_store::FileFingerprint {
            folder_id: &folder,
            file_path: "a.rs",
            mtime: 0,
            content_hash: "h",
        },
        None,
    )
    .await
    .unwrap();
    let node = |name: &'static str| NodeRow {
        folder_id: &folder,
        kind: "function",
        name,
        file_path: "a.rs",
        parent_id: None,
        signature: None,
        line_start: Some(1),
        line_end: Some(2),
        is_exported: false,
    };
    let (src, dst) =
        (s.upsert_node(&node("src")).await.unwrap(), s.upsert_node(&node("dst")).await.unwrap());
    let props = serde_json::json!({ "k": "v" });
    type Edge = (Option<uuid::Uuid>, Option<String>, Option<String>, String, serde_json::Value);
    let read = async |id: uuid::Uuid| -> Edge {
        sqlx_core::query_as::query_as(
            "SELECT target_id, target_name, target_file, kind::text, props FROM sensei.edges WHERE id = $1",
        )
        .bind(id)
        .fetch_one(s.pool())
        .await
        .unwrap()
    };

    let resolved = s
        .insert_edge_with_props(&EdgeRow {
            folder_id: &folder,
            source_id: &src,
            target: EdgeTarget::Node(&dst),
            kind: "calls",
            props: &props,
        })
        .await
        .unwrap();
    assert_eq!(read(resolved).await, (Some(dst), None, None, "calls".into(), props.clone()));

    let unresolved = s
        .insert_edge_with_props(&EdgeRow {
            folder_id: &folder,
            source_id: &src,
            target: EdgeTarget::Unresolved { name: Some("ghost"), file: Some("g.rs") },
            kind: "calls",
            props: &props,
        })
        .await
        .unwrap();
    assert_eq!(
        read(unresolved).await,
        (None, Some("ghost".into()), Some("g.rs".into()), "calls".into(), props)
    );
}

#[tokio::test]
async fn a_folder_row_lands_field_for_field() {
    use crate::db::pg_store::FolderRow;
    let s = super::pg_store().await;
    let root = s.add_watch_root("/_t161f", "t161f", &serde_json::json!([])).await.unwrap();
    let parent = s.upsert_repo_kind(&root, "git", "t161f", "/_t161f/repo").await.unwrap();
    let id = s
        .upsert_folder(&FolderRow {
            root_id: &root,
            kind: "module",
            name: "fname",
            path: "fpath",
            abs_path: "/_t161f/repo/fpath",
            parent_id: Some(&parent),
            workspace_root_id: Some(&parent),
        })
        .await
        .unwrap();
    type Folder =
        (uuid::Uuid, String, String, String, String, Option<uuid::Uuid>, Option<uuid::Uuid>);
    let got: Folder = sqlx_core::query_as::query_as(
        "SELECT root_id, kind::text, name, path, abs_path, parent_id, workspace_root_id
           FROM sensei.folders WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        (
            root,
            "module".into(),
            "fname".into(),
            "fpath".into(),
            "/_t161f/repo/fpath".into(),
            Some(parent),
            Some(parent)
        )
    );
}

#[tokio::test]
async fn a_task_execution_start_lands_field_for_field() {
    use crate::db::pg_store::TaskExecutionStart;
    let s = super::pg_store().await;
    let id = s
        .start_task_execution(&TaskExecutionStart {
            task_id: 41,
            parent_task_id: Some(40),
            task_kind: "process_file",
            folder_path: "/fp",
            path: "src/p.rs",
            retry_number: 2,
        })
        .await
        .unwrap();
    type Exec = (i64, Option<i64>, String, String, String, String, i32);
    let got: Exec = sqlx_core::query_as::query_as(
        "SELECT task_id, parent_task_id, task_kind::text, folder_path, path, status, retry_number
           FROM activity.task_executions WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        (41, Some(40), "process_file".into(), "/fp".into(), "src/p.rs".into(), "running".into(), 2)
    );
}

#[tokio::test]
async fn an_extension_row_lands_field_for_field() {
    use crate::db::pg_store::ExtensionRow;
    let s = super::pg_store().await;
    let id = s
        .create_extension(&ExtensionRow {
            kind: "agent",
            name: "ext-161",
            description: Some("desc"),
            content: Some("body"),
            scope: "workspace",
            source: "marketplace",
        })
        .await
        .unwrap();
    type Ext = (String, String, Option<String>, Option<String>, String, String);
    let got: Ext = sqlx_core::query_as::query_as(
        "SELECT kind::text, name, description, content, scope::text, source::text
           FROM sensei.extensions WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        (
            "agent".into(),
            "ext-161".into(),
            Some("desc".into()),
            Some("body".into()),
            "workspace".into(),
            "marketplace".into()
        )
    );
}

#[tokio::test]
async fn a_transcript_source_lands_on_every_turn() {
    use crate::db::pg_store::TranscriptSource;
    let s = super::pg_store().await;
    let turn = crate::transcript::TranscriptTurn {
        turn_index: 1,
        user_text: Some("hi".into()),
        assistant_text: "hello".into(),
        started_at: None,
        attrs: serde_json::json!({}),
        facts: Default::default(),
    };
    s.upsert_transcript_turns(
        &TranscriptSource {
            source: "zed",
            session_id: "sess-161",
            family: "fam",
            provider: Some("prov"),
            model: Some("mdl"),
        },
        &[turn],
    )
    .await
    .unwrap();
    type Turn = (String, String, Option<String>, Option<String>);
    let got: Turn = sqlx_core::query_as::query_as(
        "SELECT source, family, provider, model FROM activity.transcript_turns WHERE session_id = 'sess-161'",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, ("zed".into(), "fam".into(), Some("prov".into()), Some("mdl".into())));
}

#[tokio::test]
async fn a_pattern_row_lands_field_for_field() {
    use crate::db::pg_store::PatternRow;
    let s = super::pg_store().await;
    let ws = s.create_workspace("_t161p", None, None).await.unwrap();
    let root = s.add_watch_root("/_t161p", "t161p", &serde_json::json!([])).await.unwrap();
    let folder = s.upsert_repo_kind(&root, "git", "t161p", "/_t161p/repo").await.unwrap();
    let instances = serde_json::json!([{ "file": "a.rs" }, { "file": "b.rs" }]);
    let id = s
        .upsert_pattern(&PatternRow {
            workspace_id: &ws,
            folder_id: Some(&folder),
            name: "pat-161",
            is_anti: true,
            confidence: Some(0.75),
            instances: &instances,
        })
        .await
        .unwrap();
    type Pattern =
        (uuid::Uuid, Option<uuid::Uuid>, String, bool, Option<f64>, i32, serde_json::Value);
    let got: Pattern = sqlx_core::query_as::query_as(
        "SELECT workspace_id, folder_id, name, is_anti_pattern, confidence::float8, instance_count, instances
           FROM inference.detected_patterns WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, (ws, Some(folder), "pat-161".into(), true, Some(0.75), 2, instances));
}

#[tokio::test]
async fn a_library_row_lands_on_the_library_and_its_release() {
    use crate::db::pg_store::LibraryRow;
    let s = super::pg_store().await;
    let id = s
        .upsert_library(&LibraryRow {
            name: "lib-161",
            ecosystem: "cargo",
            version: Some("1.2.3"),
            description: Some("desc"),
            source_type: Some("llms.txt"),
            base_url: Some("https://example.test/llms.txt"),
        })
        .await
        .unwrap();
    let lib: (String, String, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT name, ecosystem::text, description FROM sensei.libraries WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(lib, ("lib-161".into(), "cargo".into(), Some("desc".into())));
    let release: (Option<String>, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT version, source_type::text, base_url FROM sensei.library_versions WHERE library_id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        release,
        (
            Some("1.2.3".into()),
            Some("llms.txt".into()),
            Some("https://example.test/llms.txt".into())
        )
    );
}

#[tokio::test]
async fn a_session_metadata_patch_lands_field_for_field() {
    use crate::db::pg_store::SessionMetadataPatch;
    let s = super::pg_store().await;
    sqlx_core::query::query("INSERT INTO activity.sessions (client_session_id) VALUES ('sm-161')")
        .execute(s.pool())
        .await
        .unwrap();
    let split = crate::transcript::SessionTokens {
        input: 11,
        output: 12,
        cache_read: Some(13),
        cache_write: Some(14),
        reasoning: Some(15),
        cost: Some(0.5),
    };
    s.set_session_metadata(
        "sm-161",
        &SessionMetadataPatch {
            provider: Some("prov"),
            model: Some("mdl"),
            tokens_in: Some(100),
            tokens_out: Some(200),
            split: Some(&split),
        },
    )
    .await
    .unwrap();
    type Meta = (
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    );
    let got: Meta = sqlx_core::query_as::query_as(
        "SELECT provider, model, tokens_in, tokens_out, tokens_fresh, cache_read, cache_write, tokens_reasoning
           FROM activity.sessions WHERE client_session_id = 'sm-161'",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        (
            Some("prov".into()),
            Some("mdl".into()),
            Some(100),
            Some(200),
            Some(11),
            Some(13),
            Some(14),
            Some(15)
        )
    );
}

#[tokio::test]
async fn a_snapshot_row_lands_field_for_field() {
    use crate::db::pg_store::SnapshotRow;
    let s = super::pg_store().await;
    let root = s.add_watch_root("/_t161s", "t161s", &serde_json::json!([])).await.unwrap();
    let folder = s.upsert_repo_kind(&root, "git", "t161s", "/_t161s/repo").await.unwrap();
    let session = s.create_session(&folder, "task", None).await.unwrap();
    let steps = vec!["a".to_string(), "b".to_string()];
    let id = s
        .create_snapshot(&SnapshotRow {
            session_id: &session,
            folder_id: &folder,
            kind: "checkpoint",
            progress: "half",
            next_step: Some("next"),
            completed_steps: &steps,
        })
        .await
        .unwrap();
    type Snap = (uuid::Uuid, uuid::Uuid, String, Option<String>, Option<String>, Vec<String>);
    let got: Snap = sqlx_core::query_as::query_as(
        "SELECT session_id, folder_id, kind::text, progress_summary, next_step_hint, completed_steps
           FROM activity.snapshots WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        (session, folder, "checkpoint".into(), Some("half".into()), Some("next".into()), steps)
    );
}

#[tokio::test]
async fn a_reasoning_trace_row_lands_field_for_field() {
    use crate::db::pg_store::ReasoningTraceRow;
    let s = super::pg_store().await;
    let ws = s.create_workspace("_t161r", None, None).await.unwrap();
    let (detail, ex, cons) =
        (serde_json::json!({ "d": 1 }), serde_json::json!([1]), serde_json::json!({ "c": 2 }));
    let models = vec!["m1".to_string()];
    let id = s
        .insert_reasoning_trace(&ReasoningTraceRow {
            workspace_id: Some(&ws),
            trigger_event: "trig",
            trigger_detail: &detail,
            models_used: &models,
            exchanges: &ex,
            consensus: &cons,
        })
        .await
        .unwrap();
    type Trace = (
        Option<uuid::Uuid>,
        String,
        serde_json::Value,
        Vec<String>,
        serde_json::Value,
        serde_json::Value,
    );
    let got: Trace = sqlx_core::query_as::query_as(
        "SELECT workspace_id, trigger_event, trigger_detail, models_used, exchanges, consensus
           FROM inference.reasoning_traces WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, (Some(ws), "trig".into(), detail, models, ex, cons));
}

#[tokio::test]
async fn a_benchmark_report_row_lands_field_for_field() {
    use crate::db::pg_store::BenchmarkReportRow;
    let s = super::pg_store().await;
    let id = s
        .create_benchmark_report(&BenchmarkReportRow {
            folder_id: None,
            run_name: "run",
            strategy: "strat",
            score: Some(0.25),
            tokens: Some(7),
            elapsed_ms: Some(9),
        })
        .await
        .unwrap();
    type Report = (Option<uuid::Uuid>, String, String, Option<f64>, Option<i32>, Option<i32>);
    let got: Report = sqlx_core::query_as::query_as(
        "SELECT folder_id, run_name, strategy, score::float8, tokens, elapsed_ms
           FROM sensei.benchmark_reports WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, (None, "run".into(), "strat".into(), Some(0.25), Some(7), Some(9)));
}

#[tokio::test]
async fn an_outbox_sent_row_lands_field_for_field() {
    use crate::db::pg_store::OutboxSentRow;
    let s = super::pg_store().await;
    let mid = uuid::Uuid::new_v4();
    s.create_dojo_membership(&crate::db::pg_store::NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/t161".into(),
        dojo_url: "http://localhost:7755/github/t161".into(),
        kind: "client".into(),
        org_slugs: vec!["t161".into()],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();
    s.outbox_mark_sent(&OutboxSentRow {
        membership_id: &mid,
        batch_id: None,
        memory_id: None,
        signature: "sig",
        sent_seq: 42,
        remote_id: "rid",
    })
    .await
    .unwrap();
    type Sent = (String, String, Option<i64>, Option<String>);
    let got: Sent = sqlx_core::query_as::query_as(
        "SELECT signature, state::text, sent_seq, remote_id FROM sensei.dojo_outbox WHERE membership_id = $1",
    )
    .bind(mid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, ("sig".into(), "sent".into(), Some(42), Some("rid".into())));
}

#[tokio::test]
async fn a_narration_row_lands_field_for_field() {
    use crate::db::pg_store::NarrationRow;
    let s = super::pg_store().await;
    s.upsert_narration_cache(&NarrationRow {
        kind: "kind-161",
        facts_hash: "fh",
        title: "ttl",
        detail: "dtl",
        model_provider: Some("prov"),
        model_id: Some("mid"),
    })
    .await;
    type Cached = (String, String, String, String, Option<String>, Option<String>);
    let got: Cached = sqlx_core::query_as::query_as(
        "SELECT kind, facts_hash, title, detail, model_provider, model_id
           FROM sensei.narration_cache WHERE kind = 'kind-161'",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        (
            "kind-161".into(),
            "fh".into(),
            "ttl".into(),
            "dtl".into(),
            Some("prov".into()),
            Some("mid".into())
        )
    );
}

/// Each override reaches its own column: the title replaces the rule's title,
/// the body its content. Every caller in the tree passes `None` for both, so
/// this is the only check that the two are not crossed.
#[tokio::test]
async fn a_rule_acceptance_puts_each_override_in_its_own_column() {
    use crate::db::pg_store::RuleAcceptance;
    let s = super::pg_store().await;
    let ws = s.create_workspace("_t161a", None, None).await.unwrap();
    let rid = s
        .create_recommendation_full(&crate::db::pg_store::RecommendationRow {
            workspace_id: &ws,
            title: "rec title",
            why: "rec why",
            impact: None,
            action_type: "revise_rule",
            urgency: "high",
            based_on: &serde_json::json!({}),
            reasoning_trace_id: None,
            prompt: None,
        })
        .await
        .unwrap();
    let mat = s
        .accept_recommendation_as_rule(
            &rid,
            &RuleAcceptance {
                namespace_id: None,
                enforcement: Some("required"),
                gov_scope: "general",
                title_override: Some("override title"),
                body_override: Some("override body"),
            },
        )
        .await
        .unwrap();
    let mem = uuid::Uuid::parse_str(mat["memory_id"].as_str().unwrap()).unwrap();
    let got: (String, String, String) = sqlx_core::query_as::query_as(
        "SELECT title, content, enforcement::text FROM sensei.memories WHERE id = $1",
    )
    .bind(mem)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, ("override title".into(), "override body".into(), "required".into()));
}

#[tokio::test]
async fn a_subfolder_row_lands_field_for_field() {
    use crate::db::pg_store::SubfolderRow;
    let s = super::pg_store().await;
    let root = s.add_watch_root("/_t161sf", "t161sf", &serde_json::json!([])).await.unwrap();
    let repo = s.upsert_repo_kind(&root, "git", "t161sf", "/_t161sf/repo").await.unwrap();
    let id = s
        .upsert_subfolder(&SubfolderRow {
            root_id: &root,
            kind: "module",
            name: "sub",
            path: "sub/path",
            abs_path: "/_t161sf/repo/sub/path",
            parent_id: Some(&repo),
        })
        .await
        .unwrap();
    type Sub = (uuid::Uuid, String, String, String, String, Option<uuid::Uuid>);
    let got: Sub = sqlx_core::query_as::query_as(
        "SELECT root_id, kind::text, name, path, abs_path, parent_id FROM sensei.folders WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        (
            root,
            "module".into(),
            "sub".into(),
            "sub/path".into(),
            "/_t161sf/repo/sub/path".into(),
            Some(repo)
        )
    );
}

#[tokio::test]
async fn a_capture_watermark_row_lands_field_for_field() {
    use crate::db::pg_store::CaptureWatermarkRow;
    let s = super::pg_store().await;
    s.set_capture_watermark(&CaptureWatermarkRow {
        source: "zed",
        file_path: "/t/161.jsonl",
        session_id: Some("sess"),
        mtime_ns: 77,
        turns: 5,
    })
    .await
    .unwrap();
    let got: (Option<String>, i64, i32, bool) = sqlx_core::query_as::query_as(
        "SELECT session_id, last_mtime_ns, turns_ingested, untracked FROM activity.capture_watermarks
          WHERE source = 'zed' AND file_path = '/t/161.jsonl'",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, (Some("sess".into()), 77, 5, false));
}

#[tokio::test]
async fn an_outbox_state_row_lands_field_for_field() {
    use crate::db::pg_store::OutboxStateRow;
    let s = super::pg_store().await;
    let mid = uuid::Uuid::new_v4();
    s.create_dojo_membership(&crate::db::pg_store::NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/t161o".into(),
        dojo_url: "http://localhost:7755/github/t161o".into(),
        kind: "client".into(),
        org_slugs: vec!["t161o".into()],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();
    s.outbox_mark_state(&OutboxStateRow {
        membership_id: &mid,
        batch_id: None,
        memory_id: None,
        signature: "sig-o",
        state: "queued",
    })
    .await
    .unwrap();
    let got: (String, String) = sqlx_core::query_as::query_as(
        "SELECT signature, state::text FROM sensei.dojo_outbox WHERE membership_id = $1",
    )
    .bind(mid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, ("sig-o".into(), "queued".into()));
}

#[tokio::test]
async fn a_service_row_lands_field_for_field() {
    use crate::db::pg_store::ServiceRow;
    let s = super::pg_store().await;
    let config = serde_json::json!({ "k": 1 });
    let id = s
        .upsert_service(&ServiceRow {
            name: "svc-161",
            display_name: "Service 161",
            kind: "inference",
            protocol: "ollama",
            config: &config,
        })
        .await
        .unwrap();
    type Svc = (String, String, String, String, serde_json::Value);
    let got: Svc = sqlx_core::query_as::query_as(
        "SELECT name, display_name, kind::text, protocol::text, config FROM sensei.services WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        ("svc-161".into(), "Service 161".into(), "inference".into(), "ollama".into(), config)
    );
}

#[tokio::test]
async fn a_workspace_identity_lands_field_for_field() {
    use crate::db::pg_store::WorkspaceIdentity;
    let s = super::pg_store().await;
    let ws = s.create_workspace("_t161w", None, None).await.unwrap();
    let (stack, tags) = (vec!["rust".to_string()], vec!["t1".to_string()]);
    s.set_workspace_identity(
        &ws,
        &WorkspaceIdentity {
            description: Some("desc"),
            client: Some("cl"),
            stack: &stack,
            tags: &tags,
        },
    )
    .await
    .unwrap();
    type Ident = (Option<String>, Option<String>, serde_json::Value, Vec<String>);
    let got: Ident = sqlx_core::query_as::query_as(
        "SELECT description, client, stack, tags FROM sensei.workspaces WHERE id = $1",
    )
    .bind(ws)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, (Some("desc".into()), Some("cl".into()), serde_json::json!(["rust"]), tags));
}

#[tokio::test]
async fn a_file_fingerprint_lands_field_for_field() {
    use crate::db::pg_store::FileFingerprint;
    let s = super::pg_store().await;
    let root = s.add_watch_root("/_t161ff", "t161ff", &serde_json::json!([])).await.unwrap();
    let folder = s.upsert_repo_kind(&root, "git", "t161ff", "/_t161ff/repo").await.unwrap();
    let fp = FileFingerprint {
        folder_id: &folder,
        file_path: "src/a.rs",
        mtime: 1234,
        content_hash: "h-161",
    };
    s.upsert_file_row(&fp, Some(crate::classifiers::ScanSkipReason::Oversized)).await.unwrap();
    type File = (i64, String, Option<String>);
    let got: File = sqlx_core::query_as::query_as(
        "SELECT mtime, content_hash, skip_reason::text FROM sensei.files WHERE folder_id = $1 AND file_path = 'src/a.rs'",
    )
    .bind(folder)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, (1234, "h-161".into(), Some("oversized".into())));
}

/// The source and version range land on every content row the capabilities
/// write. Nothing else read them back: forcing `source` passed every
/// capability test.
#[tokio::test]
async fn library_capabilities_land_with_their_source_and_range() {
    use crate::db::pg_store::{LibraryCapabilities, LibraryRow};
    use crate::libraries::manifest::{ProvidedAgent, ProvidedSkill};
    let s = super::pg_store().await;
    let lib = s
        .upsert_library(&LibraryRow {
            name: "caps-161",
            ecosystem: "cargo",
            version: None,
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
    let skills = vec![ProvidedSkill {
        name: "sk".into(),
        focus: "f".into(),
        path: None,
        body: Some("b".into()),
    }];
    let agents = vec![ProvidedAgent {
        name: "ag".into(),
        focus: "f".into(),
        path: None,
        body: Some("b".into()),
    }];
    s.replace_library_capabilities(
        &lib,
        &LibraryCapabilities {
            source: "manifest",
            version_range: Some(">=1"),
            skills: &skills,
            agents: &agents,
        },
    )
    .await
    .unwrap();
    let got: Vec<(String, String, String, Option<String>)> = sqlx_core::query_as::query_as(
        "SELECT c.kind::text, c.name, c.source::text, c.version_range
           FROM sensei.library_content c JOIN sensei.library_versions v ON v.id = c.library_version_id
          WHERE v.library_id = $1 ORDER BY c.kind::text, c.name",
    )
    .bind(lib)
    .fetch_all(s.pool())
    .await
    .unwrap();
    assert_eq!(
        got,
        vec![
            ("agent".into(), "ag".into(), "manifest".into(), Some(">=1".into())),
            ("skill".into(), "sk".into(), "manifest".into(), Some(">=1".into())),
        ]
    );
}

#[tokio::test]
async fn an_index_error_row_lands_field_for_field() {
    use crate::db::pg_store::IndexErrorRow;
    let s = super::pg_store().await;
    let root = s.add_watch_root("/_t161ie", "t161ie", &serde_json::json!([])).await.unwrap();
    let folder = s.upsert_repo_kind(&root, "git", "t161ie", "/_t161ie/repo").await.unwrap();
    s.log_index_error(&IndexErrorRow {
        folder_id: &folder,
        file_path: "src/x.rs",
        error: "boom",
        adapter: Some("rust"),
        phase: Some("parse"),
    })
    .await
    .unwrap();
    let got: (String, String, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT file_path, error, adapter, phase FROM sensei.index_errors WHERE folder_id = $1",
    )
    .bind(folder)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(got, ("src/x.rs".into(), "boom".into(), Some("rust".into()), Some("parse".into())));
}
