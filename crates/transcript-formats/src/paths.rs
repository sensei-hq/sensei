//! Resolving the paths and URIs a VS Code transcript refers to.

use std::path::Path;

/// Decode `%XX` escapes and unwrap a Windows drive.
///
/// VS Code stores Windows folders as `file:///c%3A/Users/...`. Stripping the
/// scheme alone leaves `/c%3A/Users/...`, which matches no directory and no
/// repo — every Windows session would lose its project attribution.
pub fn normalise_uri_path(path: &str) -> String {
    let out = percent_decode(path);
    // `/c:/Users/...` → `c:/Users/...`
    let trimmed = out.strip_prefix('/').unwrap_or(&out);
    if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
        return trimmed.to_string();
    }
    out
}

/// Decode `%XX` escapes, leaving everything else alone.
pub fn percent_decode(s: &str) -> String {
    let raw = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%'
            && i + 2 < raw.len()
            && let (Some(h), Some(l)) =
                ((raw[i + 1] as char).to_digit(16), (raw[i + 2] as char).to_digit(16))
        {
            out.push(((h * 16 + l) as u8) as char);
            i += 3;
            continue;
        }
        out.push(raw[i] as char);
        i += 1;
    }
    out
}

/// The project folder a chat session belongs to.
///
/// `chat_session_path` is the journal file itself, at
/// `<workspace-hash>/chatSessions/<id>.jsonl`, and `workspace.json` sits beside
/// the `chatSessions` DIRECTORY — two levels up, not one. Reading one level up
/// looks for `<hash>/chatSessions/workspace.json`, which does not exist, so the
/// folder resolves to `None` and the session loses its project attribution
/// entirely.
pub fn workspace_folder(chat_session_path: &Path) -> Option<String> {
    let ws = chat_session_path.parent()?.parent()?.join("workspace.json");
    let content = std::fs::read_to_string(&ws).ok()?;
    folder_from_workspace_json(&content)
}

/// The folder URI inside a `workspace.json`, resolved to a path.
///
/// Split out from [`workspace_folder`] so the shape handling is testable without
/// a filesystem.
pub fn folder_from_workspace_json(content: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    // Three shapes in the wild: a plain string, and two `$mid`-tagged objects
    // VS Code writes for some workspaces. Accept any.
    let uri = v
        .get("folder")
        .and_then(|f| f.as_str().map(str::to_string))
        .or_else(|| v["workspace"]["folder"]["path"].as_str().map(str::to_string))
        .or_else(|| v["folder"]["path"].as_str().map(str::to_string))?;

    if let Some(path) = uri.strip_prefix("file://") {
        return Some(normalise_uri_path(path));
    }
    // vscode-remote://wsl+<distro>/path — keep the path, drop the authority.
    if let Some(rest) = uri.strip_prefix("vscode-remote://")
        && let Some(slash) = rest.find('/')
    {
        return Some(rest[slash..].to_string());
    }
    Some(uri)
}

/// Every `file:///` path mentioned in a rendered message.
///
/// The journal records no tool arguments, but it renders each call into prose
/// that embeds the file as a link — `Reading [](file:///c%3A/...)`. That is the
/// only place a journal names the file a call touched.
pub fn file_uris(message: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = message;
    while let Some(start) = rest.find("file:///") {
        let tail = &rest[start + "file:///".len()..];
        // The URI runs to the first character that cannot appear in one.
        let end = tail.find([')', ' ', '"', '\'', '>', '\n']).unwrap_or(tail.len());
        let uri = percent_decode(&tail[..end]);
        if !uri.is_empty() {
            out.push(uri);
        }
        rest = &tail[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_windows_drive_is_unwrapped() {
        assert_eq!(normalise_uri_path("/c%3A/Users/r/app"), "c:/Users/r/app");
        assert_eq!(normalise_uri_path("/home/j/repo"), "/home/j/repo");
    }

    /// `workspace.json` sits beside the chatSessions DIRECTORY, so resolution
    /// has to climb two levels. One level finds nothing and the session loses
    /// its project.
    #[test]
    fn workspace_json_is_two_levels_up_from_the_journal() {
        let dir = std::env::temp_dir().join("stf-workspace-depth/hash1");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        std::fs::create_dir_all(dir.join("chatSessions")).unwrap();
        std::fs::write(dir.join("workspace.json"), r#"{"folder":"file:///repo"}"#).unwrap();
        let journal = dir.join("chatSessions").join("s.jsonl");
        std::fs::write(&journal, "").unwrap();

        assert_eq!(workspace_folder(&journal), Some("/repo".to_string()));
    }

    #[test]
    fn the_mid_tagged_object_shape_is_accepted() {
        assert_eq!(
            folder_from_workspace_json(r#"{"folder":{"$mid":1,"path":"/repo/a"}}"#),
            Some("/repo/a".into())
        );
        assert_eq!(
            folder_from_workspace_json(r#"{"workspace":{"folder":{"path":"/repo/b"}}}"#),
            Some("/repo/b".into())
        );
    }

    #[test]
    fn a_remote_uri_keeps_its_path() {
        assert_eq!(
            folder_from_workspace_json(r#"{"folder":"vscode-remote://wsl+ubuntu/home/j/x"}"#),
            Some("/home/j/x".into())
        );
    }

    #[test]
    fn file_links_are_lifted_out_of_rendered_prose() {
        let msg = "Reading [](file:///c%3A/Users/r/app/src/main.ts) and done";
        assert_eq!(file_uris(msg), vec!["c:/Users/r/app/src/main.ts"]);
        assert!(file_uris("Searching for `**/src/**`").is_empty());
    }
}
