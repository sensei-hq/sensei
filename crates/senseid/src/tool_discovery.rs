//! Per-assistant tool discovery + capture into the unified `sensei.assistant_tools`
//! inventory (the registry half of the Instruments · Health share grid).
//!
//! Discovery differs by assistant, so it is a trait ([`ToolDiscovery`]) with one
//! impl per assistant family (Claude Code, Zed, Cursor) — mirroring the assistant
//! adapters. Each impl knows where that assistant stores MCP config; the actual
//! config parsing is reused from [`crate::tasks::mcp_discovery`]. The capture
//! then probes each discovered server for its tool list and explodes it into
//! `assistant_tools`, plus a per-harness built-in catalog derived from observed
//! usage.
//!
//! The **bridge** ([`bridge_source_key`]) reconciles the forward registry (a
//! probed server's bare tool names) with the reverse usage naming: Claude Code
//! invokes MCP tools as `mcp__<harness_key>__<tool>`, and the harness key can
//! differ from the config key (`plugin_playwright_playwright` vs `playwright`).
//! We pick the usage prefix whose tool set best matches the probed set, so the
//! inventory's `source_key` + `invoked_name` line up with `tool_usage_stats`.

use crate::db::pg_store::PgStore;
use crate::tasks::mcp_discovery::{self, AcpFamily, DiscoveredMcp};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A per-assistant discovery adapter. New assistants add an impl; the capture
/// loops [`all_discoverers`].
pub trait ToolDiscovery: Send + Sync {
    /// The `assistant_family` string stored on rows this adapter produces.
    fn family(&self) -> &'static str;
    /// Discover the MCP servers configured for this assistant, user + project scope.
    fn discover_mcp(
        &self,
        home: &Path,
        project_roots: &[(uuid::Uuid, PathBuf)],
    ) -> Vec<DiscoveredMcp>;
}

/// One adapter per known assistant. Config parsing is reused from
/// `mcp_discovery`; each adapter just filters to its own family.
pub struct ClaudeCodeDiscovery;
pub struct ZedDiscovery;
pub struct CursorDiscovery;

fn scan(home: &Path, roots: &[(uuid::Uuid, PathBuf)], fam: AcpFamily) -> Vec<DiscoveredMcp> {
    let mut out: Vec<DiscoveredMcp> = mcp_discovery::discover_user_scope(home)
        .into_iter()
        .filter(|d| d.acp_family == fam)
        .collect();
    for (pid, root) in roots {
        out.extend(
            mcp_discovery::discover_project_scope(root, *pid)
                .into_iter()
                .filter(|d| d.acp_family == fam),
        );
    }
    out
}

impl ToolDiscovery for ClaudeCodeDiscovery {
    fn family(&self) -> &'static str {
        "claude"
    }
    fn discover_mcp(&self, home: &Path, roots: &[(uuid::Uuid, PathBuf)]) -> Vec<DiscoveredMcp> {
        let mut out = scan(home, roots, AcpFamily::Claude);
        out.extend(discover_claude_plugin_mcps(home));
        out
    }
}
impl ToolDiscovery for ZedDiscovery {
    fn family(&self) -> &'static str {
        "zed"
    }
    fn discover_mcp(&self, home: &Path, roots: &[(uuid::Uuid, PathBuf)]) -> Vec<DiscoveredMcp> {
        scan(home, roots, AcpFamily::Zed)
    }
}
impl ToolDiscovery for CursorDiscovery {
    fn family(&self) -> &'static str {
        "cursor"
    }
    fn discover_mcp(&self, home: &Path, roots: &[(uuid::Uuid, PathBuf)]) -> Vec<DiscoveredMcp> {
        scan(home, roots, AcpFamily::Cursor)
    }
}

/// The registered discovery adapters, one per assistant family.
pub fn all_discoverers() -> Vec<Box<dyn ToolDiscovery>> {
    vec![Box::new(ClaudeCodeDiscovery), Box::new(ZedDiscovery), Box::new(CursorDiscovery)]
}

/// Bridge a probed server's bare tool names to the harness usage prefix.
/// Returns the usage prefix whose observed tool set overlaps the probed set the
/// most (must be > 0). `None` when no usage prefix matches — the caller then
/// falls back to the config key, and the source simply won't join usage.
pub fn bridge_source_key(
    probed: &HashSet<String>,
    usage_prefixes: &HashMap<String, HashSet<String>>,
) -> Option<String> {
    usage_prefixes
        .iter()
        .map(|(p, tools)| (p.clone(), probed.intersection(tools).count()))
        .filter(|(_, n)| *n > 0)
        .max_by_key(|(_, n)| *n)
        .map(|(p, _)| p)
}

