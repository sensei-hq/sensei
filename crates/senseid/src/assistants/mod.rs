mod helpers;
mod trait_def;
mod claude_code;
mod mcp_file;

use serde::{Deserialize, Serialize};
use trait_def::Acp;
use claude_code::ClaudeCodeAcp;
use mcp_file::{McpFileAcp, McpEntryFormat};
use helpers::{home, find_mcp_binary};

// ── Registry ───────────────────────────────────────────────────────────────

fn all_acps() -> Vec<Box<dyn Acp>> {
    vec![
        Box::new(McpFileAcp {
            id: "claude-desktop", name: "Claude Desktop",
            mcp_key: "mcpServers",
            config_rel: "Library/Application Support/Claude/claude_desktop_config.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Claude.app"],
            bin_names: &[],
            home_paths: &[],
        }),
        Box::new(ClaudeCodeAcp),
        Box::new(McpFileAcp {
            id: "cursor", name: "Cursor",
            mcp_key: "mcpServers",
            config_rel: ".cursor/mcp.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Cursor.app"],
            bin_names: &["cursor"],
            home_paths: &[],
        }),
        Box::new(McpFileAcp {
            id: "windsurf", name: "Windsurf",
            mcp_key: "mcpServers",
            config_rel: ".codeium/windsurf/mcp_config.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Windsurf.app"],
            bin_names: &["windsurf"],
            home_paths: &[],
        }),
        Box::new(McpFileAcp {
            id: "zed", name: "Zed",
            mcp_key: "mcpServers",
            config_rel: ".config/zed/settings.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Zed.app"],
            bin_names: &["zed"],
            home_paths: &[".config/zed/settings.json"],
        }),
        Box::new(McpFileAcp {
            id: "kiro", name: "Kiro",
            mcp_key: "mcpServers",
            config_rel: ".kiro/settings/mcp.json",
            entry_format: McpEntryFormat::Standard,
            app_names: &["Kiro.app"],
            bin_names: &["kiro"],
            home_paths: &[],
        }),
        Box::new(McpFileAcp {
            id: "opencode", name: "OpenCode",
            mcp_key: "mcp",
            config_rel: ".config/opencode/opencode.json",
            entry_format: McpEntryFormat::OpenCode,
            app_names: &[],
            bin_names: &["opencode"],
            home_paths: &[".config/opencode/opencode.json", ".local/bin/opencode"],
        }),
        Box::new(McpFileAcp {
            id: "vscode", name: "VS Code",
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
pub struct AcpStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub mcp_configured: bool,
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

pub fn detect() -> Vec<AcpStatus> {
    all_acps().iter().map(|acp| acp.status()).collect()
}

pub fn configure(acp_ids: &[String]) -> ConfigureResult {
    let acps = all_acps();
    let h = home();
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

    let targets: Vec<&Box<dyn Acp>> = if acp_ids.is_empty() {
        acps.iter().filter(|a| a.detect()).collect()
    } else {
        acps.iter().filter(|a| acp_ids.contains(&a.id().to_string())).collect()
    };

    for acp in &targets {
        match acp.configure(&mcp_cmd) {
            Ok(ok) => {
                result.configured.push(acp.id().to_string());
                if ok.plugin { result.plugin_installed = true; }
                result.errors.extend(ok.warnings);
            }
            Err(e) => result.errors.push(format!("{}: {}", acp.id(), e)),
        }
    }

    // Save configured ACPs
    let sensei_dir = h.join(".sensei");
    std::fs::create_dir_all(&sensei_dir).ok();
    let config_file = sensei_dir.join("config.json");
    let mut config: serde_json::Value = config_file
        .exists()
        .then(|| std::fs::read_to_string(&config_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    config["configured_acps"] = serde_json::json!(
        targets.iter()
            .filter(|a| result.configured.contains(&a.id().to_string()))
            .map(|a| a.id().to_string())
            .collect::<Vec<_>>()
    );
    std::fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap()).ok();

    result
}

/// Remove specific ACP configs by ID. Empty slice = remove all.
pub fn remove_selected(ids: &[String]) -> Vec<String> {
    let acps = all_acps();
    let targets: Vec<&Box<dyn Acp>> = if ids.is_empty() {
        acps.iter().collect()
    } else {
        acps.iter().filter(|a| ids.contains(&a.id().to_string())).collect()
    };
    targets
        .iter()
        .filter_map(|acp| {
            if acp.remove() { Some(acp.id().to_string()) } else { None }
        })
        .collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use helpers::{upsert_sensei_in_json, remove_sensei_from_json};

    // ── upsert_sensei_in_json ──────────────────────────────────────────

    #[test]
    fn upsert_creates_file_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});

        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
    }

    #[test]
    fn upsert_preserves_existing_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"svelte":{"command":"npx","args":["-y","svelte-mcp"]}}}"#).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn upsert_overwrites_existing_sensei_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"sensei":{"command":"old-binary"}}}"#).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
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
        assert_eq!(content["mcp"]["sensei"]["type"], "local");
        assert_eq!(content["mcp"]["sensei"]["enabled"], true);
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
        std::fs::write(&path, r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"},"svelte":{"command":"npx"}}}"#).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn remove_sensei_only_server() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"}}}"#).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert!(content["mcpServers"].as_object().unwrap().is_empty());
    }

    #[test]
    fn remove_with_opencode_mcp_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        std::fs::write(&path, r#"{"mcp":{"sensei":{"type":"local","command":["sensei-mcp",""]}}}"#).unwrap();

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
        assert!(content["mcpServers"]["sensei"].is_object());
        assert!(content["mcpServers"]["svelte"].is_object());

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
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

    // ── McpFileAcp configure/unconfigure ───────────────────────────────

    #[test]
    fn mcp_file_acp_configure_creates_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("test-acp/mcp.json");

        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});
        upsert_sensei_in_json(&config_path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");

        assert!(remove_sensei_from_json(&config_path, "mcpServers"));
        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
    }

    #[test]
    fn opencode_acp_uses_different_entry_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        let entry = serde_json::json!({"type": "local", "command": ["sensei-mcp", ""], "enabled": true});
        upsert_sensei_in_json(&path, "mcp", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcp"]["sensei"]["type"], "local");
        assert_eq!(content["mcp"]["sensei"]["enabled"], true);
        let cmd = content["mcp"]["sensei"]["command"].as_array().unwrap();
        assert_eq!(cmd[0], "sensei-mcp");
    }

    // ── Registry consistency ───────────────────────────────────────────

    #[test]
    fn all_acps_have_unique_ids() {
        let acps = all_acps();
        let mut ids: Vec<&str> = acps.iter().map(|a| a.id()).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "duplicate ACP ids found");
    }

    #[test]
    fn all_acps_have_nonempty_fields() {
        for acp in all_acps() {
            assert!(!acp.id().is_empty(), "empty id");
            assert!(!acp.name().is_empty(), "empty name for {}", acp.id());
            assert!(!acp.mcp_key().is_empty(), "empty mcp_key for {}", acp.id());
            assert!(!acp.config_path().as_os_str().is_empty(), "empty config_path for {}", acp.id());
        }
    }

    #[test]
    fn claude_code_is_registered() {
        let acps = all_acps();
        assert!(acps.iter().any(|a| a.id() == "claude-code"));
    }

    #[test]
    fn opencode_uses_mcp_key() {
        let acps = all_acps();
        let oc = acps.iter().find(|a| a.id() == "opencode").unwrap();
        assert_eq!(oc.mcp_key(), "mcp");
    }

    #[test]
    fn standard_acps_use_mcp_servers_key() {
        let acps = all_acps();
        for id in ["cursor", "windsurf", "zed", "kiro", "vscode", "claude-desktop", "claude-code"] {
            let acp = acps.iter().find(|a| a.id() == id).expect(id);
            assert_eq!(acp.mcp_key(), "mcpServers", "{} should use mcpServers key", id);
        }
    }

    #[test]
    fn expected_acps_are_registered() {
        let acps = all_acps();
        let ids: Vec<&str> = acps.iter().map(|a| a.id()).collect();
        for expected in ["claude-desktop", "claude-code", "cursor", "windsurf", "zed", "kiro", "opencode", "vscode"] {
            assert!(ids.contains(&expected), "missing ACP: {}", expected);
        }
        assert_eq!(ids.len(), 8, "unexpected number of ACPs");
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
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
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

    // Note: ClaudeCodeAcp::configure/remove call `claude` CLI which
    // can't be safely mocked in parallel unit tests (requires PATH mutation).
    // The CLI invocation behavior is tested via the integration/e2e path.
}
