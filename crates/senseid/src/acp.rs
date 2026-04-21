use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SENSEI_MARKETPLACE_REPO: &str = "mizukisu/sensei-marketplace";

// ── Trait ───────────────────────────────────────────────────────────────────

/// Result of configuring an ACP. `plugin` is true when `claude plugin install` succeeded.
struct AcpConfigureOk {
    plugin: bool,
    warnings: Vec<String>,
}

/// Each AI Coding Platform implements detect, configure, and remove.
trait Acp {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn mcp_key(&self) -> &str;
    fn config_path(&self) -> PathBuf;
    fn detect(&self) -> bool;
    fn configure(&self, mcp_cmd: &str, marketplace_path: Option<&str>) -> Result<AcpConfigureOk, String>;
    fn remove(&self) -> bool;

    fn is_configured(&self) -> bool {
        check_mcp_configured(&self.config_path(), self.mcp_key())
    }

    fn status(&self) -> AcpStatus {
        AcpStatus {
            id: self.id().to_string(),
            name: self.name().to_string(),
            installed: self.detect(),
            mcp_configured: self.is_configured(),
            config_path: self.config_path().to_string_lossy().into_owned(),
        }
    }
}

// ── Claude Code ACP ────────────────────────────────────────────────────────

struct ClaudeCodeAcp;

impl Acp for ClaudeCodeAcp {
    fn id(&self) -> &str { "claude-code" }
    fn name(&self) -> &str { "Claude Code" }
    fn mcp_key(&self) -> &str { "mcpServers" }
    fn config_path(&self) -> PathBuf { home().join(".claude/settings.json") }

    fn detect(&self) -> bool {
        let h = home();
        h.join(".claude/settings.json").exists()
            || h.join(".claude/CLAUDE.md").exists()
            || h.join(".claude").exists()
    }

    fn configure(&self, mcp_cmd: &str, marketplace_path: Option<&str>) -> Result<AcpConfigureOk, String> {
        let claude_bin = find_claude_binary()
            .ok_or_else(|| "claude binary not found on PATH".to_string())?;

        let mut warnings: Vec<String> = Vec::new();

        // 1. Register marketplace + install plugin (handles commands, skills, hooks, MCP)
        let marketplace_source = marketplace_path.unwrap_or(SENSEI_MARKETPLACE_REPO);

        let add_out = std::process::Command::new(&claude_bin)
            .args(["plugin", "marketplace", "add", marketplace_source, "--scope", "user"])
            .output();
        match &add_out {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                if !err.contains("already") {
                    warnings.push(format!("marketplace add: {}", err));
                }
            }
            Err(e) => return Err(format!("marketplace add: {}", e)),
        }

        let install_out = std::process::Command::new(&claude_bin)
            .args(["plugin", "install", "sensei", "--scope", "user"])
            .output();
        match install_out {
            Ok(o) if o.status.success() => {
                return Ok(AcpConfigureOk { plugin: true, warnings });
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                warnings.push(format!("plugin install: {}", err));
            }
            Err(e) => warnings.push(format!("plugin install: {}", e)),
        }

        // 2. Fallback: install hooks + MCP manually
        //    Plugin install failed, so set up hooks via the installer and MCP via CLI/JSON.
        if let Err(e) = crate::installer::install_hooks_only() {
            warnings.push(format!("hooks: {}", e));
        }

        // 3. Try `claude mcp add`
        let mcp_added = std::process::Command::new(&claude_bin)
            .args(["mcp", "add", "-t", "stdio", "-s", "user", "sensei", "--", mcp_cmd])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // 4. Fallback: write ~/.claude.json directly
        if !mcp_added {
            let claude_json = home().join(".claude.json");
            upsert_sensei_in_json(&claude_json, "mcpServers", serde_json::json!({"command": mcp_cmd, "args": []}))?;
        }

        // 5. Write hook config to ~/.claude/settings.json so hooks run even without the plugin
        let hooks_dir = crate::paths::plugin_dir().join("hooks");
        let run_hook = hooks_dir.join("run-hook.cmd");
        if run_hook.exists() {
            let rh = run_hook.to_string_lossy().to_string();
            let hook_entry = |script: &str| -> serde_json::Value {
                serde_json::json!([{
                    "hooks": [{
                        "type": "command",
                        "command": format!("{} {}", rh, script)
                    }]
                }])
            };

            let settings_path = home().join(".claude/settings.json");
            let mut settings: serde_json::Value = settings_path
                .exists()
                .then(|| std::fs::read_to_string(&settings_path).ok())
                .flatten()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::json!({}));

