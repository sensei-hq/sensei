use sensei_bootstrap::{MCP_REGISTRY_KEY, SENSEI_MCP_BIN};
use std::path::PathBuf;

pub(crate) fn home() -> PathBuf {
    crate::paths::home()
}

/// Default integration check for assistants that wire sensei as an MCP
/// server — Cursor, Zed, etc. Looks for the sensei MCP entry in the
/// assistant's config file. Claude Code overrides this with a plugin-presence
/// check (`is_configured()` in claude_code.rs) since it integrates via
/// `claude plugin install`, not via the settings.json MCP block.
pub(crate) fn check_mcp_in_config(config_path: &std::path::Path, mcp_key: &str) -> bool {
    if !config_path.exists() {
        return false;
    }
    // Uses json5 (not serde_json) so files with a preserved JSONC
    // header — either the user's original comments (Zed's shipping
    // `settings.json`) or ones we round-tripped through
    // [`upsert_sensei_in_json`] — still parse. Strict serde_json here
    // would silently report "not configured" right after a successful
    // upsert on any JSONC file.
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| json5::from_str::<serde_json::Value>(&s).ok())
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

/// Capture the file prefix before the first `{` — leading whitespace,
/// `//` line comments, and `/* */` block comments. Re-prepended on
/// write so JSONC configs with a header (Zed's shipping
/// `settings.json` is `// Folder-specific settings ...`) don't lose
/// their preamble when we round-trip through `serde_json`. Inline
/// comments inside the JSON body still get stripped — a CST-preserving
/// editor is the full fix (#51); this is the pragmatic option A the
/// ticket names.
fn leading_jsonc_prefix(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'{' || c == b'[' {
            return &s[..i];
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next == b'/' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if next == b'*' {
                i += 2;
                let mut closed = false;
                while i + 1 < bytes.len() {
                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        i += 2;
                        closed = true;
                        break;
                    }
                    i += 1;
                }
                if !closed {
                    // Unterminated block comment — treat the rest of the
                    // file as prefix; the JSON parser will reject it
                    // separately.
                    i = bytes.len();
                }
                continue;
            }
        }
        break;
    }
    &s[..i]
}

/// Read a JSON/JSONC file, remove the sensei MCP registry entry from the
/// object at `mcp_key`, write back. The entry name is mode-aware via
/// [`MCP_REGISTRY_KEY`] — dev runs target `"sensei-dev"`, prod `"sensei"`.
pub(crate) fn remove_sensei_from_json(path: &std::path::Path, mcp_key: &str) -> bool {
    if !path.exists() {
        return false;
    }
    let raw = std::fs::read_to_string(path).ok().unwrap_or_default();
    let mut v = match json5::from_str::<serde_json::Value>(&raw) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if let Some(servers) = v.get_mut(mcp_key).and_then(|s| s.as_object_mut())
        && servers.remove(MCP_REGISTRY_KEY).is_some()
    {
        let prefix = leading_jsonc_prefix(&raw);
        let body = serde_json::to_string_pretty(&v).unwrap();
        let out = format!("{prefix}{body}");
        if let Err(e) = std::fs::write(path, out) {
            tracing::warn!(error = %e, path = %path.display(), "remove_sensei_from_json: failed to write config back");
        }
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
    let raw = if path.exists() {
        std::fs::read_to_string(path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    let mut config: serde_json::Value = if raw.is_empty() {
        serde_json::json!({})
    } else {
        json5::from_str::<serde_json::Value>(&raw).unwrap_or(serde_json::json!({}))
    };

    config
        .as_object_mut()
        .ok_or("invalid config")?
        .entry(mcp_key)
        .or_insert(serde_json::json!({}))
        .as_object_mut()
        .ok_or("invalid mcp section")?
        .insert(MCP_REGISTRY_KEY.into(), entry);

    let prefix = leading_jsonc_prefix(&raw);
    let body = serde_json::to_string_pretty(&config).unwrap();
    let out = format!("{prefix}{body}");
    std::fs::write(path, out).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_prefix_captures_line_comments_and_ws() {
        let s = "// Zed settings\n// second line\n\n{\"a\":1}";
        let prefix = leading_jsonc_prefix(s);
        assert_eq!(prefix, "// Zed settings\n// second line\n\n");
    }

    #[test]
    fn leading_prefix_captures_block_comment() {
        let s = "/* header\n * lines\n */\n{\"a\":1}";
        let prefix = leading_jsonc_prefix(s);
        assert_eq!(prefix, "/* header\n * lines\n */\n");
    }

    #[test]
    fn leading_prefix_captures_mixed_comments_and_arbitrary_ws() {
        let s = "\n\n// one\n  \t/* two */\n// three\n{\"a\":1}";
        let prefix = leading_jsonc_prefix(s);
        assert_eq!(prefix, "\n\n// one\n  \t/* two */\n// three\n");
    }

    #[test]
    fn leading_prefix_empty_when_json_starts_immediately() {
        assert_eq!(leading_jsonc_prefix("{\"a\":1}"), "");
    }

    #[test]
    fn leading_prefix_stops_at_top_level_array() {
        // Not our config shape today, but the walker shouldn't run off the
        // end into the JSON body when the top-level value is an array.
        let s = "// header\n[1,2,3]";
        assert_eq!(leading_jsonc_prefix(s), "// header\n");
    }

    #[test]
    fn leading_prefix_handles_unterminated_block_comment_without_panicking() {
        // A malformed file (unterminated `/*`) shouldn't crash the walker.
        // The parse step will reject the file separately; the prefix helper
        // just needs to be safe.
        let s = "/* never ends";
        let prefix = leading_jsonc_prefix(s);
        assert_eq!(prefix, s);
    }

    #[test]
    fn upsert_preserves_leading_line_comment_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            "// Zed settings — full reference at https://zed.dev/docs/configuring-zed\n\
             // Auto-generated on first run.\n\
             {\"terminal\": {\"dock\": \"right\"}, \"mcpServers\": {}}",
        )
        .unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(
            written.contains("// Zed settings — full reference at"),
            "leading line-comment header must survive round-trip; got:\n{written}"
        );
        assert!(
            written.contains("// Auto-generated on first run."),
            "second header line must survive; got:\n{written}"
        );
    }

    #[test]
    fn upsert_preserves_leading_block_comment_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            "/*\n * Cursor MCP config — see docs/mcp.md\n * DO NOT edit generated blocks.\n */\n\
             {\"mcpServers\": {}}",
        )
        .unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(
            written.contains("DO NOT edit generated blocks."),
            "leading block-comment header must survive round-trip; got:\n{written}"
        );
    }

    #[test]
    fn remove_preserves_leading_comment_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let body = format!(
            "// user header\n{{\"mcpServers\": {{\"{}\": {{\"command\": \"sensei-mcp\"}}, \"other\": {{\"command\": \"o\"}}}}}}",
            sensei_bootstrap::MCP_REGISTRY_KEY
        );
        std::fs::write(&path, &body).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(
            written.starts_with("// user header\n"),
            "leading comment must survive remove; got:\n{written}"
        );
    }
}