/// Build the harness-qualified invoked name for an MCP tool: `mcp__<key>__<tool>`.
pub fn mcp_invoked_name(source_key: &str, tool: &str) -> String {
    format!("mcp__{source_key}__{tool}")
}

/// Friendly display name for a tool source. Strips a leading `plugin_` and
/// collapses a duplicated `<x>_<x>` tail to one, so `plugin_sensei_sensei` →
/// "sensei" and `plugin_playwright_playwright` → "playwright"; a bare `svelte`
/// stays "svelte"; the built-in bucket reads "built-ins". The raw `source_key`
/// is still what the grid groups on — this only shapes the label.
pub fn pretty_source_name(source_type: &str, source_key: &str) -> String {
    if source_type == "builtin" {
        return "built-ins".to_string();
    }
    let s = source_key.strip_prefix("plugin_").unwrap_or(source_key);
    if let Some((a, b)) = s.split_once('_')
        && a == b
    {
        return a.to_string();
    }
    s.to_string()
}

/// Parse a server map that may be wrapped in `mcpServers` (svelte/semgrep) OR a
/// direct top-level `{ <key>: {command,args} }` map (playwright). Reuses
/// `mcp_discovery::parse_mcp_section` under the hood.
pub fn parse_any_server_map(doc: &Value, fam: AcpFamily, source: &Path) -> Vec<DiscoveredMcp> {
    if doc.get("mcpServers").and_then(|v| v.as_object()).is_some() {
        mcp_discovery::parse_mcp_section(doc, "mcpServers", fam, None, source)
    } else if doc.as_object().is_some() {
        let wrapped = serde_json::json!({ "mcpServers": doc });
        mcp_discovery::parse_mcp_section(&wrapped, "mcpServers", fam, None, source)
    } else {
        Vec::new()
    }
}

/// `.../cache/<marketplace>/<plugin>/<ver>/.mcp.json` → `<plugin>`.
pub fn plugin_name_from_cache_path(path: &Path) -> Option<String> {
    let comps: Vec<String> =
        path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    let idx = comps.iter().position(|c| c == "cache")?;
    comps.get(idx + 2).cloned()
}

/// Claude Code's tool sources beyond `~/.claude/mcp.json`: the direct user MCPs
/// in `~/.claude.json`, every installed plugin's `.mcp.json` under
/// `~/.claude/plugins/cache`, and the sensei plugin's `config.json`. Plugin
/// servers get `mcp_key = plugin_<plugin>_<server>` to line up with Claude
/// Code's invoked `mcp__plugin_<plugin>_<server>__<tool>` prefix.
pub fn discover_claude_plugin_mcps(home: &Path) -> Vec<DiscoveredMcp> {
    let mut out = Vec::new();

    // 1. ~/.claude.json → top-level "mcpServers" (direct user MCPs, e.g. svelte).
    let claude_json = home.join(".claude.json");
    if let Some(doc) = mcp_discovery::read_jsonc(&claude_json) {
        out.extend(
            mcp_discovery::parse_mcp_section(
                &doc,
                "mcpServers",
                AcpFamily::Claude,
                None,
                &claude_json,
            )
            .into_iter()
            .filter(|d| !d.command.is_empty()),
        );
    }

    // 2. Installed plugins' .mcp.json under ~/.claude/plugins/cache.
    let cache = home.join(".claude").join("plugins").join("cache");
    for entry in walkdir::WalkDir::new(&cache).max_depth(6).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() != std::ffi::OsStr::new(".mcp.json") {
            continue;
        }
        let path = entry.path();
        let Some(doc) = mcp_discovery::read_jsonc(path) else { continue };
        let plugin = plugin_name_from_cache_path(path).unwrap_or_else(|| "plugin".into());
        for d in parse_any_server_map(&doc, AcpFamily::Claude, path) {
            if d.command.is_empty() {
                continue;
            }
            let server = d.mcp_key.clone();
            out.push(DiscoveredMcp { mcp_key: format!("plugin_{plugin}_{server}"), ..d });
        }
    }

    // 3. sensei plugin — declared in config.json's `mcp_server`, not a .mcp.json.
    let sensei_cfg =
        home.join(".claude/plugins/marketplaces/sensei-marketplace/plugins/sensei-mcp/config.json");
    if let Some(doc) = mcp_discovery::read_jsonc(&sensei_cfg)
        && let Some(srv) = doc.get("mcp_server")
    {
        let command = srv.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !command.is_empty() {
            out.push(DiscoveredMcp {
                acp_family: AcpFamily::Claude,
                mcp_key: "plugin_sensei_sensei".to_string(),
                project_id: None,
                config_source: sensei_cfg.clone(),
                command,
                args: srv.get("args").cloned().unwrap_or(Value::Array(vec![])),
                env: srv.get("env").cloned().unwrap_or(Value::Object(Default::default())),
            });
        }
    }

    out
}

