mod claude_code;
mod health;
mod helpers;
mod mcp_file;
mod trait_def;
mod watchdog;
pub use health::{
    AdapterCheck, AdapterHealth, AdapterResolveReport, CheckStatus, business_elapsed_hours,
    capture_freshness,
};
pub use watchdog::{BreakerMap, CaptureWindow, health_report, resolve_adapter, run_sweep};

use crate::api::events::{AssistantPartEvent, AssistantPartStatus, StateEvent};
use claude_code::ClaudeCodeAssistant;
use helpers::find_mcp_binary;
use mcp_file::{McpEntryFormat, McpFileAssistant};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::broadcast;
use trait_def::Assistant;

// ── Registry ───────────────────────────────────────────────────────────────

fn all_assistants() -> Vec<Box<dyn Assistant>> {
    // ClaudeCodeAssistant is registered before Claude Desktop so the
    // detect_families() aggregation seeds the Claude family from the more
    // capable variant — surface part order becomes
    // [plugins, skills, commands, agents, mcp], which matches the
    // user-visible install sequence (manifest → skills → commands →
    // agents → mcp wiring). Claude Desktop's lone `mcp` part dedups in.
    vec![
        Box::new(ClaudeCodeAssistant),
        Box::new(McpFileAssistant {
            id: "claude-desktop",
            name: "Claude Desktop",
            family_id: Some("claude"),
            family_label: Some("Claude"),
            mcp_key: "mcpServers",
            config_rel: "Library/Application Support/Claude/claude_desktop_config.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Claude.app"],
            bin_names: &[],
            home_paths: &[],
        }),
        Box::new(McpFileAssistant {
            id: "cursor",
            name: "Cursor",
            family_id: None,
            family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".cursor/mcp.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Cursor.app"],
            bin_names: &["cursor"],
            home_paths: &[],
        }),
        Box::new(McpFileAssistant {
            id: "windsurf",
            name: "Windsurf",
            family_id: None,
            family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".codeium/windsurf/mcp_config.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Windsurf.app"],
            bin_names: &["windsurf"],
            home_paths: &[],
        }),
        Box::new(McpFileAssistant {
            id: "zed",
            name: "Zed",
            family_id: None,
            family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".config/zed/settings.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Zed.app"],
            bin_names: &["zed"],
            home_paths: &[".config/zed/settings.json"],
        }),
        Box::new(McpFileAssistant {
            id: "kiro",
            name: "Kiro",
            family_id: None,
            family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".kiro/settings/mcp.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Kiro.app"],
            bin_names: &["kiro"],
            home_paths: &[],
        }),
        Box::new(McpFileAssistant {
            id: "opencode",
            name: "OpenCode",
            family_id: None,
            family_label: None,
            mcp_key: "mcp",
            config_rel: ".config/opencode/opencode.json",
            entry_format: McpEntryFormat::OpenCode,
            app_names: &[],
            bin_names: &["opencode"],
            home_paths: &[".config/opencode/opencode.json", ".local/bin/opencode"],
        }),
        Box::new(McpFileAssistant {
            id: "vscode",
            name: "VS Code",
            family_id: None,
            family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".vscode/mcp.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Visual Studio Code.app"],
            bin_names: &["code"],
            home_paths: &[],
        }),
    ]
}

// ── Public types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantStatus {
    pub id: String,
    pub name: String,
    pub family: String,
    pub installed: bool,
    /// True when sensei is integrated with this assistant. The integration
    /// mechanism varies by assistant — Claude Code installs a plugin
    /// (skills, commands, agents, hooks, MCP all bundled); Cursor/Zed/etc.
    /// just register sensei as an MCP server in their config file. This
    /// flag is the unified "sensei is wired into this tool" signal.
    pub configured: bool,
    pub config_path: String,
}

/// A capability area an Assistant configures (e.g. plugins, skills, mcp).
/// The wizard renders one chip per part on each AssistantCard, and the
/// daemon emits per-part status transitions over SSE during configure().
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AssistantPart {
    pub id: String,
    pub label: String,
}

