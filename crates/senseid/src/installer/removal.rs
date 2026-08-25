//! Remove sensei artifacts — uninstall logic.

use std::fs;
use std::path::Path;

use super::{RemoveRequest, RemoveResult, cache_dir, home, plugin_dir, sensei_dir};

// ── Remove ──────────────────────────────────────────────────────────────────

/// Remove sensei artifacts. With purge=true, also removes project data.
/// Data directory (~/.sensei/) deletion is handled by the CLI after stopping the daemon.
pub fn remove(req: &RemoveRequest) -> RemoveResult {
    // Resolve the real locations once and delegate. The path-injected core lets
    // tests exercise the logic against a temp dir without uninstalling the
    // developer's own plugin (the real `claude plugin uninstall` + ~/.claude
    // deletes would otherwise wipe the running install on every test run).
    remove_with(req, &home(), &plugin_dir(), &cache_dir(), &sensei_dir(), true)
}

/// Path-injected core. `run_uninstall` gates the external
/// `claude plugin uninstall` (skipped in tests so it never touches real config).
/// `_req.purge` and `_sensei_dir` are currently unused: pre-release there is no
/// project registry to purge (the daemon owns projects in Postgres), so uninstall
/// no longer walks a `projects.json` manifest cleaning each repo.
fn remove_with(
    _req: &RemoveRequest,
    home: &Path,
    plugin: &Path,
    cache: &Path,
    _sensei_dir: &Path,
    run_uninstall: bool,
) -> RemoveResult {
    let mut result = RemoveResult {
        // CLI uninstall path — no SSE consumer.
        acps_removed: if run_uninstall {
            crate::assistants::remove_selected(&[], None)
        } else {
            vec![]
        },
        ..Default::default()
    };

    // 2. Remove plugin artifacts (commands, skills, agents, hooks)
    remove_plugin_artifacts(&mut result, home, plugin);

    // 3. Clear marketplace cache
    remove_cache(&mut result, cache);

    result
}

/// Remove plugin directory, commands, skills, agents, hooks config.
fn remove_plugin_artifacts(result: &mut RemoveResult, h: &Path, plugin: &Path) {
    // Plugin directory (hooks + binaries)
    if plugin.exists() {
        if let Err(e) = fs::remove_dir_all(plugin) {
            tracing::warn!(error = %e, path = %plugin.display(), "failed to remove plugin directory during uninstall");
            result.errors.push(format!("plugin dir {}: {}", plugin.display(), e));
        }
        result.plugin_removed = true;
        result.hooks_removed = true;
    }

    // Global commands
    let commands_dir = h.join(".claude/commands");
    result.commands_removed += remove_md_files_in(&commands_dir);

    // Global skills
    let skills_dir = h.join(".claude/skills");
    result.skills_removed += remove_md_files_in(&skills_dir);

    // Global agents
    let agents_dir = h.join(".claude/agents");
    result.agents_removed += remove_md_files_in(&agents_dir);

    // Note: settings.json hooks are intentionally NOT touched here.
    // Hook registration is managed by `claude plugin install/uninstall sensei`.
    // Removing the entire "hooks" block would destroy hooks not owned by sensei.
}

/// Clear marketplace cache.
fn remove_cache(result: &mut RemoveResult, cache: &Path) {
    if cache.exists() {
        if let Err(e) = fs::remove_dir_all(cache) {
            tracing::warn!(error = %e, path = %cache.display(), "failed to clear marketplace cache during uninstall");
            result.errors.push(format!("cache {}: {}", cache.display(), e));
        }
        result.cache_cleared = true;
    }
}

