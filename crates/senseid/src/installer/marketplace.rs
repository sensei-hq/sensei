//! Marketplace directory operations — fetch catalog, install skills/commands,
//! clean up stale items.

use std::fs;

use super::{cache_dir, home, Catalog, CatalogItem};
use super::catalog::{fetch_catalog, load_or_download, save_marketplace_version};

// ── Marketplace install ──────────────────────────────────────────────────────

/// Fetch catalog, download items, install skills & commands, clean up stale items.
/// Returns (skills_count, commands_count, stale_commands, stale_skills, version).
pub(super) fn install_marketplace(
    scope: &str,
    acps: &[String],
) -> Result<(u32, u32, u32, u32, String), String> {
    let catalog = fetch_catalog()?;
    let version = catalog.version.clone().unwrap_or_default();

    let supports_skills = acps.iter().any(|a| a == "claude-code");
    let items: Vec<&CatalogItem> = catalog
        .items
        .iter()
        .filter(|i| scope == "all" || i.scope == scope || i.scope == "global")
        .filter(|i| match i.kind.as_str() {
            "skill" | "command" => supports_skills,
            _ => false,
        })
        .collect();

    let h = home();
    let cache = cache_dir();
    let mut skills = 0u32;
    let mut commands = 0u32;

    for item in &items {
        let content = load_or_download(&cache, &item.path)?;

        let dest = match item.kind.as_str() {
            "skill" => {
                skills += 1;
                h.join(".claude/skills").join(format!("{}.md", item.name))
            }
            "command" => {
                commands += 1;
                h.join(".claude/commands").join(format!("{}.md", item.name))
            }
            _ => continue,
        };
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&dest, &content).map_err(|e| e.to_string())?;
    }

    // Clean up stale items from previous versions
    let (stale_cmds, stale_skills) = cleanup_stale_items(&catalog);

    // Save version to ~/.sensei/config.json
    save_marketplace_version(&version);

    Ok((skills, commands, stale_cmds, stale_skills, version))
}

/// Remove command/skill files that are no longer in the catalog.
fn cleanup_stale_items(catalog: &Catalog) -> (u32, u32) {
    let h = home();
    let command_names: std::collections::HashSet<String> = catalog
        .items
        .iter()
        .filter(|i| i.kind == "command")
        .map(|i| format!("{}.md", i.name))
        .collect();
    let skill_names: std::collections::HashSet<String> = catalog
        .items
        .iter()
        .filter(|i| i.kind == "skill")
        .map(|i| format!("{}.md", i.name))
        .collect();

    let commands_removed = remove_stale_in(&h.join(".claude/commands"), &command_names);
    let skills_removed = remove_stale_in(&h.join(".claude/skills"), &skill_names);
    (commands_removed, skills_removed)
}

/// Remove .md files in `dir` whose names are not in `keep`.
fn remove_stale_in(dir: &std::path::Path, keep: &std::collections::HashSet<String>) -> u32 {
    let mut removed = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") && !keep.contains(&name)
                && fs::remove_file(entry.path()).is_ok() {
                    removed += 1;
                }
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn remove_stale_in_removes_unlisted_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        fs::write(path.join("keep.md"), "keep").unwrap();
        fs::write(path.join("stale.md"), "stale").unwrap();
        fs::write(path.join("also-stale.md"), "stale").unwrap();
        fs::write(path.join("not-md.txt"), "skip").unwrap();

        let keep: std::collections::HashSet<String> = ["keep.md".to_string()].into();
        let removed = remove_stale_in(path, &keep);
        assert_eq!(removed, 2);
        assert!(path.join("keep.md").exists());
        assert!(!path.join("stale.md").exists());
        assert!(!path.join("also-stale.md").exists());
        assert!(path.join("not-md.txt").exists());
    }

    #[test]
    fn remove_stale_in_nonexistent_dir_returns_zero() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");
        let keep: std::collections::HashSet<String> = std::collections::HashSet::new();
        assert_eq!(remove_stale_in(&missing, &keep), 0);
    }
}
