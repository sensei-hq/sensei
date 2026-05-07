//! Remove sensei artifacts — uninstall logic.

use std::fs;

use super::{cache_dir, home, plugin_dir, sensei_dir, RemoveRequest, RemoveResult};

// ── Remove ──────────────────────────────────────────────────────────────────

/// Remove sensei artifacts. With purge=true, also removes project data.
/// Data directory (~/.sensei/) deletion is handled by the CLI after stopping the daemon.
pub fn remove(req: &RemoveRequest) -> RemoveResult {
    let mut result = RemoveResult {
        acps_removed: crate::assistants::remove_selected(&[]),
        ..Default::default()
    };

    // 2. Remove plugin artifacts (commands, skills, agents, hooks)
    remove_plugin_artifacts(&mut result);

    // 3. Clear marketplace cache
    remove_cache(&mut result);

    // 4. If purge: remove project .sensei/ dirs
    if req.purge {
        remove_registered_projects(&mut result);
    }

    result
}

/// Remove plugin directory, commands, skills, agents, hooks config.
fn remove_plugin_artifacts(result: &mut RemoveResult) {
    let h = home();

    // Plugin directory (hooks + binaries)
    let plugin = plugin_dir();
    if plugin.exists() {
        fs::remove_dir_all(&plugin).ok();
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
fn remove_cache(result: &mut RemoveResult) {
    let cache = cache_dir();
    if cache.exists() {
        fs::remove_dir_all(&cache).ok();
        result.cache_cleared = true;
    }
}

/// Remove .sensei/ dirs from all registered projects.
fn remove_registered_projects(result: &mut RemoveResult) {
    let projects_file = sensei_dir().join("projects.json");
    let projects: Vec<String> = projects_file
        .exists()
        .then(|| fs::read_to_string(&projects_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["projects"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    for path in &projects {
        remove_project_scope(path, result);
    }
}

/// Remove sensei artifacts from a single project directory.
fn remove_project_scope(project_path: &str, result: &mut RemoveResult) {
    let root = std::path::PathBuf::from(project_path);
    if !root.exists() { return; }

    // .sensei/ directory
    let sensei = root.join(".sensei");
    if sensei.exists() {
        fs::remove_dir_all(&sensei).ok();
    }

    // .claude/commands/, .claude/skills/, .claude/agents/
    for subdir in &["commands", "skills", "agents"] {
        let dir = root.join(".claude").join(subdir);
        if dir.exists() {
            let count = remove_md_files_in(&dir);
            match *subdir {
                "skills" => result.skills_removed += count,
                "commands" => result.commands_removed += count,
                "agents" => result.agents_removed += count,
                _ => {}
            }
        }
    }

    // Remove sensei entry from .mcp.json (preserve other servers)
    let mcp_file = root.join(".mcp.json");
    if mcp_file.exists()
        && let Ok(content) = fs::read_to_string(&mcp_file)
            && let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content)
                && let Some(servers) = config
                    .get_mut("mcpServers")
                    .and_then(|s| s.as_object_mut())
                    && servers.remove("sensei").is_some() {
                        if servers.is_empty() {
                            fs::remove_file(&mcp_file).ok();
                        } else {
                            fs::write(&mcp_file, serde_json::to_string_pretty(&config).unwrap()).ok();
                        }
                    }

    result.projects_cleaned.push(project_path.to_string());
}

/// Remove all .md files in a directory. Removes the directory if empty afterward.
fn remove_md_files_in(dir: &std::path::Path) -> u32 {
    if !dir.exists() { return 0; }
    let mut count = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "md") {
                fs::remove_file(entry.path()).ok();
                count += 1;
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

    // ── remove_project_scope ────────────────────────────────────────

    #[test]
    fn remove_project_scope_nonexistent_path_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("no_such_project");
        let mut result = RemoveResult::default();
        remove_project_scope(missing.to_str().unwrap(), &mut result);
        // Should not be added to projects_cleaned because path doesn't exist
        assert!(result.projects_cleaned.is_empty());
    }

    #[test]
    fn remove_project_scope_removes_sensei_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("my_project");
        let sensei = project.join(".sensei");
        fs::create_dir_all(sensei.join("indexes")).unwrap();
        fs::write(sensei.join("config.json"), "{}").unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(project.to_str().unwrap(), &mut result);

        assert!(!sensei.exists());
        assert_eq!(result.projects_cleaned, vec![project.to_str().unwrap()]);
    }

    #[test]
    fn remove_project_scope_removes_claude_subdirs_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let claude = project.join(".claude");

        // Create skills with 2 md files
        let skills_dir = claude.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("a.md"), "skill a").unwrap();
        fs::write(skills_dir.join("b.md"), "skill b").unwrap();

        // Create commands with 1 md file + 1 non-md
        let cmds_dir = claude.join("commands");
        fs::create_dir_all(&cmds_dir).unwrap();
        fs::write(cmds_dir.join("c.md"), "command c").unwrap();
        fs::write(cmds_dir.join("keep.txt"), "keep").unwrap();

        // Create agents with 1 md file (agents count is not tracked)
        let agents_dir = claude.join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("d.md"), "agent d").unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(project.to_str().unwrap(), &mut result);

        assert_eq!(result.skills_removed, 2);
        assert_eq!(result.commands_removed, 1);
        // Non-md file is preserved, so commands dir still exists
        assert!(cmds_dir.join("keep.txt").exists());
        // Skills dir should be cleaned up (was only md files)
        assert!(!skills_dir.exists());
        // Agents dir should be cleaned up
        assert!(!agents_dir.exists());
    }

    #[test]
    fn remove_project_scope_removes_sensei_from_mcp_json() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let mcp_file = project.join(".mcp.json");
        let config = serde_json::json!({
            "mcpServers": {
                "sensei": {"command": "sensei-mcp", "args": []},
                "other": {"command": "other-mcp", "args": []}
            }
        });
        fs::write(&mcp_file, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(project.to_str().unwrap(), &mut result);

        // File should still exist with 'other' server preserved
        assert!(mcp_file.exists());
        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&mcp_file).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert_eq!(content["mcpServers"]["other"]["command"], "other-mcp");
    }

    #[test]
    fn remove_project_scope_deletes_mcp_json_when_sensei_is_only_server() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let mcp_file = project.join(".mcp.json");
        let config = serde_json::json!({
            "mcpServers": {
                "sensei": {"command": "sensei-mcp"}
            }
        });
        fs::write(&mcp_file, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(project.to_str().unwrap(), &mut result);

        // File should be deleted when sensei was the only server
        assert!(!mcp_file.exists());
    }

    #[test]
    fn remove_project_scope_preserves_mcp_json_without_sensei_key() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let mcp_file = project.join(".mcp.json");
        let original = serde_json::json!({
            "mcpServers": {
                "other": {"command": "other-mcp"}
            }
        });
        let original_str = serde_json::to_string_pretty(&original).unwrap();
        fs::write(&mcp_file, &original_str).unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(project.to_str().unwrap(), &mut result);

        // File should be untouched — no sensei key to remove
        assert!(mcp_file.exists());
        let content = fs::read_to_string(&mcp_file).unwrap();
        assert_eq!(content, original_str);
    }

    // ── remove (integration-level) ──────────────────────────────────

    #[test]
    fn remove_without_purge_does_not_clean_projects() {
        let req = RemoveRequest { purge: false };
        let result = remove(&req);
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

    // ── remove_project_scope edge cases ────────────────────────────

    #[test]
    fn remove_project_scope_handles_invalid_mcp_json() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        // Write invalid JSON
        let mcp_file = project.join(".mcp.json");
        fs::write(&mcp_file, "not valid json!!!").unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(project.to_str().unwrap(), &mut result);

        // Should not panic, file should still exist (couldn't parse it)
        assert!(mcp_file.exists());
        assert_eq!(result.projects_cleaned, vec![project.to_str().unwrap()]);
    }

    #[test]
    fn remove_project_scope_handles_mcp_json_without_mcp_servers() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let mcp_file = project.join(".mcp.json");
        fs::write(&mcp_file, r#"{"other_key": "value"}"#).unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(project.to_str().unwrap(), &mut result);

        // File untouched, no panic
        assert!(mcp_file.exists());
        let content = fs::read_to_string(&mcp_file).unwrap();
        assert!(content.contains("other_key"));
    }

    #[test]
    fn remove_project_scope_full_cleanup() {
        // Test a project with .sensei, all .claude subdirs, and .mcp.json
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");

        // .sensei/
        fs::create_dir_all(project.join(".sensei/indexes")).unwrap();
        fs::write(project.join(".sensei/config.json"), "{}").unwrap();

        // .claude/skills/, .claude/commands/, .claude/agents/
        fs::create_dir_all(project.join(".claude/skills")).unwrap();
        fs::write(project.join(".claude/skills/s1.md"), "s1").unwrap();
        fs::write(project.join(".claude/skills/s2.md"), "s2").unwrap();
        fs::create_dir_all(project.join(".claude/commands")).unwrap();
        fs::write(project.join(".claude/commands/c1.md"), "c1").unwrap();
        fs::create_dir_all(project.join(".claude/agents")).unwrap();
        fs::write(project.join(".claude/agents/a1.md"), "a1").unwrap();

        // .mcp.json with sensei as sole server
        fs::write(
            project.join(".mcp.json"),
            r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"}}}"#,
        )
        .unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(project.to_str().unwrap(), &mut result);

        assert!(!project.join(".sensei").exists());
        assert!(!project.join(".claude/skills").exists());
        assert!(!project.join(".claude/commands").exists());
        assert!(!project.join(".claude/agents").exists());
        assert!(!project.join(".mcp.json").exists());
        assert_eq!(result.skills_removed, 2);
        assert_eq!(result.commands_removed, 1);
        assert_eq!(result.agents_removed, 1);
        assert_eq!(result.projects_cleaned, vec![project.to_str().unwrap()]);
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

    // ── remove_project_scope accumulation ────────────────────────────

    #[test]
    fn remove_project_scope_accumulates_counts() {
        let tmp = tempfile::tempdir().unwrap();

        let p1 = tmp.path().join("p1");
        fs::create_dir_all(p1.join(".claude/skills")).unwrap();
        fs::write(p1.join(".claude/skills/a.md"), "a").unwrap();
        fs::write(p1.join(".claude/skills/b.md"), "b").unwrap();
        fs::create_dir_all(p1.join(".claude/commands")).unwrap();
        fs::write(p1.join(".claude/commands/c.md"), "c").unwrap();

        let p2 = tmp.path().join("p2");
        fs::create_dir_all(p2.join(".claude/skills")).unwrap();
        fs::write(p2.join(".claude/skills/d.md"), "d").unwrap();
        fs::create_dir_all(p2.join(".claude/commands")).unwrap();
        fs::write(p2.join(".claude/commands/e.md"), "e").unwrap();
        fs::write(p2.join(".claude/commands/f.md"), "f").unwrap();

        let mut result = RemoveResult::default();
        remove_project_scope(p1.to_str().unwrap(), &mut result);
        remove_project_scope(p2.to_str().unwrap(), &mut result);

        assert_eq!(result.skills_removed, 3);   // 2 + 1
        assert_eq!(result.commands_removed, 3);  // 1 + 2
        assert_eq!(result.projects_cleaned.len(), 2);
    }
}