/// Remove all .md files in a directory. Removes the directory if empty afterward.
fn remove_md_files_in(dir: &std::path::Path) -> u32 {
    if !dir.exists() {
        return 0;
    }
    let mut count = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "md") {
                if let Err(e) = fs::remove_file(entry.path()) {
                    tracing::warn!(error = %e, path = %entry.path().display(), "failed to remove .md file during uninstall");
                } else {
                    count += 1;
                }
            }
        }
    }
    if fs::read_dir(dir).map(|mut d| d.next().is_none()).unwrap_or(true) {
        fs::remove_dir(dir).ok();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── remove_md_files_in ─────────────────────────────────────────────

    #[test]
    fn remove_md_files_in_nonexistent_dir_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does_not_exist");
        assert_eq!(remove_md_files_in(&missing), 0);
    }

    #[test]
    fn remove_md_files_in_empty_dir_returns_zero_and_removes_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("empty");
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(remove_md_files_in(&dir), 0);
        // Empty dir should be removed
        assert!(!dir.exists());
    }

    #[test]
    fn remove_md_files_in_removes_only_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("one.md"), "# skill one").unwrap();
        fs::write(dir.join("two.md"), "# skill two").unwrap();
        fs::write(dir.join("keep.txt"), "not markdown").unwrap();

        let count = remove_md_files_in(&dir);
        assert_eq!(count, 2);
        assert!(!dir.join("one.md").exists());
        assert!(!dir.join("two.md").exists());
        // Non-md file is preserved
        assert!(dir.join("keep.txt").exists());
        // Dir is NOT removed because it still has files
        assert!(dir.exists());
    }

    #[test]
    fn remove_md_files_in_cleans_empty_dir_after_removing_all_md() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("commands");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("only.md"), "# only md file").unwrap();

        let count = remove_md_files_in(&dir);
        assert_eq!(count, 1);
        // Dir should be removed because it's now empty
        assert!(!dir.exists());
    }

    // ── remove (integration-level) ──────────────────────────────────

    #[test]
    fn remove_without_purge_does_not_clean_projects() {
        // Run the path-injected core against a temp home with the external
        // `claude plugin uninstall` skipped — calling the real remove() here used
        // to uninstall sensei from the developer's own ~/.claude on every test run.
        let tmp = tempfile::tempdir().unwrap();
        let h = tmp.path();
        let result = remove_with(
            &RemoveRequest { purge: false },
            h,
            &h.join(".claude/plugins/sensei"),
            &h.join(".sensei/cache/marketplace"),
            &h.join(".sensei"),
            false,
        );
        assert!(result.projects_cleaned.is_empty());
    }

    // ── RemoveRequest deserialization ───────────────────────────────

    #[test]
    fn remove_request_default_purge_is_false() {
        let req: RemoveRequest = serde_json::from_str("{}").unwrap();
        assert!(!req.purge);
    }

    #[test]
    fn remove_request_parses_purge_true() {
        let req: RemoveRequest = serde_json::from_str(r#"{"purge": true}"#).unwrap();
        assert!(req.purge);
    }

    // ── RemoveResult serialization ─────────────────────────────────

    #[test]
    fn remove_result_default_is_empty() {
        let result = RemoveResult::default();
        assert!(!result.hooks_removed);
        assert!(!result.plugin_removed);
        assert!(!result.cache_cleared);
        assert_eq!(result.skills_removed, 0);
        assert_eq!(result.commands_removed, 0);
        assert_eq!(result.agents_removed, 0);
        assert!(result.acps_removed.is_empty());
        assert!(result.projects_cleaned.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn remove_result_serializes_to_json() {
        let result = RemoveResult {
            acps_removed: vec!["claude-code".into()],
            hooks_removed: true,
            skills_removed: 5,
            commands_removed: 3,
            agents_removed: 8,
            plugin_removed: true,
            cache_cleared: true,
            projects_cleaned: vec!["/tmp/proj".into()],
            errors: vec!["some error".into()],
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["hooks_removed"], true);
        assert_eq!(json["skills_removed"], 5);
        assert_eq!(json["commands_removed"], 3);
        assert_eq!(json["agents_removed"], 8);
        assert_eq!(json["plugin_removed"], true);
        assert_eq!(json["cache_cleared"], true);
        assert_eq!(json["projects_cleaned"][0], "/tmp/proj");
        assert_eq!(json["errors"][0], "some error");
    }

    // ── remove_md_files_in edge cases ─────────────────────────────────

    #[test]
    fn remove_md_files_in_ignores_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("a.md"), "a").unwrap();
        fs::write(dir.join("subdir/b.md"), "b").unwrap();

        let count = remove_md_files_in(&dir);
        // Only top-level .md files are removed
        assert_eq!(count, 1);
        assert!(!dir.join("a.md").exists());
        // Subdirectory and its files are untouched
        assert!(dir.join("subdir/b.md").exists());
    }

    #[test]
    fn remove_md_files_in_handles_mixed_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("commands");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("file.md"), "md").unwrap();
        fs::write(dir.join("file.txt"), "txt").unwrap();
        fs::write(dir.join("file.json"), "json").unwrap();
        fs::write(dir.join("file.md.bak"), "bak").unwrap();
        fs::write(dir.join("no_ext"), "none").unwrap();

        let count = remove_md_files_in(&dir);
        assert_eq!(count, 1);
        assert!(!dir.join("file.md").exists());
        assert!(dir.join("file.txt").exists());
        assert!(dir.join("file.json").exists());
        assert!(dir.join("file.md.bak").exists());
        assert!(dir.join("no_ext").exists());
    }
}