            let hooks = settings
                .as_object_mut()
                .ok_or("invalid settings")?
                .entry("hooks")
                .or_insert(serde_json::json!({}))
                .as_object_mut()
                .ok_or("invalid hooks section")?;

            hooks.insert("SessionStart".into(), hook_entry("session-start"));
            hooks.insert("UserPromptSubmit".into(), hook_entry("user-prompt"));
            hooks.insert("PreCompact".into(), hook_entry("pre-compact"));

            if let Some(parent) = settings_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&settings_path, serde_json::to_string_pretty(&settings).unwrap())
                .map_err(|e| format!("settings.json: {}", e))?;
        }

        Ok(AcpConfigureOk { plugin: false, warnings })
    }

    fn remove(&self) -> bool {
        let h = home();
        let mut removed = false;

        // 1. Try `claude plugin uninstall` (mirrors plugin install path)
        if std::process::Command::new("claude")
            .args(["plugin", "uninstall", "sensei"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            removed = true;
        }

        // 2. Try `claude mcp remove` (covers non-plugin install path)
        if !removed
            && std::process::Command::new("claude")
                .args(["mcp", "remove", "-s", "user", "sensei"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                removed = true;
            }

        // 3. Fallback: remove from ~/.claude.json
        if !removed
            && remove_sensei_from_json(&h.join(".claude.json"), "mcpServers") {
                removed = true;
            }

        // 4. Remove hooks (in case plugin uninstall didn't clean them)
        let hooks_file = h.join(".claude/hooks.json");
        if hooks_file.exists() {
            std::fs::remove_file(&hooks_file).ok();
        }

        removed
    }
}

// ── MCP File ACP ───────────────────────────────────────────────────────────

/// Entry format written to the MCP config file.
enum McpEntryFormat {
    /// `{"command": mcp_cmd, "args": []}` — used by most ACPs
    Standard,
    /// `{"type": "local", "command": [mcp_cmd, ""], "enabled": true}` — OpenCode
    OpenCode,
}

/// Generic ACP that reads/writes a JSON MCP config file.
/// Covers Claude Desktop, Cursor, Windsurf, Zed, Kiro, VS Code, and OpenCode.
struct McpFileAcp {
    id: &'static str,
    name: &'static str,
    mcp_key: &'static str,
    config_rel: &'static str,
    entry_format: McpEntryFormat,
    /// App bundle names to check in /Applications and ~/Applications
    app_names: &'static [&'static str],
    /// Binary names to check on PATH
    bin_names: &'static [&'static str],
    /// Additional paths relative to $HOME to check for detection
    home_paths: &'static [&'static str],
}

impl Acp for McpFileAcp {
    fn id(&self) -> &str { self.id }
    fn name(&self) -> &str { self.name }
    fn mcp_key(&self) -> &str { self.mcp_key }
    fn config_path(&self) -> PathBuf { home().join(self.config_rel) }

    fn detect(&self) -> bool {
        let h = home();
        for app in self.app_names {
            if std::path::Path::new("/Applications").join(app).exists()
                || h.join("Applications").join(app).exists()
            {
                return true;
            }
        }
        for bin in self.bin_names {
            if which_exists(bin) { return true; }
        }
        for path in self.home_paths {
            if h.join(path).exists() { return true; }
        }
        false
    }

    fn configure(&self, mcp_cmd: &str, _marketplace_path: Option<&str>) -> Result<AcpConfigureOk, String> {
        let entry = match self.entry_format {
            McpEntryFormat::Standard => {
                serde_json::json!({"command": mcp_cmd, "args": []})
            }
            McpEntryFormat::OpenCode => {
                serde_json::json!({"type": "local", "command": [mcp_cmd, ""], "enabled": true})
            }
        };
        upsert_sensei_in_json(&self.config_path(), self.mcp_key, entry)?;
        Ok(AcpConfigureOk { plugin: false, warnings: vec![] })
    }