/// Grouped view for the UI — one entry per family.
#[derive(Debug, Serialize, Clone)]
pub struct AssistantFamily {
    pub family: String,
    pub name: String,
    pub members: Vec<AssistantStatus>,
    pub installed: bool,
    pub config_path: String,
    /// Union of parts across every variant in the family, in declaration
    /// order with duplicates removed. Stable across detect() so the UI
    /// can render chips without flicker when variants come and go.
    pub parts: Vec<AssistantPart>,
}

#[derive(Debug, Serialize, Default)]
pub struct ConfigureResult {
    pub configured: Vec<String>,
    pub skipped: Vec<String>,
    /// Hard failures — configure() did not complete for this assistant.
    pub errors: Vec<String>,
    /// Soft failures — configure() finished but a secondary side-effect (e.g.
    /// writing dev hooks) failed. The assistant is still considered configured.
    /// Kept separate from `errors` so callers can decide whether to surface
    /// loudly or just log.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// True if `claude plugin install` succeeded (commands are namespaced under the plugin).
    pub plugin_installed: bool,
}

// ── Public API ─────────────────────────────────────────────────────────────

pub fn detect() -> Vec<AssistantStatus> {
    all_assistants().iter().map(|a| a.status()).collect()
}

/// config_health for a single adapter id, or None if not registered.
pub fn config_health_for_id(adapter_id: &str) -> Option<Vec<crate::assistants::AdapterCheck>> {
    all_assistants().iter().find(|a| a.id() == adapter_id).map(|a| a.config_health())
}

/// Resolve a single adapter by id via its `resolve()` (reinstall). None if the
/// id isn't registered or the mcp binary can't be found.
pub fn resolve_by_id(adapter_id: &str) -> Option<crate::assistants::AdapterResolveReport> {
    let mcp_cmd = find_mcp_binary()?.to_string_lossy().to_string();
    all_assistants().iter().find(|a| a.id() == adapter_id).map(|a| a.resolve(&mcp_cmd))
}

/// Refresh an adapter's keep-alive (e.g. Claude Code's plugin in-use marker) by
/// id. No-op for adapters that don't override `keep_alive` or aren't registered.
pub fn keep_alive_by_id(adapter_id: &str) {
    if let Some(a) = all_assistants().iter().find(|a| a.id() == adapter_id) {
        a.keep_alive();
    }
}

/// Grouped view — one entry per family for the UI.
/// Claude Desktop + Claude Code become a single "Claude" family card.
///
/// `parts` is the union of every member variant's parts(), preserving the
/// order in which each unique part id was first seen. The dedup pass is
/// O(n²) but n is bounded by the small fixed registry — readable code wins.
pub fn detect_families() -> Vec<AssistantFamily> {
    let statuses = detect();
    let assistants = all_assistants();
    let mut families: Vec<AssistantFamily> = vec![];

    for asst in &assistants {
        let Some(status) = statuses.iter().find(|s| s.id == asst.id()).cloned() else { continue };
        let fam_id = asst.family();
        let variant_parts = asst.parts();

        if let Some(existing) = families.iter_mut().find(|f| f.family == fam_id) {
            if status.installed {
                existing.installed = true;
            }
            existing.members.push(status);
            for part in variant_parts {
                if !existing.parts.iter().any(|p| p.id == part.id) {
                    existing.parts.push(part);
                }
            }
        } else {
            families.push(AssistantFamily {
                family: fam_id.to_string(),
                name: asst.family_name().to_string(),
                installed: status.installed,
                config_path: status.config_path.clone(),
                members: vec![status],
                parts: variant_parts,
            });
        }
    }
    families
}