/// Counts from one capture pass.
pub struct CaptureCounts {
    pub discovered: usize,
    pub builtins: usize,
    pub probed_ok: usize,
    pub probed_err: usize,
}

/// The full Instruments · Health capture: discover each assistant's MCP servers
/// (per-assistant adapters), probe them, and rebuild the `assistant_tools`
/// inventory (MCP tools + a built-in catalog from observed usage). Idempotent —
/// safe to run at daemon startup and from the refresh endpoint.
pub async fn run_capture(pg: &PgStore) -> Result<CaptureCounts, String> {
    let home = crate::paths::home();

    // Project roots (project-scope MCP config: .mcp.json, .cursor/…, .zed/…).
    let projects = pg.list_projects().await?;
    let mut roots: Vec<(uuid::Uuid, PathBuf)> = Vec::new();
    for p in projects {
        let Some(pid) = p["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) else {
            continue;
        };
        let repos = pg.get_project_repos(&pid).await.unwrap_or_default();
        if let Some(first) = repos.first()
            && let Some(path) = first["path"].as_str()
        {
            roots.push((pid, PathBuf::from(path)));
        }
    }

    // 1. Per-assistant discovery → upsert mcp_servers; prune stale.
    let scan_start = chrono::Utc::now() - chrono::Duration::seconds(1);
    let mut discovered = 0usize;
    for adapter in all_discoverers() {
        for e in adapter.discover_mcp(&home, &roots) {
            if let Err(err) = pg
                .upsert_mcp_server(
                    e.acp_family.as_str(),
                    &e.mcp_key,
                    e.project_id,
                    &e.config_source.to_string_lossy(),
                    &e.command,
                    &e.args,
                    &e.env,
                )
                .await
            {
                tracing::error!(error = %err, key = %e.mcp_key, "tool capture: upsert_mcp_server failed");
            } else {
                discovered += 1;
            }
        }
    }
    let _ = pg.prune_stale_mcp_servers(scan_start).await;

    // 2. Rebuild the inventory from scratch.
    pg.clear_assistant_tools().await?;

    // 3. Built-in catalog from observed usage (invoked_name == bare name).
    let builtins = pg.distinct_builtin_tool_names().await.unwrap_or_default();
    for name in &builtins {
        let _ =
            pg.upsert_assistant_tool("claude", "builtin", "builtin", name, name, None, None).await;
    }

    // 4. Probe each server; explode its tools into the inventory via the bridge.
    let usage_prefixes = pg.usage_mcp_prefix_tools().await.unwrap_or_default();
    let servers = pg.list_mcp_servers(None).await.unwrap_or_default();
    let (mut probed_ok, mut probed_err) = (0usize, 0usize);
    for s in &servers {
        let Some(id) = s["id"].as_str().and_then(|x| uuid::Uuid::parse_str(x).ok()) else {
            continue;
        };
        let family = s["acp_family"].as_str().unwrap_or("claude").to_string();
        let mcp_key = s["mcp_key"].as_str().unwrap_or("").to_string();
        let command = s["command"].as_str().unwrap_or("").to_string();
        let args: Vec<String> = s["args"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let env: HashMap<String, String> = s["env"]
            .as_object()
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| v.as_str().map(|x| (k.clone(), x.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        // Relative plugin commands resolve against the config file's directory.
        let cwd = s["config_source"]
            .as_str()
            .map(PathBuf::from)
            .and_then(|p| p.parent().map(Path::to_path_buf));

        match crate::tasks::mcp_probe::probe_tools(&command, &args, &env, cwd.as_deref()).await {
            crate::tasks::mcp_probe::ProbeOutcome::Ok(manifest) => {
                probed_ok += 1;
                let tools_val = Value::Array(manifest.tools.clone());
                let _ = pg
                    .upsert_mcp_tool_manifest(
                        &id,
                        &tools_val,
                        manifest.tools.len() as i32,
                        manifest.protocol_version.as_deref(),
                        manifest.server_name.as_deref(),
                        manifest.server_version.as_deref(),
                        None,
                    )
                    .await;
                let _ = pg.set_mcp_server_connection_state(&id, "connected").await;

                let bare: HashSet<String> = manifest
                    .tools
                    .iter()
                    .filter_map(|t| t["name"].as_str().map(String::from))
                    .collect();
                let source_key =
                    bridge_source_key(&bare, &usage_prefixes).unwrap_or_else(|| mcp_key.clone());
                for t in &manifest.tools {
                    if let Some(tn) = t["name"].as_str() {
                        let inv = mcp_invoked_name(&source_key, tn);
                        let _ = pg
                            .upsert_assistant_tool(
                                &family,
                                "mcp",
                                &source_key,
                                tn,
                                &inv,
                                t["description"].as_str(),
                                Some(id),
                            )
                            .await;
                    }
                }
            }
            crate::tasks::mcp_probe::ProbeOutcome::Error(msg) => {
                probed_err += 1;
                let empty = Value::Array(vec![]);
                let _ =
                    pg.upsert_mcp_tool_manifest(&id, &empty, 0, None, None, None, Some(&msg)).await;
                let _ = pg.set_mcp_server_connection_state(&id, "error").await;
                tracing::warn!(server = %mcp_key, error = %msg,
                    "tool capture: probe failed; source shows usage-only (registered unknown)");
            }
        }
    }

    Ok(CaptureCounts { discovered, builtins: builtins.len(), probed_ok, probed_err })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(xs: &[&str]) -> HashSet<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn bridge_picks_the_best_overlapping_prefix() {
        let probed = set(&["browser_click", "browser_navigate", "browser_evaluate"]);
        let mut usage = HashMap::new();
        usage.insert(
            "plugin_playwright_playwright".to_string(),
            set(&["browser_click", "browser_navigate"]),
        ); // overlap 2
        usage.insert("svelte".to_string(), set(&["svelte-autofixer"])); // overlap 0
        assert_eq!(
            bridge_source_key(&probed, &usage).as_deref(),
            Some("plugin_playwright_playwright")
        );
    }

    #[test]
    fn bridge_none_when_no_overlap() {
        let probed = set(&["a", "b"]);
        let mut usage = HashMap::new();
        usage.insert("x".to_string(), set(&["c", "d"]));
        assert!(bridge_source_key(&probed, &usage).is_none());
    }

    #[test]
    fn invoked_name_format() {
        assert_eq!(mcp_invoked_name("sensei", "search"), "mcp__sensei__search");
    }

    #[test]
    fn pretty_source_name_labels() {
        // plugin_<x>_<x> collapses to the bare name.
        assert_eq!(pretty_source_name("mcp", "plugin_sensei_sensei"), "sensei");
        assert_eq!(pretty_source_name("mcp", "plugin_playwright_playwright"), "playwright");
        // A plain single-segment key is left as-is.
        assert_eq!(pretty_source_name("mcp", "svelte"), "svelte");
        assert_eq!(pretty_source_name("mcp", "playwright"), "playwright");
        // A leading plugin_ with a distinct tail keeps the remainder.
        assert_eq!(pretty_source_name("mcp", "plugin_foo_bar"), "foo_bar");
        // The built-in bucket gets a friendly label regardless of key.
        assert_eq!(pretty_source_name("builtin", "builtin"), "built-ins");
    }

    #[test]
    fn discoverers_cover_the_three_families() {
        let fams: Vec<_> = all_discoverers().iter().map(|d| d.family()).collect();
        assert!(fams.contains(&"claude") && fams.contains(&"zed") && fams.contains(&"cursor"));
    }

    #[test]
    fn parse_any_server_map_handles_wrapper_and_direct() {
        use serde_json::json;
        // Wrapper form (svelte / semgrep).
        let wrapped =
            json!({ "mcpServers": { "semgrep": { "command": "semgrep", "args": ["mcp"] } } });
        let a = parse_any_server_map(&wrapped, AcpFamily::Claude, Path::new("/x/.mcp.json"));
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].mcp_key, "semgrep");
        assert_eq!(a[0].command, "semgrep");
        // Direct form (playwright): no `mcpServers` wrapper.
        let direct =
            json!({ "playwright": { "command": "npx", "args": ["@playwright/mcp@latest"] } });
        let b = parse_any_server_map(&direct, AcpFamily::Claude, Path::new("/x/.mcp.json"));
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].mcp_key, "playwright");
        assert_eq!(b[0].command, "npx");
    }

    #[test]
    fn plugin_name_parsed_from_cache_path() {
        let p = Path::new(
            "/Users/j/.claude/plugins/cache/claude-plugins-official/semgrep/0.5.3/.mcp.json",
        );
        assert_eq!(plugin_name_from_cache_path(p).as_deref(), Some("semgrep"));
    }
}