    fn remove(&self) -> bool {
        remove_sensei_from_json(&self.config_path(), self.mcp_key)
    }
}

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

pub fn configure(acp_ids: &[String], marketplace_path: Option<&str>) -> ConfigureResult {
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
        match acp.configure(&mcp_cmd, marketplace_path) {
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

// ── Shared helpers ─────────────────────────────────────────────────────────

fn home() -> PathBuf { crate::paths::home() }

fn which_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

fn check_mcp_configured(config_path: &std::path::Path, mcp_key: &str) -> bool {
    if !config_path.exists() { return false; }
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| v[mcp_key]["sensei"].is_object())
        .unwrap_or(false)
}

fn find_mcp_binary() -> Option<PathBuf> {
    if which_exists("sensei-mcp") {
        return Some(PathBuf::from("sensei-mcp"));
    }
    let search = [
        PathBuf::from("/opt/homebrew/bin/sensei-mcp"),
        PathBuf::from("/usr/local/bin/sensei-mcp"),
    ];
    search.into_iter().find(|p| p.exists())
}

fn find_claude_binary() -> Option<PathBuf> {
    if which_exists("claude") {
        return Some(PathBuf::from("claude"));
    }
    let search = [
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
        home().join(".claude/bin/claude"),
    ];
    search.into_iter().find(|p| p.exists())
}

/// Read a JSON file, remove "sensei" from the object at `mcp_key`, write back.
fn remove_sensei_from_json(path: &std::path::Path, mcp_key: &str) -> bool {
    if !path.exists() { return false; }
    let s = match std::fs::read_to_string(path) { Ok(s) => s, Err(_) => return false };
    let mut v: serde_json::Value = match serde_json::from_str(&s) { Ok(v) => v, Err(_) => return false };
    if let Some(servers) = v.get_mut(mcp_key).and_then(|s| s.as_object_mut())
        && servers.remove("sensei").is_some() {
            std::fs::write(path, serde_json::to_string_pretty(&v).unwrap()).ok();
            return true;
        }
    false
}

/// Write sensei MCP entry into a JSON config file at the given key.
fn upsert_sensei_in_json(
    path: &std::path::Path,
    mcp_key: &str,
    entry: serde_json::Value,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut config: serde_json::Value = path
        .exists()
        .then(|| std::fs::read_to_string(path).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    config
        .as_object_mut()
        .ok_or("invalid config")?
        .entry(mcp_key)
        .or_insert(serde_json::json!({}))
        .as_object_mut()
        .ok_or("invalid mcp section")?
        .insert("sensei".into(), entry);

    std::fs::write(path, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| e.to_string())
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    // Note: ClaudeCodeAcp::configure/unconfigure call `claude` CLI which
    // can't be safely mocked in parallel unit tests (requires PATH mutation).
    // The CLI invocation behavior is tested via the integration/e2e path.
    // Here we test the fallback paths that operate on JSON files.

    #[test]
    fn claude_code_unconfigure_fallback_removes_from_claude_json() {
        let dir = tempfile::tempdir().unwrap();
        let claude_json = dir.path().join(".claude.json");
        std::fs::write(&claude_json, r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"},"svelte":{"command":"npx"}}}"#).unwrap();

        assert!(remove_sensei_from_json(&claude_json, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&claude_json).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn claude_code_configure_fallback_writes_claude_json() {
        let dir = tempfile::tempdir().unwrap();
        let claude_json = dir.path().join(".claude.json");

        upsert_sensei_in_json(&claude_json, "mcpServers", serde_json::json!({"command": "sensei-mcp", "args": []})).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&claude_json).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
    }

    #[test]
    fn claude_code_configure_fallback_preserves_existing_claude_json() {
        let dir = tempfile::tempdir().unwrap();
        let claude_json = dir.path().join(".claude.json");
        std::fs::write(&claude_json, r#"{"mcpServers":{"svelte":{"command":"npx"}},"other":"data"}"#).unwrap();

        upsert_sensei_in_json(&claude_json, "mcpServers", serde_json::json!({"command": "sensei-mcp"})).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&claude_json).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
        assert_eq!(content["other"], "data");
    }
}
