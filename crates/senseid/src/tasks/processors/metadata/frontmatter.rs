//! README frontmatter parser — derives repo identity (organization, workspace,
//! team, role, stack, summary, tagline, icon) from the YAML frontmatter at the
//! top of a repo's root README. Scanning never writes identity back — that is an
//! opt-in sync gated by a preference. The one write a scan makes is D31's
//! one-time key migration ([`migrate_readme_workspace_key`]): a README still
//! grouping its repository with `project:` is rewritten to `workspace:`, the
//! only key the reader understands.

use std::path::{Path, PathBuf};

/// Parsed identity from a repo's README frontmatter. Every field is optional;
/// absent fields fall back to best-guess elsewhere in the scanner.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
pub struct Frontmatter {
    pub organization: Option<String>,
    pub client: Option<String>,
    /// The workspace this repository is grouped into (`workspace:`).
    pub workspace: Option<String>,
    pub team: Option<String>,
    pub role: Option<String>,
    pub stack: Vec<String>,
    pub summary: Option<String>,
    pub tagline: Option<String>,
    /// Default/light icon. A repo-relative path (e.g. `sensei.svg`) or a URL.
    pub icon: Option<String>,
    /// Optional dark-mode icon variant (`icon_dark:` in frontmatter).
    pub icon_dark: Option<String>,
}

/// Parse YAML frontmatter from the top of a markdown document. Returns `None`
/// when there is no `---`-fenced block at the very start, the block is unclosed,
/// or the YAML does not parse.
pub fn parse_frontmatter(content: &str) -> Option<Frontmatter> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut yaml = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed {
        return None;
    }

    let v: serde_json::Value = serde_yaml::from_str(&yaml).ok()?;

    let str_field = |k: &str| {
        v.get(k).and_then(|x| x.as_str()).map(|x| x.trim().to_string()).filter(|x| !x.is_empty())
    };
    let stack = match v.get("stack") {
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .filter_map(|e| e.as_str())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(serde_json::Value::String(s)) => {
            s.split(',').map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty()).collect()
        }
        _ => Vec::new(),
    };

    Some(Frontmatter {
        organization: str_field("organization"),
        client: str_field("client"),
        workspace: str_field("workspace"),
        team: str_field("team"),
        role: str_field("role"),
        stack,
        summary: str_field("summary"),
        tagline: str_field("tagline"),
        icon: str_field("icon"),
        icon_dark: str_field("icon_dark"),
    })
}

/// Classify an icon reference as a remote URL (vs a repo-relative path) so the
/// consumer knows whether to load it directly or resolve it against the repo.
pub fn icon_is_url(path: &str) -> bool {
    let p = path.trim();
    p.starts_with("http://")
        || p.starts_with("https://")
        || p.starts_with("//")
        || p.starts_with("data:")
}

/// Map a free-text frontmatter `role` to a canonical `folder_role` enum value,
/// accepting common synonyms. Returns `None` for values that aren't generic
/// folder roles (e.g. project-specific `marketplace`, or structural `monorepo`)
/// — those are kept only as a `role:<raw>` project tag by the caller.
pub fn folder_role_from_frontmatter(role: &str) -> Option<&'static str> {
    match role.trim().to_ascii_lowercase().as_str() {
        "backend" | "server" | "api" | "service" | "daemon" => Some("backend"),
        "frontend" | "ui" | "client" => Some("frontend"),
        "library" | "lib" | "package" | "sdk" => Some("library"),
        "tool" | "cli" | "command" | "binary" => Some("tool"),
        "docs" | "doc" | "documentation" => Some("docs"),
        "infra" | "infrastructure" | "ops" | "devops" | "ci" => Some("infra"),
        // "app" is a (web) application surface → website; a native desktop app
        // must declare `role: desktop` explicitly.
        "website" | "site" | "web" | "marketing" | "app" => Some("website"),
        "desktop" => Some("desktop"),
        "mobile" | "ios" | "android" => Some("mobile"),
        "config" | "configuration" | "settings" => Some("config"),
        "packaging" | "tap" | "homebrew" | "brew" | "formula" | "distribution" => Some("packaging"),
        // monorepo (structural), marketplace/extensions (project-specific, rare),
        // and anything unrecognised → no enum role; kept as a tag.
        _ => None,
    }
}

