//! Solution grouping — suggest which repos belong together.

use std::path::Path;
use serde::Serialize;

/// Suggested solution grouping for discovered repos.
#[derive(Debug, Clone, Serialize)]
pub struct SolutionMatch {
    /// Suggested solution name.
    pub name: String,
    /// How the match was found: "parent-folder", "name-prefix", "subtree".
    pub strategy: String,
    /// Repo IDs that belong to this group.
    pub repo_ids: Vec<String>,
}

/// Given a list of (repo_id, repo_path) pairs, suggest solution groupings.
///
/// Strategies (applied in order, repos can only belong to one group):
/// 1. **Parent folder** — repos sharing an immediate parent directory
///    (e.g., `~/projects/acme/api` and `~/projects/acme/frontend` -> "acme")
/// 2. **Name prefix** — repos with a common name stem
///    (e.g., `acme-api`, `acme-ui`, `acme-shared` -> "acme")
///
/// Subtree detection is handled separately in process_git_folder (already exists).
pub fn suggest_solutions(repos: &[(String, String)]) -> Vec<SolutionMatch> {
    let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut matches = Vec::new();

    // Strategy 1: Parent folder grouping
    let parent_groups = group_by_parent_folder(repos);
    for (parent_name, repo_ids) in &parent_groups {
        if repo_ids.len() >= 2 {
            for id in repo_ids {
                assigned.insert(id.clone());
            }
            matches.push(SolutionMatch {
                name: parent_name.clone(),
                strategy: "parent-folder".into(),
                repo_ids: repo_ids.clone(),
            });
        }
    }

    // Strategy 2: Name prefix (only for repos not already grouped)
    let remaining: Vec<(String, String)> = repos.iter()
        .filter(|(id, _)| !assigned.contains(id))
        .cloned()
        .collect();

    let prefix_groups = group_by_name_prefix(&remaining);
    for (prefix, repo_ids) in &prefix_groups {
        if repo_ids.len() >= 2 {
            matches.push(SolutionMatch {
                name: prefix.clone(),
                strategy: "name-prefix".into(),
                repo_ids: repo_ids.clone(),
            });
        }
    }

    matches
}

/// Group repos by their immediate parent directory name.
fn group_by_parent_folder(repos: &[(String, String)]) -> Vec<(String, Vec<String>)> {
    let mut groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for (repo_id, repo_path) in repos {
        if let Some(parent) = Path::new(repo_path).parent() {
            let parent_name = parent.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            // Skip generic parent names
            if !parent_name.is_empty()
                && !matches!(parent_name.as_str(),
                    "Developer" | "dev" | "projects" | "repos" | "src" | "code"
                    | "workspace" | "workspaces" | "home" | "Users" | "Home"
                )
            {
                groups.entry(parent_name).or_default().push(repo_id.clone());
            }
        }
    }

    groups.into_iter().collect()
}

/// Group repos by common name prefix (split on `-`, `_`, `.`).
fn group_by_name_prefix(repos: &[(String, String)]) -> Vec<(String, Vec<String>)> {
    let mut groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for (repo_id, _) in repos {
        let prefix = extract_name_prefix(repo_id);
        if !prefix.is_empty() && prefix != *repo_id {
            groups.entry(prefix).or_default().push(repo_id.clone());
        }
    }

    groups.into_iter().collect()
}

/// Extract the common prefix from a repo name.
/// "acme-api" -> "acme", "my-project-frontend" -> "my-project"
pub(crate) fn extract_name_prefix(name: &str) -> String {
    // Split on common separators
    let parts: Vec<&str> = name.split(|c| c == '-' || c == '_' || c == '.').collect();
    if parts.len() >= 2 {
        parts[0].to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_solutions_by_parent_folder() {
        let repos = vec![
            ("api".into(), "/home/dev/acme/api".into()),
            ("frontend".into(), "/home/dev/acme/frontend".into()),
            ("shared".into(), "/home/dev/acme/shared".into()),
            ("blog".into(), "/home/dev/other/blog".into()),
        ];
        let matches = suggest_solutions(&repos);
        let acme = matches.iter().find(|m| m.name == "acme");
        assert!(acme.is_some());
        assert_eq!(acme.unwrap().repo_ids.len(), 3);
        assert_eq!(acme.unwrap().strategy, "parent-folder");
    }

    #[test]
    fn suggest_solutions_skips_generic_parents() {
        let repos = vec![
            ("api".into(), "/home/dev/Developer/api".into()),
            ("ui".into(), "/home/dev/Developer/ui".into()),
        ];
        let matches = suggest_solutions(&repos);
        // "Developer" is a generic name, should not create a solution
        assert!(matches.iter().all(|m| m.strategy != "parent-folder" || m.name != "Developer"));
    }

    #[test]
    fn suggest_solutions_by_name_prefix() {
        let repos = vec![
            ("acme-api".into(), "/a/acme-api".into()),
            ("acme-ui".into(), "/b/acme-ui".into()),
            ("blog".into(), "/c/blog".into()),
        ];
        let matches = suggest_solutions(&repos);
        let acme = matches.iter().find(|m| m.name == "acme");
        assert!(acme.is_some());
        assert_eq!(acme.unwrap().strategy, "name-prefix");
    }

    #[test]
    fn extract_name_prefix_splits_on_dash() {
        assert_eq!(extract_name_prefix("acme-api"), "acme");
        assert_eq!(extract_name_prefix("my-project-frontend"), "my");
    }

    #[test]
    fn extract_name_prefix_returns_empty_for_simple_name() {
        assert_eq!(extract_name_prefix("blog"), "");
    }
}