/// Configure assistants identified by id. When `event_tx` is Some, emits
/// per-part SSE events as each variant transitions through
/// configuring → done (or error). Each variant's parts transition
/// together because `claude plugin install` and the MCP-file writes are
/// atomic at the integration level. The wizard cascades the chip
/// animations visually.
///
/// `event_tx = None` keeps existing CLI callers (installer/install.rs)
/// behaviour identical — no SSE consumer in that path.
pub fn configure(
    assistant_ids: &[String],
    event_tx: Option<&broadcast::Sender<StateEvent>>,
) -> ConfigureResult {
    let assistants = all_assistants();
    let mut result = ConfigureResult::default();

    let mcp_cmd = match find_mcp_binary() {
        Some(p) => p.to_string_lossy().to_string(),
        None => {
            result.errors.push("sensei-mcp not found on PATH".into());
            return result;
        }
    };

    let targets: Vec<&Box<dyn Assistant>> = if assistant_ids.is_empty() {
        assistants.iter().filter(|a| a.detect()).collect()
    } else {
        assistants.iter().filter(|a| assistant_ids.contains(&a.id().to_string())).collect()
    };

    for asst in &targets {
        let family = asst.family().to_string();
        let parts = asst.parts();

        emit_parts(event_tx, &family, &parts, AssistantPartStatus::Configuring, None);

        match asst.configure(&mcp_cmd) {
            Ok(ok) => {
                result.configured.push(asst.id().to_string());
                if ok.plugin {
                    result.plugin_installed = true;
                }
                // Soft failures stay in `warnings` — the assistant did
                // configure successfully, but a side-effect (e.g. dev-hook
                // entries) didn't. Merging them into `errors` was hiding the
                // distinction; callers were treating successful installs as
                // failed when only dev hooks had a problem.
                for w in ok.warnings {
                    result.warnings.push(format!("{}: {}", asst.id(), w));
                }
                emit_parts(event_tx, &family, &parts, AssistantPartStatus::Done, None);
            }
            Err(e) => {
                emit_parts(event_tx, &family, &parts, AssistantPartStatus::Error, Some(&e));
                result.errors.push(format!("{}: {}", asst.id(), e));
            }
        }
    }

    // Persist the set of successfully configured assistants. If the save fails
    // (disk full, permission denied), surface it as a warning — configure()
    // already completed, the local config just won't reflect the run.
    let sensei_dir = crate::paths::sensei_dir();
    let mut local_cfg = sensei_bootstrap::SenseiLocalConfig::load(&sensei_dir);
    local_cfg.configured_assistants = targets
        .iter()
        .filter(|a| result.configured.contains(&a.id().to_string()))
        .map(|a| a.id().to_string())
        .collect();
    if let Err(e) = local_cfg.save(&sensei_dir) {
        result.warnings.push(format!("save SenseiLocalConfig: {}", e));
    }

    result
}

/// Refresh assistant integrations after a sensei binary upgrade. Mirrors
/// `configure()`'s target selection: an empty `assistant_ids` slice targets
/// every *detected* assistant; a non-empty slice targets exactly those ids
/// (regardless of detection). Each target's [`Assistant::upgrade`] runs and
/// its outcome is mapped to an [`AdapterResolveReport`] — the same per-adapter
/// result shape `/api/assistants/resolve` returns — so callers get a
/// per-assistant array.
///
/// For Claude Code this runs `claude plugin update sensei` (fixing the class
/// of bug where a new sensei binary leaves a stale plugin/MCP behind); every
/// file-based MCP assistant reports a no-op action (its config re-reads on the
/// assistant's own restart).
pub fn upgrade(assistant_ids: &[String]) -> Vec<AdapterResolveReport> {
    let assistants = all_assistants();
    let targets: Vec<&Box<dyn Assistant>> = if assistant_ids.is_empty() {
        assistants.iter().filter(|a| a.detect()).collect()
    } else {
        assistants.iter().filter(|a| assistant_ids.contains(&a.id().to_string())).collect()
    };

    targets
        .iter()
        .map(|asst| match asst.upgrade() {
            Ok(ok) => {
                let mut actions = Vec::new();
                if ok.plugin {
                    actions.push(format!("upgraded {} plugin", asst.id()));
                }
                // A no-op assistant carries its explanatory note in warnings;
                // surface it as an action line (same convention `resolve()`
                // uses — informational lines land in `actions`).
                actions.extend(ok.warnings);
                AdapterResolveReport {
                    adapter_id: asst.id().to_string(),
                    ok: true,
                    actions,
                    errors: vec![],
                }
            }
            Err(e) => AdapterResolveReport {
                adapter_id: asst.id().to_string(),
                ok: false,
                actions: vec![],
                errors: vec![e],
            },
        })
        .collect()
}

