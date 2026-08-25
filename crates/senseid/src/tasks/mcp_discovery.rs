//! #84 T2 Slice A — Discover MCP servers from ACP config files and persist
//! them into `sensei.mcp_servers`.
//!
//! Scans (per platform):
//!
//! - **Claude Code** — `~/.claude/mcp.json` (user), each project's
//!   `.mcp.json` (project).
//! - **Zed** — `~/.config/zed/settings.json` (user) — the `context_servers`
//!   block. Project-scope Zed config lives in `.zed/settings.json`.
//! - **Cursor** — `~/.cursor/mcp.json` (user), `<project>/.cursor/mcp.json`
//!   (project).
//! - **Codex / OpenCode** — path convention varies; probed when the ACP is
//!   detected. Skipped on machines that don't have the ACP installed.
//!
//! The scan is idempotent — [`PgStore::upsert_mcp_server`] preserves the
//! `enabled` field on repeat runs, so a user's manual disable survives.
//! Stale rows (servers no longer in any config) are pruned at the end via
//! [`PgStore::prune_stale_mcp_servers`] against the scan-start timestamp.
//!
//! Live-probing "is the server actually reachable?" is separate — this task
//! only ingests configured presence. Connection-state probing lands in a
//! follow-up when the Playground tab needs it.

use crate::db::pg_store::PgStore;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Every ACP family the discovery task knows about. Codex + OpenCode are
/// declared but not yet scanned — path conventions vary across those
/// platforms, so they get their own scan pass when the ACP is detected
/// (follow-up commit). Kept in the enum so callers can key on them once
/// the scanners exist without a breaking ecosystem rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Codex + OpenCode variants land with their scanners
pub enum AcpFamily {
    Claude,
    Zed,
    Cursor,
    Codex,
    OpenCode,
}

impl AcpFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            AcpFamily::Claude => "claude",
            AcpFamily::Zed => "zed",
            AcpFamily::Cursor => "cursor",
            AcpFamily::Codex => "codex",
            AcpFamily::OpenCode => "opencode",
        }
    }
}

/// One MCP entry pulled from a config file, ready to upsert.
#[derive(Debug, Clone)]
pub struct DiscoveredMcp {
    pub acp_family: AcpFamily,
    pub mcp_key: String,
    pub project_id: Option<uuid::Uuid>,
    pub config_source: PathBuf,
    pub command: String,
    pub args: Value,
    pub env: Value,
}

/// Parse an already-loaded JSON value as an ACP config's MCP section. The
/// wrapper key varies: Claude Code + Cursor use `"mcpServers"`, Zed uses
/// `"context_servers"`. Returns a flat vec of discovered entries for a
/// given `acp_family` + `wrapper_key`, or empty when the wrapper isn't
/// present.
pub fn parse_mcp_section(
    doc: &Value,
    wrapper_key: &str,
    acp_family: AcpFamily,
    project_id: Option<uuid::Uuid>,
    config_source: &Path,
) -> Vec<DiscoveredMcp> {
    let Some(map) = doc.get(wrapper_key).and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    map.iter()
        .map(|(key, entry)| {
            let command = entry.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let args = entry.get("args").cloned().unwrap_or(Value::Array(vec![]));
            let env = entry.get("env").cloned().unwrap_or(Value::Object(serde_json::Map::new()));
            DiscoveredMcp {
                acp_family,
                mcp_key: key.clone(),
                project_id,
                config_source: config_source.to_path_buf(),
                command,
                args,
                env,
            }
        })
        .collect()
}

/// Read a JSON-or-JSONC file at `path`. Returns `None` if the file doesn't
/// exist or fails to parse. Uses json5 so `.claude/settings.json`-style
/// files with `//` comments still parse (same as the assistants config
/// helpers post-#51).
pub fn read_jsonc(path: &Path) -> Option<Value> {
    if !path.exists() {
        return None;
    }
    let s = std::fs::read_to_string(path).ok()?;
    json5::from_str::<Value>(&s).ok()
}

