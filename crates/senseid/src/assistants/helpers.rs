use std::path::PathBuf;

pub(crate) fn home() -> PathBuf { crate::paths::home() }

pub(crate) fn which_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

pub(crate) fn check_mcp_configured(config_path: &std::path::Path, mcp_key: &str) -> bool {
    if !config_path.exists() { return false; }
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| v[mcp_key]["sensei"].is_object())
        .unwrap_or(false)
}

pub(crate) fn find_mcp_binary() -> Option<PathBuf> {
    if which_exists("sensei-mcp") {
        return Some(PathBuf::from("sensei-mcp"));
    }
    let search = [
        PathBuf::from("/opt/homebrew/bin/sensei-mcp"),
        PathBuf::from("/usr/local/bin/sensei-mcp"),
    ];
    search.into_iter().find(|p| p.exists())
}

pub(crate) fn find_claude_binary() -> Option<PathBuf> {
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
pub(crate) fn remove_sensei_from_json(path: &std::path::Path, mcp_key: &str) -> bool {
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