/// Broadcast one StateEvent per part. No-op when `tx` is `None` (CLI path)
/// or when the broadcast channel currently has no subscribers — `send`
/// returns Err in that case and we ignore it; SSE events are best-effort.
fn emit_parts(
    tx: Option<&broadcast::Sender<StateEvent>>,
    family: &str,
    parts: &[AssistantPart],
    status: AssistantPartStatus,
    error: Option<&str>,
) {
    let Some(tx) = tx else { return };
    for part in parts {
        let evt = StateEvent::assistant_part(AssistantPartEvent {
            family: family.to_string(),
            part: part.id.clone(),
            status: status.clone(),
            error: error.map(|s| s.to_string()),
        });
        let _ = tx.send(evt);
    }
}

/// Remove specific Assistant configs by ID. Empty slice = remove all.
///
/// When `event_tx` is Some, emits per-part `configuring` then `idle`
/// transitions so the wizard can show the removal in progress and then
/// reset the card. Same `None` fast-path as `configure()` for CLI callers.
pub fn remove_selected(
    ids: &[String],
    event_tx: Option<&broadcast::Sender<StateEvent>>,
) -> Vec<String> {
    let assistants = all_assistants();
    let targets: Vec<&Box<dyn Assistant>> = if ids.is_empty() {
        assistants.iter().collect()
    } else {
        assistants.iter().filter(|a| ids.contains(&a.id().to_string())).collect()
    };
    targets
        .iter()
        .filter_map(|asst| {
            let family = asst.family().to_string();
            let parts = asst.parts();
            emit_parts(event_tx, &family, &parts, AssistantPartStatus::Configuring, None);
            if asst.remove() {
                // After removal, the parts return to the empty/idle ring.
                emit_parts(event_tx, &family, &parts, AssistantPartStatus::Idle, None);
                Some(asst.id().to_string())
            } else {
                emit_parts(
                    event_tx,
                    &family,
                    &parts,
                    AssistantPartStatus::Error,
                    Some("remove failed"),
                );
                None
            }
        })
        .collect()
}