/// Normalize a name into a slug (lowercase, non-alphanumeric runs collapsed to
/// a single `-`, trimmed). Used as the unique key for a namespace within a scope.
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Split a markdown document into its leading YAML frontmatter mapping (empty
/// if none) and the body that follows. Tolerates BOM and CRLF. Used by
/// [`merge_frontmatter`] so unmanaged keys + the body survive a write-back.
fn split_frontmatter(content: &str) -> (serde_yaml::Mapping, String) {
    let c = content.strip_prefix('\u{feff}').unwrap_or(content).replace("\r\n", "\n");
    if let Some(rest) = c.strip_prefix("---\n") {
        // Closing fence is a line that is exactly `---`: match `\n---\n` (body
        // follows) or `\n---` at end of file.
        if let Some(idx) = rest.find("\n---\n") {
            let map = serde_yaml::from_str::<serde_yaml::Mapping>(&rest[..idx]).unwrap_or_default();
            return (map, rest[idx + 5..].to_string());
        }
        if let Some(stripped) = rest.strip_suffix("\n---") {
            let map = serde_yaml::from_str::<serde_yaml::Mapping>(stripped).unwrap_or_default();
            return (map, String::new());
        }
    }
    (serde_yaml::Mapping::new(), c)
}

/// Merge sensei's managed identity fields (organization, client, workspace, team,
/// role, stack, summary, tagline, icon, icon_dark) into a README's YAML
/// frontmatter, preserving the body and any unmanaged keys the user added, and
/// writing a `# sensei:frontmatter` marker as the first line of the block. Only
/// fields that are `Some`/non-empty are written (a cleared field is left as-is
/// rather than emitting an empty value). Pure — returns the full new README
/// content.
pub fn merge_frontmatter(existing: &str, fm: &Frontmatter) -> String {
    use serde_yaml::Value;
    // A README still on `project:` is migrated first, so the old key cannot
    // linger beside the `workspace:` this writes.
    let migrated = migrate_workspace_key(existing);
    let (mut map, body) = split_frontmatter(migrated.as_deref().unwrap_or(existing));

    let mut set = |key: &str, val: &Option<String>| {
        if let Some(v) = val.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            map.insert(Value::from(key), Value::from(v));
        }
    };
    set("organization", &fm.organization);
    set("client", &fm.client);
    set("workspace", &fm.workspace);
    set("team", &fm.team);
    set("role", &fm.role);
    set("summary", &fm.summary);
    set("tagline", &fm.tagline);
    set("icon", &fm.icon);
    set("icon_dark", &fm.icon_dark);
    if !fm.stack.is_empty() {
        let seq: Vec<Value> = fm.stack.iter().map(|s| Value::from(s.as_str())).collect();
        map.insert(Value::from("stack"), Value::Sequence(seq));
    }

    let yaml = serde_yaml::to_string(&map).unwrap_or_default();
    // serde_yaml emits a trailing newline after the mapping; the body carries its own.
    format!("---\n# sensei:frontmatter\n{yaml}---\n{body}")
}

/// A repo's root README, if it has one — the file every frontmatter read and
/// write resolves to.
pub fn find_readme(repo_path: &Path) -> Option<PathBuf> {
    ["README.md", "readme.md", "Readme.md", "README"]
        .iter()
        .map(|name| repo_path.join(name))
        .find(|p| p.is_file())
}

/// Read and parse frontmatter from a repo's root README, if present.
pub fn read_frontmatter(repo_path: &Path) -> Option<Frontmatter> {
    let content = std::fs::read_to_string(find_readme(repo_path)?).ok()?;
    parse_frontmatter(&content)
}

