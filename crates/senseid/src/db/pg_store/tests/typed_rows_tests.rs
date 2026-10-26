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
    "config.rs::upsert_workflow_state",
    "dojo.rs::outbox_mark_sent",
    "dojo.rs::outbox_mark_state",
    "dojo.rs::upsert_federated_memory",
    "extensions.rs::create_extension",
    "folders.rs::upsert_file_row",
    "folders.rs::upsert_folder",
    "folders.rs::upsert_scan_state_skipped",
    "folders.rs::upsert_subfolder",
    "folders.rs::upsert_subfolder_kind",
    "folders.rs::write_scan_state",
    "governance.rs::assemble_context",
    "governance.rs::insert_check_run",
    "governance.rs::insert_consolidated_ruleset",
    "governance.rs::upsert_stance",
    "graph.rs::callee_row",
    "graph.rs::insert_edge",
    "graph.rs::insert_edge_with_props",
    "graph.rs::merge_function",
    "graph.rs::merge_type",
    "graph.rs::semantic_search_nodes",
    "graph.rs::upsert_lib_node_by_fqn",
    "graph.rs::upsert_node_by_fqn",
    "library.rs::replace_library_capabilities",
    "library.rs::upsert_folder_dependency",
    "library.rs::upsert_library",
    "logs.rs::log_index_error",
    "logs.rs::query_logs",
    "logs.rs::start_task_execution",
    "mcp.rs::upsert_service",
    "metrics.rs::create_benchmark_report",
    "patterns.rs::accept_recommendation_as_rule",
    "patterns.rs::create_recommendation",
    "patterns.rs::insert_reasoning_trace",
    "patterns.rs::upsert_narration_cache",
    "patterns.rs::upsert_pattern",
    "personas.rs::link_persona_identity",
    "playbook.rs::playbook_combo_trust",
    "runs.rs::append_run_event",
    "sessions.rs::create_snapshot",
    "sessions.rs::record_session_event",
    "sessions.rs::set_session_metadata",
    "trackers.rs::apply_discovered",
    "trackers.rs::apply_tracker_snapshot",
    "transcript.rs::set_capture_watermark",
    "transcript.rs::upsert_transcript_turns",
    "workspaces.rs::set_workspace_identity",
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