/// Scan a user's home directory for known ACP MCP config files. Returns
/// every entry found. `home_dir` is passed in so tests can point at a temp
/// dir instead of the real `$HOME`.
pub fn discover_user_scope(home_dir: &Path) -> Vec<DiscoveredMcp> {
    let mut out = Vec::new();

    // Claude Code: ~/.claude/mcp.json wrapping "mcpServers".
    let claude_mcp = home_dir.join(".claude").join("mcp.json");
    if let Some(doc) = read_jsonc(&claude_mcp) {
        out.extend(parse_mcp_section(&doc, "mcpServers", AcpFamily::Claude, None, &claude_mcp));
    }

    // Cursor: ~/.cursor/mcp.json wrapping "mcpServers".
    let cursor_mcp = home_dir.join(".cursor").join("mcp.json");
    if let Some(doc) = read_jsonc(&cursor_mcp) {
        out.extend(parse_mcp_section(&doc, "mcpServers", AcpFamily::Cursor, None, &cursor_mcp));
    }

    // Zed: ~/.config/zed/settings.json wrapping "context_servers".
    let zed_settings = home_dir.join(".config").join("zed").join("settings.json");
    if let Some(doc) = read_jsonc(&zed_settings) {
        out.extend(parse_mcp_section(&doc, "context_servers", AcpFamily::Zed, None, &zed_settings));
    }

    out
}

/// Scan a project's root directory for project-scope ACP config files.
/// `project_root` is the project's absolute path; `project_id` is the
/// sensei projects.id UUID.
pub fn discover_project_scope(project_root: &Path, project_id: uuid::Uuid) -> Vec<DiscoveredMcp> {
    let mut out = Vec::new();

    // Claude Code project-scope: <project>/.mcp.json wrapping "mcpServers".
    let claude_project = project_root.join(".mcp.json");
    if let Some(doc) = read_jsonc(&claude_project) {
        out.extend(parse_mcp_section(
            &doc,
            "mcpServers",
            AcpFamily::Claude,
            Some(project_id),
            &claude_project,
        ));
    }

    // Cursor project-scope: <project>/.cursor/mcp.json.
    let cursor_project = project_root.join(".cursor").join("mcp.json");
    if let Some(doc) = read_jsonc(&cursor_project) {
        out.extend(parse_mcp_section(
            &doc,
            "mcpServers",
            AcpFamily::Cursor,
            Some(project_id),
            &cursor_project,
        ));
    }

    // Zed project-scope: <project>/.zed/settings.json.
    let zed_project = project_root.join(".zed").join("settings.json");
    if let Some(doc) = read_jsonc(&zed_project) {
        out.extend(parse_mcp_section(
            &doc,
            "context_servers",
            AcpFamily::Zed,
            Some(project_id),
            &zed_project,
        ));
    }

    out
}