/// D31 (user 2026-10-21): the frontmatter key that groups a repository into a
/// workspace was `project:`; it is `workspace:`. Returns the document with a
/// top-level `project:` line in its leading frontmatter block renamed to
/// `workspace:` — every other byte (the value, a trailing comment, line
/// endings, the body) unchanged — or `None` when there is nothing to migrate:
/// no frontmatter block, no top-level `project:` key in it, or a `workspace:`
/// key already there. Pure.
pub fn migrate_workspace_key(content: &str) -> Option<String> {
    const OLD: &str = "project";
    let (bom, rest) = match content.strip_prefix('\u{feff}') {
        Some(r) => ("\u{feff}", r),
        None => ("", content),
    };
    let lines: Vec<&str> = rest.split_inclusive('\n').collect();
    if lines.first()?.trim() != "---" {
        return None;
    }
    let close = lines.iter().skip(1).position(|l| l.trim() == "---")? + 1;
    let block = &lines[1..close];
    let is_key = |line: &str, key: &str| {
        line.strip_prefix(key).is_some_and(|r| r.trim_start_matches([' ', '\t']).starts_with(':'))
    };
    if block.iter().any(|l| is_key(l, "workspace")) {
        return None;
    }
    let at = block.iter().position(|l| is_key(l, OLD))? + 1;
    let mut out = String::with_capacity(content.len() + 2);
    out.push_str(bom);
    for (i, line) in lines.iter().enumerate() {
        if i == at {
            out.push_str("workspace");
            out.push_str(&line[OLD.len()..]);
        } else {
            out.push_str(line);
        }
    }
    Some(out)
}

