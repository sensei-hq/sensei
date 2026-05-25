mod helpers;
mod trait_def;
mod claude_code;
mod mcp_file;

use serde::{Deserialize, Serialize};
use trait_def::Assistant;
use claude_code::ClaudeCodeAssistant;
use mcp_file::{McpFileAssistant, McpEntryFormat};
use helpers::find_mcp_binary;

// ── Registry ───────────────────────────────────────────────────────────────

fn all_assistants() -> Vec<Box<dyn Assistant>> {
    vec![
        Box::new(McpFileAssistant {
            id: "claude-desktop", name: "Claude Desktop",
            family_id: Some("claude"), family_label: Some("Claude"),
            mcp_key: "mcpServers",
            config_rel: "Library/Application Support/Claude/claude_desktop_config.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Claude.app"],
            bin_names: &[],
            home_paths: &[],
        }),
        Box::new(ClaudeCodeAssistant),
        Box::new(McpFileAssistant {
            id: "cursor", name: "Cursor",
            family_id: None, family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".cursor/mcp.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Cursor.app"],
            bin_names: &["cursor"],
            home_paths: &[],
        }),
        Box::new(McpFileAssistant {
            id: "windsurf", name: "Windsurf",
            family_id: None, family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".codeium/windsurf/mcp_config.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Windsurf.app"],
            bin_names: &["windsurf"],
            home_paths: &[],
        }),
        Box::new(McpFileAssistant {
            id: "zed", name: "Zed",
            family_id: None, family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".config/zed/settings.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Zed.app"],
            bin_names: &["zed"],
            home_paths: &[".config/zed/settings.json"],
        }),
        Box::new(McpFileAssistant {
            id: "kiro", name: "Kiro",
            family_id: None, family_label: None,
            mcp_key: "mcpServers",
            config_rel: ".kiro/settings/mcp.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Kiro.app"],
            bin_names: &["kiro"],
            home_paths: &[],
        }),
        Box::new(McpFileAssistant {
            id: "opencode", name: "OpenCode",
            family_id: None, family_label: None,
            mcp_key: "mcp",
            config_rel: ".config/opencode/opencode.json",
            entry_format: McpEntryFormat::OpenCode,
            app_names: &[],
            bin_names: &["opencode"],
            home_paths: &[".config/opencode/opencode.json", ".local/bin/opencode"],
        }),
        Box::new(McpFileAssistant {
            id: "vscode", name: "VS Code",
            family_id: None, family_label: None,
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

/// Grouped view for the UI — one entry per family.
#[derive(Debug, Serialize, Clone)]
pub struct AssistantFamily {
    pub family: String,
    pub name: String,
    pub members: Vec<AssistantStatus>,
    pub installed: bool,
    pub config_path: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigureResult {
    pub configured: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
    /// True if `claude plugin install` succeeded (commands are namespaced under the plugin).
    pub plugin_installed: bool,
}

// ── Public API ─────────────────────────────────────────────────────────────

pub fn detect() -> Vec<AssistantStatus> {
    all_assistants().iter().map(|a| a.status()).collect()
}

/// Grouped view — one entry per family for the UI.
/// Claude Desktop + Claude Code become a single "Claude" family card.
pub fn detect_families() -> Vec<AssistantFamily> {
    let statuses = detect();
    let assistants = all_assistants();
    let mut families: Vec<AssistantFamily> = vec![];

    for asst in &assistants {
        let Some(status) = statuses.iter().find(|s| s.id == asst.id()).cloned() else { continue };
        let fam_id = asst.family();

        if let Some(existing) = families.iter_mut().find(|f| f.family == fam_id) {
            if status.installed { existing.installed = true; }
            existing.members.push(status);
        } else {
            families.push(AssistantFamily {
                family: fam_id.to_string(),
                name: asst.family_name().to_string(),
                installed: status.installed,
                config_path: status.config_path.clone(),
                members: vec![status],
            });
        }
    }
    families
}

pub fn configure(assistant_ids: &[String]) -> ConfigureResult {
    let assistants = all_assistants();
    let mut result = ConfigureResult {
        configured: vec![],
        skipped: vec![],
        errors: vec![],
        plugin_installed: false,
    };

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
        match asst.configure(&mcp_cmd) {
            Ok(ok) => {
                result.configured.push(asst.id().to_string());
                if ok.plugin { result.plugin_installed = true; }
                result.errors.extend(ok.warnings);
            }
            Err(e) => result.errors.push(format!("{}: {}", asst.id(), e)),
        }
    }

    // Persist the set of successfully configured assistants.
    let sensei_dir = crate::paths::sensei_dir();
    let mut local_cfg = sensei_bootstrap::SenseiLocalConfig::load(&sensei_dir);
    local_cfg.configured_assistants = targets.iter()
        .filter(|a| result.configured.contains(&a.id().to_string()))
        .map(|a| a.id().to_string())
        .collect();
    local_cfg.save(&sensei_dir).ok();

    result
}

/// Remove specific Assistant configs by ID. Empty slice = remove all.
pub fn remove_selected(ids: &[String]) -> Vec<String> {
    let assistants = all_assistants();
    let targets: Vec<&Box<dyn Assistant>> = if ids.is_empty() {
        assistants.iter().collect()
    } else {
        assistants.iter().filter(|a| ids.contains(&a.id().to_string())).collect()
    };
    targets
        .iter()
        .filter_map(|asst| {
            if asst.remove() { Some(asst.id().to_string()) } else { None }
        })
        .collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use helpers::{upsert_sensei_in_json, remove_sensei_from_json};
    use sensei_bootstrap::MCP_REGISTRY_KEY;

    // ── upsert_sensei_in_json ──────────────────────────────────────────

    #[test]
    fn upsert_creates_file_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});

        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
    }

    // ── JSONC (JSON with comments) ─────────────────────────────────────────

    #[test]
    fn upsert_preserves_all_keys_in_jsonc_file() {
        // Mirrors a real Zed settings.json: JSONC with comments, trailing commas,
        // and unrelated top-level keys that must not be touched.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"// Zed settings
{
  "terminal": { "dock": "right" },
  "mcpServers": {
    "other-tool": { "command": "other-mcp" }, // keep this
  },
  "file_types": { "SQL": ["ddl", "sql"] },
}"#).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
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
        std::fs::write(&path, format!(r#"// existing config
{{
  "vim_mode": false,
  "mcpServers": {{
    // sensei was here before
    "{MCP_REGISTRY_KEY}": {{ "command": "old-binary" }},
    "postgres": {{ "command": "pg-mcp" }},
  }},
}}"#)).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
        assert_eq!(content["mcpServers"]["postgres"]["command"], "pg-mcp");
        assert_eq!(content["vim_mode"], false);
    }

    #[test]
    fn remove_sensei_from_jsonc_preserves_other_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, format!(r#"// Zed settings
{{
  "terminal": {{ "dock": "right" }}, // trailing comma
  "mcpServers": {{
    "{MCP_REGISTRY_KEY}": {{ "command": "sensei-mcp" }},
    "svelte": {{ "command": "svelte-mcp" }},
  }},
}}"#)).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "svelte-mcp");
        assert_eq!(content["terminal"]["dock"], "right");
    }

    #[test]
    fn upsert_preserves_existing_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"svelte":{"command":"npx","args":["-y","svelte-mcp"]}}}"#).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn upsert_overwrites_existing_sensei_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, format!(r#"{{"mcpServers":{{"{MCP_REGISTRY_KEY}":{{"command":"old-binary"}}}}}}"#)).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
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

        let entry = serde_json::json!({"type": "local", "command": ["sensei-mcp", ""], "enabled": true});
        upsert_sensei_in_json(&path, "mcp", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
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
        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["svelte"].is_object());
    }

    #[test]
    fn remove_sensei_preserves_other_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, format!(r#"{{"mcpServers":{{"{MCP_REGISTRY_KEY}":{{"command":"sensei-mcp"}},"svelte":{{"command":"npx"}}}}}}"#)).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn remove_sensei_only_server() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, format!(r#"{{"mcpServers":{{"{MCP_REGISTRY_KEY}":{{"command":"sensei-mcp"}}}}}}"#)).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
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

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_object());
        assert!(content["mcpServers"]["svelte"].is_object());

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn roundtrip_fresh_file_upsert_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");

        upsert_sensei_in_json(&path, "mcpServers", serde_json::json!({"command": "sensei-mcp"})).unwrap();
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

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"][MCP_REGISTRY_KEY]["command"], "sensei-mcp");

        assert!(remove_sensei_from_json(&config_path, "mcpServers"));
        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(content["mcpServers"][MCP_REGISTRY_KEY].is_null());
    }

    #[test]
    fn opencode_assistant_uses_different_entry_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        let entry = serde_json::json!({"type": "local", "command": ["sensei-mcp", ""], "enabled": true});
        upsert_sensei_in_json(&path, "mcp", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
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
            assert!(!asst.config_path().as_os_str().is_empty(), "empty config_path for {}", asst.id());
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

    #[test]
    fn expected_assistants_are_registered() {
        let assistants = all_assistants();
        let ids: Vec<&str> = assistants.iter().map(|a| a.id()).collect();
        for expected in ["claude-desktop", "claude-code", "cursor", "windsurf", "zed", "kiro", "opencode", "vscode"] {
            assert!(ids.contains(&expected), "missing assistant: {}", expected);
        }
        assert_eq!(ids.len(), 8, "unexpected number of assistants");
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn upsert_into_invalid_json_file_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, "not json at all").unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
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