/// Every MCP entry key present in any installed assistant's MCP config file,
/// lowercased for case-insensitive matching. Used by the instruments
/// registry to decide whether a known MCP is already wired in somewhere —
/// "installed" from the user's point of view means "some AI tool can call
/// it", not just "exists on disk".
///
/// Only inspects files that already exist (no creation, no parsing of stub
/// JSONC schemas); silently skips unreadable / malformed configs.
pub fn installed_mcp_keys() -> HashSet<String> {
    let mut keys = HashSet::new();
    for asst in all_assistants() {
        let path = asst.config_path();
        if !path.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "failed to read assistant MCP config");
                continue;
            }
        };
        // Configs are JSONC for some assistants — fall back to json5 if strict JSON fails.
        let value: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => match json5::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            },
        };
        if let Some(servers) = value.get(asst.mcp_key()).and_then(|v| v.as_object()) {
            for k in servers.keys() {
                keys.insert(k.to_lowercase());
            }
        }
    }
    keys
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use helpers::{remove_sensei_from_json, upsert_sensei_in_json};
    use sensei_bootstrap::MCP_REGISTRY_KEY;

    // ── ConfigureResult: warnings vs errors ───────────────────────────────────

    #[test]
    fn configure_result_default_has_empty_warnings_and_errors() {
        let r = ConfigureResult::default();
        assert!(
            r.warnings.is_empty(),
            "default ConfigureResult must not carry warnings (got: {:?})",
            r.warnings
        );
        assert!(
            r.errors.is_empty(),
            "default ConfigureResult must not carry errors (got: {:?})",
            r.errors
        );
        assert!(r.configured.is_empty());
        assert!(!r.plugin_installed);
    }

    #[test]
    fn configure_result_serialises_warnings_alongside_errors() {
        // Callers (CLI/UI) need both fields so they can decide whether a
        // run was a hard failure (errors) or a soft failure (warnings).
        // If serde drops `warnings`, the CLI silently treats every install
        // as success-with-no-caveats — the exact pattern we're fixing.
        let r = ConfigureResult {
            configured: vec!["claude-code".into()],
            errors: vec!["plugin install: boom".into()],
            warnings: vec!["dev hooks: settings.json not writable".into()],
            plugin_installed: true,
            ..Default::default()
        };
        let j = serde_json::to_value(&r).unwrap();
        assert_eq!(j["errors"][0], "plugin install: boom");
        assert_eq!(j["warnings"][0], "dev hooks: settings.json not writable");
        assert_eq!(j["plugin_installed"], true);
        assert_eq!(j["configured"][0], "claude-code");
    }

    // ── upsert_sensei_in_json ──────────────────────────────────────────

    #[test]
    fn upsert_creates_file_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});

        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
    }

    // ── JSONC (JSON with comments) ─────────────────────────────────────────

    #[test]
    fn upsert_preserves_all_keys_in_jsonc_file() {
        // Mirrors a real Zed settings.json: JSONC with comments, trailing commas,
        // and unrelated top-level keys that must not be touched.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"// Zed settings
{
  "terminal": { "dock": "right" },
  "mcpServers": {
    "other-tool": { "command": "other-mcp" }, // keep this
  },
  "file_types": { "SQL": ["ddl", "sql"] },
}"#,
        )
        .unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        // Written file preserves the leading `// Zed settings` header (#51),
        // so the assertion uses json5 (JSONC-tolerant) rather than strict
        // serde_json.
        let content: serde_json::Value =
            json5::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // Sensei entry added
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
        // Sibling mcpServer preserved
        assert_eq!(content["mcpServers"]["other-tool"]["command"], "other-mcp");
        // Unrelated top-level keys preserved
        assert_eq!(content["terminal"]["dock"], "right");
        assert_eq!(content["file_types"]["SQL"][0], "ddl");
    }

    #[test]
    fn upsert_jsonc_updates_existing_sensei_without_touching_siblings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            format!(
                r#"// existing config
{{
  "vim_mode": false,
  "mcpServers": {{
    // sensei was here before
    "{MCP_REGISTRY_KEY}": {{ "command": "old-binary" }},
    "postgres": {{ "command": "pg-mcp" }},
  }},
}}"#
            ),
        )
        .unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value =
            json5::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
        assert_eq!(content["mcpServers"]["postgres"]["command"], "pg-mcp");
        assert_eq!(content["vim_mode"], false);
    }

    #[test]
    fn remove_sensei_from_jsonc_preserves_other_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            format!(
                r#"// Zed settings
{{
  "terminal": {{ "dock": "right" }}, // trailing comma
  "mcpServers": {{
    "{MCP_REGISTRY_KEY}": {{ "command": "sensei-mcp" }},
    "svelte": {{ "command": "svelte-mcp" }},
  }},
}}"#
            ),
        )
        .unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value =
            json5::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "svelte-mcp");
        assert_eq!(content["terminal"]["dock"], "right");
    }

    #[test]
    fn upsert_preserves_existing_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(
            &path,
            r#"{"mcpServers":{"svelte":{"command":"npx","args":["-y","svelte-mcp"]}}}"#,
        )
        .unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn upsert_overwrites_existing_sensei_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(
            &path,
            format!(r#"{{"mcpServers":{{"{MCP_REGISTRY_KEY}":{{"command":"old-binary"}}}}}}"#),
        )
        .unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
    }

    #[test]
    fn upsert_creates_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep/nested/mcp.json");

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn upsert_with_different_mcp_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        let entry =
            serde_json::json!({"type": "local", "command": ["sensei-mcp", ""], "enabled": true});
        upsert_sensei_in_json(&path, "mcp", entry).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcp"][MCP_REGISTRY_KEY]["type"], "local");
        assert_eq!(content["mcp"][MCP_REGISTRY_KEY]["enabled"], true);
    }

    // ── remove_sensei_from_json ────────────────────────────────────────

    #[test]
    fn remove_from_nonexistent_file_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.json");
        assert!(!remove_sensei_from_json(&path, "mcpServers"));
    }

    #[test]
    fn remove_when_sensei_not_present_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"svelte":{"command":"npx"}}}"#).unwrap();

        assert!(!remove_sensei_from_json(&path, "mcpServers"));
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["svelte"].is_object());
    }

    #[test]
    fn remove_sensei_preserves_other_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, format!(r#"{{"mcpServers":{{"{MCP_REGISTRY_KEY}":{{"command":"sensei-mcp"}},"svelte":{{"command":"npx"}}}}}}"#)).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn remove_sensei_only_server() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(
            &path,
            format!(r#"{{"mcpServers":{{"{MCP_REGISTRY_KEY}":{{"command":"sensei-mcp"}}}}}}"#),
        )
        .unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
        assert!(content["mcpServers"].as_object().unwrap().is_empty());
    }

    #[test]
    fn remove_with_opencode_mcp_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        std::fs::write(&path, format!(r#"{{"mcp":{{"{MCP_REGISTRY_KEY}":{{"type":"local","command":["sensei-mcp",""]}}}}}}"#)).unwrap();

        assert!(remove_sensei_from_json(&path, "mcp"));
    }

    // ── roundtrip: upsert then remove ──────────────────────────────────

    #[test]
    fn roundtrip_upsert_then_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"svelte":{"command":"npx"}}}"#).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_object());
        assert!(content["mcpServers"]["svelte"].is_object());

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn roundtrip_fresh_file_upsert_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");

        upsert_sensei_in_json(&path, "mcpServers", serde_json::json!({"command": "sensei-mcp"}))
            .unwrap();
        assert!(path.exists());

        assert!(remove_sensei_from_json(&path, "mcpServers"));
    }

    // ── McpFileAssistant configure/unconfigure ──────────────────────────

    #[test]
    fn mcp_file_assistant_configure_creates_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("test-assistant/mcp.json");

        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});
        upsert_sensei_in_json(&config_path, "mcpServers", entry).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");

        assert!(remove_sensei_from_json(&config_path, "mcpServers"));
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
    }

    #[test]
    fn opencode_assistant_uses_different_entry_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        let entry =
            serde_json::json!({"type": "local", "command": ["sensei-mcp", ""], "enabled": true});
        upsert_sensei_in_json(&path, "mcp", entry).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcp"][MCP_REGISTRY_KEY]["type"], "local");
        assert_eq!(content["mcp"][MCP_REGISTRY_KEY]["enabled"], true);
        let cmd = content["mcp"][MCP_REGISTRY_KEY]["command"].as_array().unwrap();
        assert_eq!(cmd[0], "sensei-mcp");
    }

    // ── Registry consistency ───────────────────────────────────────────

    #[test]
    fn all_assistants_have_unique_ids() {
        let assistants = all_assistants();
        let mut ids: Vec<&str> = assistants.iter().map(|a| a.id()).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "duplicate assistant ids found");
    }

    #[test]
    fn all_assistants_have_nonempty_fields() {
        for asst in all_assistants() {
            assert!(!asst.id().is_empty(), "empty id");
            assert!(!asst.name().is_empty(), "empty name for {}", asst.id());
            assert!(!asst.mcp_key().is_empty(), "empty mcp_key for {}", asst.id());
            assert!(
                !asst.config_path().as_os_str().is_empty(),
                "empty config_path for {}",
                asst.id()
            );
        }
    }

    #[test]
    fn claude_code_is_registered() {
        let assistants = all_assistants();
        assert!(assistants.iter().any(|a| a.id() == "claude-code"));
    }

    #[test]
    fn opencode_uses_mcp_key() {
        let assistants = all_assistants();
        let oc = assistants.iter().find(|a| a.id() == "opencode").unwrap();
        assert_eq!(oc.mcp_key(), "mcp");
    }

    #[test]
    fn standard_assistants_use_mcp_servers_key() {
        let assistants = all_assistants();
        for id in ["cursor", "windsurf", "zed", "kiro", "vscode", "claude-desktop", "claude-code"] {
            let asst = assistants.iter().find(|a| a.id() == id).expect(id);
            assert_eq!(asst.mcp_key(), "mcpServers", "{} should use mcpServers key", id);
        }
    }

    // ── SSE event emission: emit_parts() helper ────────────────────────
    //
    // The wizard's AssistantCard relies on per-part SSE events to drive
    // the chip animations. These tests pin the broadcast contract without
    // needing a live HTTP server: a broadcast channel with a subscriber,
    // a manual call to `emit_parts()`, and a drain of the receiver.

    #[tokio::test]
    async fn emit_parts_broadcasts_one_event_per_part() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<StateEvent>(16);
        let parts = vec![
            AssistantPart { id: "plugins".into(), label: "plugins".into() },
            AssistantPart { id: "skills".into(), label: "skills".into() },
        ];

        emit_parts(Some(&tx), "claude", &parts, AssistantPartStatus::Configuring, None);

        let first = rx.recv().await.unwrap();
        let second = rx.recv().await.unwrap();
        assert_eq!(first.entity, "assistant");
        assert_eq!(first.data["family"], "claude");
        assert_eq!(first.data["part"], "plugins");
        assert_eq!(first.data["status"], "configuring");
        assert_eq!(second.data["part"], "skills");
        assert_eq!(second.data["status"], "configuring");
    }

    #[tokio::test]
    async fn emit_parts_carries_error_message_when_status_is_error() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<StateEvent>(8);
        let parts = vec![AssistantPart { id: "mcp".into(), label: "mcp server".into() }];

        emit_parts(
            Some(&tx),
            "cursor",
            &parts,
            AssistantPartStatus::Error,
            Some("permission denied"),
        );

        let evt = rx.recv().await.unwrap();
        assert_eq!(evt.data["status"], "error");
        assert_eq!(evt.data["error"], "permission denied");
    }

    #[tokio::test]
    async fn emit_parts_omits_error_field_when_none() {
        // serde skips `error: None` so the JSON shape stays tight on the
        // happy path. The AssistantCard treats missing error as "no message
        // to consolidate" — leaving the field in as `null` would force the
        // app to treat null as a distinct case from absent.
        let (tx, mut rx) = tokio::sync::broadcast::channel::<StateEvent>(8);
        let parts = vec![AssistantPart { id: "mcp".into(), label: "mcp server".into() }];

        emit_parts(Some(&tx), "zed", &parts, AssistantPartStatus::Done, None);

        let evt = rx.recv().await.unwrap();
        assert!(
            evt.data.get("error").is_none(),
            "error key should be absent on success, got {:?}",
            evt.data
        );
    }

    #[test]
    fn emit_parts_no_op_when_tx_is_none() {
        // CLI callers pass None — must not panic, must not allocate a
        // channel, must not require any subscriber.
        let parts = vec![AssistantPart { id: "mcp".into(), label: "mcp server".into() }];
        emit_parts(None, "cursor", &parts, AssistantPartStatus::Configuring, None);
    }

    #[tokio::test]
    async fn emit_parts_swallows_send_error_when_no_subscribers() {
        // SSE events are best-effort. A broadcast with no live subscriber
        // returns Err(SendError) from send(); emit_parts must drop it
        // silently so configure() isn't aborted by transient UI churn.
        let (tx, _rx) = tokio::sync::broadcast::channel::<StateEvent>(4);
        // Drop the only receiver, then send.
        drop(_rx);
        let parts = vec![AssistantPart { id: "mcp".into(), label: "mcp server".into() }];
        emit_parts(Some(&tx), "cursor", &parts, AssistantPartStatus::Configuring, None);
    }

    // ── Parts: per-Assistant capability declaration ────────────────────

    #[test]
    fn claude_code_declares_five_parts_in_canonical_order() {
        // The wizard renders chips in declaration order; lock the order so
        // a future refactor doesn't silently reshuffle the user-visible
        // strip of capabilities on every Claude install. MCP is listed
        // explicitly — skills/commands/agents all invoke sensei through
        // it, so it belongs in the chip list alongside the others rather
        // than being hidden as an implementation detail of the plugin.
        let claude = all_assistants()
            .into_iter()
            .find(|a| a.id() == "claude-code")
            .expect("claude-code registered");
        let ids: Vec<String> = claude.parts().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, ["plugins", "skills", "commands", "agents", "mcp"]);
    }

    #[test]
    fn mcp_file_assistants_declare_single_mcp_part() {
        // Cursor/Zed/Windsurf/etc. all wire sensei in via the same MCP-config
        // mechanism. They each get one chip labelled "mcp server" — anything
        // else would imply per-feature granularity the integration doesn't
        // actually have.
        for id in ["cursor", "zed", "windsurf", "kiro", "opencode", "vscode", "claude-desktop"] {
            let asst = all_assistants().into_iter().find(|a| a.id() == id).expect(id);
            let parts = asst.parts();
            assert_eq!(parts.len(), 1, "{id} should declare exactly one part");
            assert_eq!(parts[0].id, "mcp", "{id} part id should be 'mcp'");
            assert_eq!(parts[0].label, "mcp server", "{id} part label should be 'mcp server'");
        }
    }

    #[test]
    fn family_parts_union_dedups_in_declaration_order() {
        // Claude family contains claude-code (4 parts) + claude-desktop (mcp).
        // The aggregate should preserve claude-code's declaration order at
        // the front, then append claude-desktop's mcp once. No duplicate
        // ids in the union — the UI renders one chip per id and a dup
        // would break the React-style key invariant on the chip list.
        let families = detect_families();
        let claude = families
            .iter()
            .find(|f| f.family == "claude")
            .expect("claude family should be present");
        let ids: Vec<&str> = claude.parts.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["plugins", "skills", "commands", "agents", "mcp"]);
    }

    #[test]
    fn family_parts_serialise_into_assistant_family() {
        // The API hands AssistantFamily to the app over JSON. Pin the shape
        // here so a future serde change (renaming `parts`, dropping fields)
        // would fail loudly instead of silently breaking the wizard card.
        let families = detect_families();
        let cursor = families
            .iter()
            .find(|f| f.family == "cursor")
            .expect("cursor family should be present");
        let json = serde_json::to_value(cursor).unwrap();
        let parts = json
            .get("parts")
            .and_then(|p| p.as_array())
            .expect("parts array on AssistantFamily JSON");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["id"], "mcp");
        assert_eq!(parts[0]["label"], "mcp server");
    }

    #[test]
    fn expected_assistants_are_registered() {
        let assistants = all_assistants();
        let ids: Vec<&str> = assistants.iter().map(|a| a.id()).collect();
        for expected in [
            "claude-desktop",
            "claude-code",
            "cursor",
            "windsurf",
            "zed",
            "kiro",
            "opencode",
            "vscode",
        ] {
            assert!(ids.contains(&expected), "missing assistant: {}", expected);
        }
        assert_eq!(ids.len(), 8, "unexpected number of assistants");
    }

    // ── upgrade() fan-out ──────────────────────────────────────────────
    //
    // The fan-out is exercised against a *file-based* assistant id (cursor),
    // whose upgrade() is the trait default no-op — so no `claude plugin update`
    // side effect fires. Targeting claude-code here would run the real update
    // and mutate the installed plugin, so these tests never do that (same
    // constraint noted for configure/remove at the bottom of this module).

    #[test]
    fn upgrade_noop_assistant_returns_ok_report_with_note() {
        let reports = upgrade(&["cursor".to_string()]);
        assert_eq!(reports.len(), 1, "one target selected → one report");
        let r = &reports[0];
        assert_eq!(r.adapter_id, "cursor");
        assert!(r.ok, "a file-based assistant upgrade is a successful no-op");
        assert!(r.errors.is_empty());
        assert!(
            r.actions.iter().any(|a| a.contains("no upgrade action")),
            "no-op note should surface as an action; got {:?}",
            r.actions
        );
    }

    #[test]
    fn upgrade_unknown_id_yields_empty_report_array() {
        // A non-empty id list that matches nothing selects nothing — no
        // accidental fallback to "all detected" (which would run the real
        // claude update).
        let reports = upgrade(&["does-not-exist".to_string()]);
        assert!(reports.is_empty());
    }

    #[test]
    fn upgrade_report_serialises_as_per_assistant_array() {
        // Pin the endpoint's wire shape: an array of {adapter_id, ok, actions,
        // errors}. This is what POST /api/assistants/upgrade hands the CLI/app.
        let reports = upgrade(&["cursor".to_string()]);
        let json = serde_json::to_value(&reports).unwrap();
        let arr = json.as_array().expect("upgrade reports serialise to an array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["adapter_id"], "cursor");
        assert_eq!(arr[0]["ok"], true);
        assert!(arr[0]["actions"].is_array());
        assert!(arr[0]["errors"].is_array());
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn upsert_into_invalid_json_file_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, "not json at all").unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
    }

    #[test]
    fn remove_from_invalid_json_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, "not json").unwrap();

        assert!(!remove_sensei_from_json(&path, "mcpServers"));
    }

    #[test]
    fn remove_when_mcp_key_missing_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"otherKey": {}}"#).unwrap();

        assert!(!remove_sensei_from_json(&path, "mcpServers"));
    }

    // Note: ClaudeCodeAssistant::configure/remove call `claude` CLI which
    // can't be safely mocked in parallel unit tests (requires PATH mutation).
    // The CLI invocation behavior is tested via the integration/e2e path.
}
