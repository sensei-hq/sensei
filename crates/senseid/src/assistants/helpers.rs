use std::path::PathBuf;
use sensei_bootstrap::{SENSEI_MCP_BIN, MCP_REGISTRY_KEY};

pub(crate) fn home() -> PathBuf { crate::paths::home() }

pub(crate) fn check_mcp_configured(config_path: &std::path::Path, mcp_key: &str) -> bool {
    if !config_path.exists() { return false; }
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| v[mcp_key][MCP_REGISTRY_KEY].is_object())
        .unwrap_or(false)
}

pub(crate) fn find_mcp_binary() -> Option<PathBuf> {
    sensei_bootstrap::util::which_binary(SENSEI_MCP_BIN).map(PathBuf::from)
}

pub(crate) fn find_claude_binary() -> Option<PathBuf> {
    if let Some(p) = sensei_bootstrap::util::which_binary("claude") {
        return Some(PathBuf::from(p));
    }
    // Extra fallback: Claude installs to ~/.claude/bin/ which may not be in PATH
    let fallback = home().join(".claude/bin/claude");
    fallback.exists().then_some(fallback)
}

/// Parse a JSON or JSONC (JSON with comments) file into a serde_json::Value.
/// Uses json5 to handle comments and trailing commas (e.g. Zed settings.json).
/// Returns an empty object if the file is missing or unparseable.
fn read_json_or_jsonc(path: &std::path::Path) -> Option<serde_json::Value> {
    let s = std::fs::read_to_string(path).ok()?;
    // json5 is a superset of JSONC — handles // and /* */ comments, trailing commas.
    json5::from_str::<serde_json::Value>(&s).ok()
}

/// Read a JSON/JSONC file, remove the sensei MCP registry entry from the
/// object at `mcp_key`, write back. The entry name is mode-aware via
/// [`MCP_REGISTRY_KEY`] — dev runs target `"sensei-dev"`, prod `"sensei"`.
pub(crate) fn remove_sensei_from_json(path: &std::path::Path, mcp_key: &str) -> bool {
    if !path.exists() { return false; }
    let mut v = match read_json_or_jsonc(path) { Some(v) => v, None => return false };
    if let Some(servers) = v.get_mut(mcp_key).and_then(|s| s.as_object_mut())
        && servers.remove(MCP_REGISTRY_KEY).is_some() {
            std::fs::write(path, serde_json::to_string_pretty(&v).unwrap()).ok();
            return true;
        }
    false
}

/// Write the sensei MCP entry into a JSON/JSONC config file at the given key.
/// Preserves all existing keys — only adds or updates the current-mode entry
/// (key from [`MCP_REGISTRY_KEY`]).
pub(crate) fn upsert_sensei_in_json(
    path: &std::path::Path,
    mcp_key: &str,
    entry: serde_json::Value,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut config: serde_json::Value = path
        .exists()
        .then(|| read_json_or_jsonc(path))
        .flatten()
        .unwrap_or(serde_json::json!({}));

    config
        .as_object_mut()
        .ok_or("invalid config")?
        .entry(mcp_key)
        .or_insert(serde_json::json!({}))
        .as_object_mut()
        .ok_or("invalid mcp section")?
        .insert(MCP_REGISTRY_KEY.into(), entry);

    std::fs::write(path, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| e.to_string())
}