/// Run one discovery pass — scan user + project configs, upsert, prune stale.
/// Returns `(discovered, pruned)` counts. `home_dir` and `project_roots`
/// are injected so tests can point at temp dirs.
pub async fn run_once(
    pg: &PgStore,
    home_dir: &Path,
    project_roots: &[(uuid::Uuid, PathBuf)],
) -> Result<(usize, u64), String> {
    // Grab a scan-start timestamp so we can prune anything that isn't
    // refreshed by this pass (with a small safety margin so an in-flight
    // upsert isn't collateral).
    let scan_start = chrono::Utc::now() - chrono::Duration::seconds(1);

    let mut entries = discover_user_scope(home_dir);
    for (project_id, root) in project_roots {
        entries.extend(discover_project_scope(root, *project_id));
    }

    let discovered = entries.len();
    for e in &entries {
        pg.upsert_mcp_server(
            e.acp_family.as_str(),
            &e.mcp_key,
            e.project_id,
            &e.config_source.to_string_lossy(),
            &e.command,
            &e.args,
            &e.env,
        )
        .await?;
    }

    let pruned = pg.prune_stale_mcp_servers(scan_start).await?;
    Ok((discovered, pruned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn parse_mcp_section_pulls_command_args_env() {
        let doc = json!({
            "mcpServers": {
                "sensei":   { "command": "sensei-mcp" },
                "postgres": { "command": "pg-mcp", "args": ["--url", "postgresql://localhost/x"], "env": { "PGUSER": "u" } }
            }
        });
        let out = parse_mcp_section(&doc, "mcpServers", AcpFamily::Claude, None, Path::new("/x"));
        assert_eq!(out.len(), 2);
        let sensei = out.iter().find(|d| d.mcp_key == "sensei").unwrap();
        assert_eq!(sensei.command, "sensei-mcp");
        assert_eq!(sensei.args, json!([]));
        assert_eq!(sensei.env, json!({}));
        let pg = out.iter().find(|d| d.mcp_key == "postgres").unwrap();
        assert_eq!(pg.command, "pg-mcp");
        assert_eq!(pg.args, json!(["--url", "postgresql://localhost/x"]));
        assert_eq!(pg.env, json!({ "PGUSER": "u" }));
    }

    #[test]
    fn parse_mcp_section_empty_when_wrapper_missing() {
        let doc = json!({ "settings": { "font_size": 14 } });
        let out = parse_mcp_section(&doc, "mcpServers", AcpFamily::Zed, None, Path::new("/x"));
        assert!(out.is_empty());
    }

    #[test]
    fn read_jsonc_tolerates_comments() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("mcp.json");
        std::fs::write(
            &p,
            r#"// Zed settings
{
  "mcpServers": {
    // sensei registered by daemon
    "sensei": { "command": "sensei-mcp" }
  }
}"#,
        )
        .unwrap();
        let doc = read_jsonc(&p).expect("json5 must parse the JSONC");
        assert_eq!(doc["mcpServers"]["sensei"]["command"], "sensei-mcp");
    }

    #[test]
    fn read_jsonc_returns_none_for_missing_file() {
        let dir = tempdir().unwrap();
        assert!(read_jsonc(&dir.path().join("does-not-exist.json")).is_none());
    }

    #[test]
    fn discover_user_scope_finds_claude_cursor_zed() {
        let home = tempdir().unwrap();

        // Claude
        let claude_dir = home.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("mcp.json"),
            r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"}}}"#,
        )
        .unwrap();

        // Cursor
        let cursor_dir = home.path().join(".cursor");
        std::fs::create_dir_all(&cursor_dir).unwrap();
        std::fs::write(
            cursor_dir.join("mcp.json"),
            r#"{"mcpServers":{"postgres":{"command":"pg-mcp"}}}"#,
        )
        .unwrap();

        // Zed (uses "context_servers", not "mcpServers")
        let zed_dir = home.path().join(".config").join("zed");
        std::fs::create_dir_all(&zed_dir).unwrap();
        std::fs::write(
            zed_dir.join("settings.json"),
            r#"{"context_servers":{"svelte":{"command":"svelte-mcp"}}}"#,
        )
        .unwrap();

        let discovered = discover_user_scope(home.path());
        assert_eq!(discovered.len(), 3);
        assert!(
            discovered.iter().any(|d| d.acp_family == AcpFamily::Claude && d.mcp_key == "sensei")
        );
        assert!(
            discovered.iter().any(|d| d.acp_family == AcpFamily::Cursor && d.mcp_key == "postgres")
        );
        assert!(discovered.iter().any(|d| d.acp_family == AcpFamily::Zed && d.mcp_key == "svelte"));
    }

    #[test]
    fn discover_project_scope_finds_dot_mcp_json() {
        let project_root = tempdir().unwrap();
        std::fs::write(
            project_root.path().join(".mcp.json"),
            r#"{"mcpServers":{"local-tool":{"command":"./bin/tool"}}}"#,
        )
        .unwrap();

        let pid = uuid::Uuid::new_v4();
        let discovered = discover_project_scope(project_root.path(), pid);
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].mcp_key, "local-tool");
        assert_eq!(discovered[0].project_id, Some(pid));
        assert_eq!(discovered[0].acp_family, AcpFamily::Claude);
    }

    #[test]
    fn discover_scopes_return_empty_when_no_config_files_exist() {
        let dir = tempdir().unwrap();
        assert!(discover_user_scope(dir.path()).is_empty());
        assert!(discover_project_scope(dir.path(), uuid::Uuid::new_v4()).is_empty());
    }

    #[test]
    fn acp_family_as_str_matches_ddl_check_values() {
        // The mcp_servers DDL doesn't CHECK acp_family, but callers key on
        // these strings — a rename here would silently break the UI join.
        assert_eq!(AcpFamily::Claude.as_str(), "claude");
        assert_eq!(AcpFamily::Zed.as_str(), "zed");
        assert_eq!(AcpFamily::Cursor.as_str(), "cursor");
        assert_eq!(AcpFamily::Codex.as_str(), "codex");
        assert_eq!(AcpFamily::OpenCode.as_str(), "opencode");
    }
}