/// Run D31's one-time migration on a repo's root README: rewrite its
/// `project:` frontmatter key to `workspace:` in place. Returns whether it
/// wrote. A README already on `workspace:` (or with no such key, or no README)
/// is not written at all. Fails when the README cannot be read or written —
/// the caller must not read on as if the repository had no workspace key.
pub fn migrate_readme_workspace_key(repo_path: &Path) -> Result<bool, String> {
    let Some(readme) = find_readme(repo_path) else {
        return Ok(false);
    };
    let content =
        std::fs::read_to_string(&readme).map_err(|e| format!("read {}: {e}", readme.display()))?;
    let Some(migrated) = migrate_workspace_key(&content) else {
        return Ok(false);
    };
    std::fs::write(&readme, migrated).map_err(|e| format!("write {}: {e}", readme.display()))?;
    tracing::info!(readme = %readme.display(), "README frontmatter: project: → workspace: (D31)");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_frontmatter() {
        let md = "---\norganization: Sensei HQ\nworkspace: sensei\nrole: monorepo\nstack: [rust, SvelteKit]\nsummary: A one-line summary.\ntagline: A tagline\nicon: sensei.svg\n---\n\n# Sensei\nbody";
        let fm = parse_frontmatter(md).unwrap();
        assert_eq!(fm.organization.as_deref(), Some("Sensei HQ"));
        assert_eq!(fm.workspace.as_deref(), Some("sensei"));
        assert_eq!(fm.role.as_deref(), Some("monorepo"));
        assert_eq!(fm.stack, vec!["rust", "sveltekit"]);
        assert_eq!(fm.summary.as_deref(), Some("A one-line summary."));
        assert_eq!(fm.tagline.as_deref(), Some("A tagline"));
        assert_eq!(fm.icon.as_deref(), Some("sensei.svg"));
    }

    #[test]
    fn none_without_frontmatter() {
        assert!(parse_frontmatter("# Title\n\nNo frontmatter here.").is_none());
    }

    #[test]
    fn none_on_unclosed_block() {
        assert!(parse_frontmatter("---\nworkspace: x\n# never closed").is_none());
    }

    #[test]
    fn stack_accepts_comma_string() {
        let fm = parse_frontmatter("---\nstack: rust, TypeScript\n---\n").unwrap();
        assert_eq!(fm.stack, vec!["rust", "typescript"]);
    }

    #[test]
    fn icon_variants_and_url_classification() {
        let fm = parse_frontmatter("---\nicon: logo.svg\nicon_dark: logo-dark.svg\n---\n").unwrap();
        assert_eq!(fm.icon.as_deref(), Some("logo.svg"));
        assert_eq!(fm.icon_dark.as_deref(), Some("logo-dark.svg"));
        assert!(!icon_is_url("logo.svg"));
        assert!(!icon_is_url("assets/icon.png"));
        assert!(icon_is_url("https://cdn.example.com/i.svg"));
        assert!(icon_is_url("//cdn.example.com/i.svg"));
        assert!(icon_is_url("data:image/svg+xml;base64,AAAA"));
    }

    #[test]
    fn folder_role_maps_synonyms_and_skips_specific() {
        assert_eq!(folder_role_from_frontmatter("desktop"), Some("desktop"));
        // "app" is a web-application surface → website (desktop must be explicit).
        assert_eq!(folder_role_from_frontmatter("App"), Some("website"));
        assert_eq!(folder_role_from_frontmatter("website"), Some("website"));
        assert_eq!(folder_role_from_frontmatter("tool"), Some("tool"));
        assert_eq!(folder_role_from_frontmatter("CLI"), Some("tool"));
        assert_eq!(folder_role_from_frontmatter("homebrew"), Some("packaging"));
        assert_eq!(folder_role_from_frontmatter("  Library "), Some("library"));
        assert_eq!(folder_role_from_frontmatter("mobile"), Some("mobile"));
        // project-specific / structural → no enum role (kept as a tag)
        assert_eq!(folder_role_from_frontmatter("monorepo"), None);
        assert_eq!(folder_role_from_frontmatter("marketplace"), None);
        assert_eq!(folder_role_from_frontmatter("whatever"), None);
    }

    #[test]
    fn slugify_normalizes() {
        assert_eq!(slugify("Sensei HQ"), "sensei-hq");
        assert_eq!(slugify("rust"), "rust");
        assert_eq!(slugify("  A / B  C "), "a-b-c");
        assert_eq!(slugify("C++"), "c");
    }

    #[test]
    fn merge_into_readme_without_frontmatter_prepends_block() {
        let fm = Frontmatter {
            workspace: Some("sensei".into()),
            role: Some("monorepo".into()),
            stack: vec!["rust".into(), "svelte".into()],
            ..Default::default()
        };
        let out = merge_frontmatter("# Title\n\nBody text.\n", &fm);
        assert!(out.starts_with("---\n# sensei:frontmatter\n"), "marker comment first");
        assert!(out.contains("workspace: sensei"));
        assert!(out.contains("role: monorepo"));
        assert!(out.ends_with("# Title\n\nBody text.\n"), "body preserved verbatim");
        // Round-trips back through the parser.
        let parsed = parse_frontmatter(&out).unwrap();
        assert_eq!(parsed.workspace.as_deref(), Some("sensei"));
        assert_eq!(parsed.stack, vec!["rust", "svelte"]);
    }

    #[test]
    fn merge_preserves_unmanaged_keys_and_body_and_overwrites_managed() {
        let existing = "---\nworkspace: oldname\ncustom_key: keep-me\n---\n# Heading\n\nProse.\n";
        let fm = Frontmatter { workspace: Some("newname".into()), ..Default::default() };
        let out = merge_frontmatter(existing, &fm);
        let parsed = parse_frontmatter(&out).unwrap();
        assert_eq!(parsed.workspace.as_deref(), Some("newname"), "managed key overwritten");
        assert!(out.contains("custom_key: keep-me"), "unmanaged key preserved");
        assert!(out.contains("# Heading\n\nProse.\n"), "body preserved");
        assert!(out.contains("# sensei:frontmatter"), "marker added");
    }

    #[test]
    fn merge_skips_empty_fields() {
        // An all-empty frontmatter should not emit blank keys, just the marker.
        let out = merge_frontmatter("body only\n", &Frontmatter::default());
        assert!(!out.contains("workspace:"));
        assert!(out.contains("# sensei:frontmatter"));
        assert!(out.ends_with("body only\n"));
    }

    #[test]
    fn partial_frontmatter_leaves_others_unset() {
        let fm = parse_frontmatter("---\nworkspace: solo\n---\n").unwrap();
        assert_eq!(fm.workspace.as_deref(), Some("solo"));
        assert!(fm.organization.is_none());
        assert!(fm.stack.is_empty());
    }

    // ── D31: the `project:` key migrates to `workspace:` in place ──────────────

    #[test]
    fn the_reader_understands_only_workspace() {
        let fm = parse_frontmatter("---\nproject: old\n---\n").unwrap();
        assert_eq!(fm.workspace, None, "no dual-read: `project:` is not a workspace");
    }

    #[test]
    fn a_project_line_in_the_frontmatter_becomes_workspace_and_nothing_else_moves() {
        let before = "---\n# sensei:frontmatter\norganization: Acme\nproject: Ledger  # grouped\nrole: backend\n---\n# Ledger\n\nproject: in the body stays\n";
        let after = migrate_workspace_key(before).expect("a project: key is migrated");
        assert_eq!(
            after,
            "---\n# sensei:frontmatter\norganization: Acme\nworkspace: Ledger  # grouped\nrole: backend\n---\n# Ledger\n\nproject: in the body stays\n"
        );
        assert_eq!(parse_frontmatter(&after).unwrap().workspace.as_deref(), Some("Ledger"));
    }

    #[test]
    fn crlf_and_a_bom_survive_the_migration() {
        let before = "\u{feff}---\r\nproject: X\r\n---\r\nbody\r\n";
        assert_eq!(
            migrate_workspace_key(before).as_deref(),
            Some("\u{feff}---\r\nworkspace: X\r\n---\r\nbody\r\n")
        );
    }

    #[test]
    fn a_readme_already_on_workspace_is_left_alone() {
        assert_eq!(migrate_workspace_key("---\nworkspace: X\n---\nbody\n"), None);
        // Both present: the reader already has its answer; nothing is rewritten.
        assert_eq!(migrate_workspace_key("---\nworkspace: X\nproject: Y\n---\n"), None);
    }

    #[test]
    fn a_project_key_outside_the_frontmatter_block_is_never_touched() {
        // In the body only.
        assert_eq!(migrate_workspace_key("---\norganization: o\n---\nproject: x\n"), None);
        // No frontmatter at all.
        assert_eq!(migrate_workspace_key("project: x\n"), None);
        assert_eq!(migrate_workspace_key("# Title\n\nproject: x\n"), None);
        // An unclosed block is not frontmatter.
        assert_eq!(migrate_workspace_key("---\nproject: x\n# never closed\n"), None);
        // Nested under another key, or a longer key that starts with the word.
        assert_eq!(migrate_workspace_key("---\nlinks:\n  project: x\n---\n"), None);
        assert_eq!(migrate_workspace_key("---\nproject_url: x\n---\n"), None);
    }

    #[test]
    fn the_writer_writes_workspace_over_a_readme_still_on_project() {
        let fm = Frontmatter { workspace: Some("New".into()), ..Default::default() };
        let out = merge_frontmatter("---\nproject: Old\n---\nbody\n", &fm);
        assert!(out.contains("workspace: New"), "{out}");
        assert!(!out.contains("project:"), "the old key does not linger beside the new: {out}");
    }

    #[test]
    fn the_readme_file_is_rewritten_once_then_left_byte_identical() {
        let t = tempfile::tempdir().unwrap();
        let readme = t.path().join("README.md");
        std::fs::write(&readme, "---\nproject: X\n---\n# X\n").unwrap();
        assert!(migrate_readme_workspace_key(t.path()).unwrap(), "first scan rewrites");
        assert_eq!(std::fs::read_to_string(&readme).unwrap(), "---\nworkspace: X\n---\n# X\n");
        assert!(!migrate_readme_workspace_key(t.path()).unwrap(), "second scan writes nothing");
        assert_eq!(std::fs::read_to_string(&readme).unwrap(), "---\nworkspace: X\n---\n# X\n");
        // No README at all: nothing to migrate, not an error.
        let empty = tempfile::tempdir().unwrap();
        assert!(!migrate_readme_workspace_key(empty.path()).unwrap());
    }
}
