use std::collections::{HashMap, HashSet};
use crate::db::Store;
use crate::types::{Project, Solution, SolutionRepo};
use super::graph::GraphDb;

/// Cross-repo relationship detected within a solution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrossRepoLink {
    pub from_repo: String,
    pub to_repo: String,
    pub link_type: String,
    pub details: Vec<String>,
    pub strength: f64,
}

/// Inferred role for a repo within a solution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InferredRole {
    pub repo_id: String,
    pub role: String,
    pub confidence: f64,
    pub reasons: Vec<String>,
}

/// Result of cross-repo analysis for a solution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SolutionAnalysis {
    pub solution_id: String,
    pub links: Vec<CrossRepoLink>,
    pub inferred_roles: Vec<InferredRole>,
    pub shared_libs: Vec<SharedLib>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SharedLib {
    pub name: String,
    pub repos: Vec<String>,
}

/// Analyze cross-repo relationships within a solution.
pub fn analyze_solution(
    store: &Store,
    graph_db: &GraphDb,
    solution: &Solution,
) -> Result<SolutionAnalysis, String> {
    // Load all projects in the solution
    let mut projects: HashMap<String, Project> = HashMap::new();
    for sr in &solution.repos {
        if let Ok(Some(p)) = store.get_project(&sr.repo_id) {
            projects.insert(sr.repo_id.clone(), p);
        }
    }

    if projects.len() < 2 {
        return Ok(SolutionAnalysis {
            solution_id: solution.id.clone(),
            links: vec![],
            inferred_roles: vec![],
            shared_libs: vec![],
        });
    }

    // 1. Detect shared libraries
    let shared_libs = detect_shared_libs(&projects);

    // 2. Detect cross-repo type/function references
    let mut links = detect_cross_imports(graph_db, &projects);

    // 3. Detect doc→code coverage across repos
    links.extend(detect_cross_doc_coverage(graph_db, &projects));

    // 4. Detect shared lib coupling (repos using same libs = likely related)
    links.extend(detect_lib_coupling(&shared_libs));

    // 5. Infer roles based on stack, libs, and file patterns
    let inferred_roles = infer_roles(&projects);

    // 6. Store cross-repo edges in graph
    store_cross_repo_edges(graph_db, &solution.id, &links)?;

    Ok(SolutionAnalysis {
        solution_id: solution.id.clone(),
        links,
        inferred_roles,
        shared_libs,
    })
}

fn detect_shared_libs(projects: &HashMap<String, Project>) -> Vec<SharedLib> {
    // lib_name → [repo_ids that use it]
    let mut lib_repos: HashMap<String, Vec<String>> = HashMap::new();
    for (repo_id, project) in projects {
        for lib in &project.libs {
            lib_repos.entry(lib.clone()).or_default().push(repo_id.clone());
        }
    }

    lib_repos.into_iter()
        .filter(|(_, repos)| repos.len() > 1)
        .map(|(name, repos)| SharedLib { name, repos })
        .collect()
}

fn detect_cross_imports(
    graph_db: &GraphDb,
    projects: &HashMap<String, Project>,
) -> Vec<CrossRepoLink> {
    let mut links = Vec::new();

    // For each repo, get its exported function/type names
    let mut exports: HashMap<String, HashSet<String>> = HashMap::new(); // repo_id → set of symbol names
    let mut export_files: HashMap<String, HashSet<String>> = HashMap::new(); // repo_id → set of module names

    for repo_id in projects.keys() {
        let (fns, _) = graph_db.count_symbols(repo_id).unwrap_or((0, 0));
        if fns == 0 { continue; }

        // Get all function names for this repo
        if let Ok(functions) = graph_db.search_functions("", repo_id) {
            let names: HashSet<String> = functions.iter().map(|f| f.name.clone()).collect();
            exports.insert(repo_id.clone(), names);
        }

        // Get all file modules
        if let Ok(nodes) = graph_db.get_nodes(repo_id) {
            let modules: HashSet<String> = nodes.iter()
                .filter(|n| n.kind == "file")
                .map(|n| n.name.clone())
                .collect();
            export_files.insert(repo_id.clone(), modules);
        }
    }

    // Cross-match: for each pair of repos, find overlapping symbol names
    let repo_ids: Vec<&String> = projects.keys().collect();
    for i in 0..repo_ids.len() {
        for j in (i+1)..repo_ids.len() {
            let a = repo_ids[i];
            let b = repo_ids[j];

            if let (Some(exports_a), Some(exports_b)) = (exports.get(a), exports.get(b)) {
                // Functions in A that are also called in B (by name match)
                let a_calls_b: Vec<String> = exports_a.intersection(exports_b)
                    .cloned().collect();

                if !a_calls_b.is_empty() && a_calls_b.len() > 2 {
                    let sample: Vec<String> = a_calls_b.iter().take(5).cloned().collect();
                    links.push(CrossRepoLink {
                        from_repo: a.clone(),
                        to_repo: b.clone(),
                        link_type: "SHARED_SYMBOLS".into(),
                        details: sample,
                        strength: (a_calls_b.len() as f64).min(100.0) / 100.0,
                    });
                }
            }
        }
    }

    links
}

fn detect_cross_doc_coverage(
    graph_db: &GraphDb,
    projects: &HashMap<String, Project>,
) -> Vec<CrossRepoLink> {
    let mut links = Vec::new();

    // Find repos with docs
    let mut doc_repos: Vec<String> = Vec::new();
    let mut code_repos: Vec<String> = Vec::new();

    for (repo_id, project) in projects {
        let stack = project.stack.join(",").to_lowercase();
        let has_docs = stack.contains("markdown") || project.path.to_lowercase().contains("doc");

        // Check if repo has doc nodes
        if let Ok(nodes) = graph_db.get_nodes(repo_id) {
            let doc_count = nodes.iter().filter(|n| n.kind == "file" && n.file.ends_with(".md")).count();
            if doc_count > 5 || has_docs {
                doc_repos.push(repo_id.clone());
            }
            let code_count = nodes.iter().filter(|n| n.kind == "function").count();
            if code_count > 0 {
                code_repos.push(repo_id.clone());
            }
        }
    }

    // For each doc repo, check if its docs mention functions from code repos
    for doc_repo in &doc_repos {
        for code_repo in &code_repos {
            if doc_repo == code_repo { continue; }

            // Get function names from code repo
            let code_fns: HashSet<String> = graph_db.search_functions("", code_repo)
                .unwrap_or_default()
                .iter()
                .map(|f| f.name.clone())
                .collect();

            // Get doc content mentions from doc repo
            let doc_fns: HashSet<String> = graph_db.search_functions("", doc_repo)
                .unwrap_or_default()
                .iter()
                .map(|f| f.name.clone())
                .collect();

            // Functions mentioned in both (cross-repo references)
            let overlap: Vec<String> = code_fns.intersection(&doc_fns).cloned().collect();

            if !overlap.is_empty() {
                links.push(CrossRepoLink {
                    from_repo: doc_repo.clone(),
                    to_repo: code_repo.clone(),
                    link_type: "DOCUMENTS".into(),
                    details: overlap.iter().take(5).cloned().collect(),
                    strength: (overlap.len() as f64).min(50.0) / 50.0,
                });
            }
        }
    }

    links
}

fn detect_lib_coupling(shared_libs: &[SharedLib]) -> Vec<CrossRepoLink> {
    let mut links: HashMap<(String, String), Vec<String>> = HashMap::new();

    for lib in shared_libs {
        for i in 0..lib.repos.len() {
            for j in (i+1)..lib.repos.len() {
                let key = if lib.repos[i] < lib.repos[j] {
                    (lib.repos[i].clone(), lib.repos[j].clone())
                } else {
                    (lib.repos[j].clone(), lib.repos[i].clone())
                };
                links.entry(key).or_default().push(lib.name.clone());
            }
        }
    }

    links.into_iter()
        .filter(|(_, libs)| libs.len() >= 2) // at least 2 shared libs to be significant
        .map(|((a, b), libs)| CrossRepoLink {
            from_repo: a,
            to_repo: b,
            link_type: "SHARED_DEPS".into(),
            strength: (libs.len() as f64).min(20.0) / 20.0,
            details: libs,
        })
        .collect()
}

/// Public wrapper for role inference (used by API).
pub fn infer_roles_pub(projects: &HashMap<String, Project>) -> Vec<InferredRole> {
    infer_roles(projects)
}

fn infer_roles(projects: &HashMap<String, Project>) -> Vec<InferredRole> {
    let mut roles = Vec::new();

    for (repo_id, project) in projects {
        let stack = project.stack.join(",").to_lowercase();
        let libs = project.libs.join(",").to_lowercase();
        let name = project.name.to_lowercase();
        let path = project.path.to_lowercase();

        let mut role = "unknown".to_string();
        let mut confidence: f64 = 0.0;
        let mut reasons = Vec::new();

        // API / Backend detection
        if libs.contains("express") || libs.contains("fastify") || libs.contains("hono")
            || libs.contains("axum") || libs.contains("actix") || libs.contains("flask")
            || libs.contains("fastapi") || libs.contains("django") || libs.contains("spring")
            || libs.contains("ktor") || libs.contains("nestjs")
        {
            role = "api".into();
            confidence = 0.9;
            reasons.push("HTTP framework detected".into());
        }
        if name.contains("api") || name.contains("backend") || name.contains("server") {
            role = "api".into();
            confidence = confidence.max(0.8_f64);
            reasons.push(format!("Name suggests API: {}", project.name));
        }

        // UI / Frontend detection
        if libs.contains("react") || libs.contains("vue") || libs.contains("svelte")
            || libs.contains("angular") || libs.contains("nextjs") || libs.contains("nuxt")
        {
            role = "ui".into();
            confidence = 0.9;
            reasons.push("UI framework detected".into());
        }
        if (name.contains("ui") || name.contains("frontend") || name.contains("web")
            || name.contains("app") || name.contains("client") || name.contains("site"))
            && role == "unknown" {
                role = "ui".into();
                confidence = confidence.max(0.7_f64);
                reasons.push(format!("Name suggests UI: {}", project.name));
            }

        // Shared library detection
        if (name.contains("lib") || name.contains("shared") || name.contains("common")
            || name.contains("core") || name.contains("utils") || name.contains("sdk")
            || name.contains("config") || name.contains("packages"))
            && role == "unknown" {
                role = "shared-lib".into();
                confidence = 0.7;
                reasons.push(format!("Name suggests shared library: {}", project.name));
            }

        // Documentation repo detection
        if (name.contains("doc") || name.contains("wiki") || name.contains("spec")
            || path.contains("/docs") || stack.contains("markdown"))
            && role == "unknown" {
                role = "docs".into();
                confidence = 0.7;
                reasons.push("Documentation patterns detected".into());
            }

        // Mobile
        if libs.contains("uikit") || libs.contains("swiftui") || libs.contains("android")
            || libs.contains("react-native") || libs.contains("flutter")
        {
            role = "mobile".into();
            confidence = 0.9;
            reasons.push("Mobile framework detected".into());
        }

        // Infrastructure
        if (name.contains("infra") || name.contains("deploy") || name.contains("terraform")
            || name.contains("k8s") || name.contains("docker"))
            && role == "unknown" {
                role = "infra".into();
                confidence = 0.7;
                reasons.push("Infrastructure patterns detected".into());
            }

        if role == "unknown" && !project.stack.is_empty() {
            role = "service".into();
            confidence = 0.3;
            reasons.push(format!("Fallback: stack is {:?}", project.stack));
        }

        roles.push(InferredRole {
            repo_id: repo_id.clone(),
            role,
            confidence,
            reasons,
        });
    }

    roles
}

/// Auto-create a Solution for a monorepo.
/// Only registers actual git repos (parent + subtrees) — NOT workspace packages.
/// Workspace packages are already tracked in the graph's `packages` table.
/// Returns Some(solution_id) if a solution was created/updated, None otherwise.
pub fn auto_solution_for_monorepo(
    store: &Store,
    project: &Project,
) -> Result<Option<String>, String> {
    let repo_path = std::path::Path::new(&project.path);

    // Detect git subtrees by looking for directories with their own git history
    // (merged via `git subtree add`). These show up as dirs that were squash-merged.
    let subtrees = detect_git_subtrees(repo_path);

    // Check if solution already exists for this project
    let solutions = store.list_solutions().map_err(|e| e.to_string())?;
    for sol in &solutions {
        if sol.repos.iter().any(|r| r.repo_id == project.repo_id) {
            // Already in a solution — add any missing subtree repos
            for (name, path) in &subtrees {
                let sub_id = format!("{}:{}", project.repo_id, name);
                if !sol.repos.iter().any(|r| r.repo_id == sub_id) {
                    store.add_repo_to_solution(&sol.id, &SolutionRepo {
                        repo_id: sub_id,
                        role: "subtree".to_string(),
                        label: Some(name.clone()),
                        path: Some(path.clone()),
                    }).ok();
                }
            }
            return Ok(Some(sol.id.clone()));
        }
    }

    // Nothing to create a solution for if no subtrees found
    // (a monorepo with only workspaces doesn't need a solution — workspaces are packages)
    if subtrees.is_empty() {
        return Ok(None);
    }

    // Create new solution
    let solution_id = format!("auto:{}", project.repo_id);
    let solution = Solution {
        id: solution_id.clone(),
        name: project.name.clone(),
        description: Some(format!(
            "Monorepo with {} subtree repo(s)",
            subtrees.len()
        )),
        client: None,
        category: "active".to_string(),
        repos: Vec::new(),
        tags: vec!["monorepo".to_string(), "auto-detected".to_string()],
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    store.create_solution(&solution).map_err(|e| e.to_string())?;

    // Add parent repo
    store.add_repo_to_solution(&solution_id, &SolutionRepo {
        repo_id: project.repo_id.clone(),
        role: "parent".to_string(),
        label: Some(project.name.clone()),
        path: Some(project.path.clone()),
    }).ok();

    // Add subtree repos only (NOT workspace packages)
    for (name, path) in &subtrees {
        let sub_id = format!("{}:{}", project.repo_id, name);
        store.add_repo_to_solution(&solution_id, &SolutionRepo {
            repo_id: sub_id,
            role: "subtree".to_string(),
            label: Some(name.clone()),
            path: Some(path.clone()),
        }).ok();
    }

    Ok(Some(solution_id))
}

/// Detect git subtrees by finding directories that were merged via `git subtree add`.
/// Looks for squash merge commits in git log that mention subtree paths.
/// Public wrapper for task handlers.
pub fn detect_git_subtrees_pub(repo_path: &std::path::Path) -> Vec<(String, String)> {
    detect_git_subtrees(repo_path)
}

fn detect_git_subtrees(repo_path: &std::path::Path) -> Vec<(String, String)> {
    let mut subtrees = Vec::new();

    // Method 1: Check git log for subtree merge patterns
    let output = std::process::Command::new("git")
        .args(["log", "--oneline", "--all", "--grep=git-subtree-dir:"])
        .current_dir(repo_path)
        .output();

    if let Ok(out) = output {
        let log = String::from_utf8_lossy(&out.stdout);
        for line in log.lines() {
            // Extract dir from "Squashed 'dirname/' content from commit"
            if let Some(start) = line.find("Squashed '") {
                let rest = &line[start + 10..];
                if let Some(end) = rest.find("/'") {
                    let dir = &rest[..end];
                    let full_path = repo_path.join(dir);
                    if full_path.is_dir() {
                        subtrees.push((dir.to_string(), full_path.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }

    // Method 2: Fallback — check .gitsubtrees or known patterns
    // Also look for git-subtree-dir in recent merge commits
    if subtrees.is_empty() {
        let output = std::process::Command::new("git")
            .args(["log", "--oneline", "--merges", "-20"])
            .current_dir(repo_path)
            .output();

        if let Ok(out) = output {
            let log = String::from_utf8_lossy(&out.stdout);
            for line in log.lines() {
                // Pattern: "Merge commit 'xxx' as 'dirname'"
                if let Some(start) = line.find("as '") {
                    let rest = &line[start + 4..];
                    if let Some(end) = rest.find('\'') {
                        let dir = &rest[..end];
                        let full_path = repo_path.join(dir);
                        if full_path.is_dir() && !subtrees.iter().any(|(n, _)| n == dir) {
                            subtrees.push((dir.to_string(), full_path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
    }

    subtrees
}

fn store_cross_repo_edges(
    graph_db: &GraphDb,
    solution_id: &str,
    links: &[CrossRepoLink],
) -> Result<(), String> {
    for link in links {
        // Create solution-level edges: solution:X:repoA → solution:X:repoB
        let from = format!("solution:{}:{}", solution_id, link.from_repo);
        let to = format!("solution:{}:{}", solution_id, link.to_repo);
        graph_db.merge_edge(&from, &to, &link.link_type).ok();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_role_api() {
        let mut projects = HashMap::new();
        projects.insert("api".into(), Project {
            repo_id: "api".into(), name: "my-api".into(), path: "/api".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec!["typescript".into()], libs: vec!["express".into(), "prisma".into()],
            tags: vec![], status: "active".into(),
        });
        let roles = infer_roles(&projects);
        assert_eq!(roles[0].role, "api");
        assert!(roles[0].confidence > 0.8);
    }

    #[test]
    fn infer_role_ui() {
        let mut projects = HashMap::new();
        projects.insert("ui".into(), Project {
            repo_id: "ui".into(), name: "dashboard".into(), path: "/ui".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec!["typescript".into()], libs: vec!["svelte".into(), "d3".into()],
            tags: vec![], status: "active".into(),
        });
        let roles = infer_roles(&projects);
        assert_eq!(roles[0].role, "ui");
    }

    #[test]
    fn shared_libs_detection() {
        let mut projects = HashMap::new();
        projects.insert("a".into(), Project {
            repo_id: "a".into(), name: "svc-a".into(), path: "/a".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec![], libs: vec!["axios".into(), "lodash".into(), "zod".into()],
            tags: vec![], status: "active".into(),
        });
        projects.insert("b".into(), Project {
            repo_id: "b".into(), name: "svc-b".into(), path: "/b".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec![], libs: vec!["axios".into(), "zod".into(), "express".into()],
            tags: vec![], status: "active".into(),
        });

        let shared = detect_shared_libs(&projects);
        let shared_names: HashSet<&str> = shared.iter().map(|s| s.name.as_str()).collect();
        assert!(shared_names.contains("axios"));
        assert!(shared_names.contains("zod"));
        assert!(!shared_names.contains("express"));
    }

    #[test]
    fn lib_coupling_creates_links() {
        let shared = vec![
            SharedLib { name: "axios".into(), repos: vec!["a".into(), "b".into()] },
            SharedLib { name: "zod".into(), repos: vec!["a".into(), "b".into()] },
            SharedLib { name: "lodash".into(), repos: vec!["a".into(), "b".into()] },
        ];
        let links = detect_lib_coupling(&shared);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type, "SHARED_DEPS");
        assert!(links[0].details.contains(&"axios".to_string()));
    }

    #[test]
    fn infer_roles_pub_wrapper() {
        let mut projects = HashMap::new();
        projects.insert("lib".into(), Project {
            repo_id: "lib".into(), name: "shared-utils".into(), path: "/lib".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec!["typescript".into()], libs: vec![],
            tags: vec![], status: "active".into(),
        });
        let roles = infer_roles_pub(&projects);
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].role, "shared-lib");
    }

    #[test]
    fn infer_role_mobile() {
        let mut projects = HashMap::new();
        projects.insert("app".into(), Project {
            repo_id: "app".into(), name: "myapp".into(), path: "/app".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec!["swift".into()], libs: vec!["swiftui".into()],
            tags: vec![], status: "active".into(),
        });
        let roles = infer_roles(&projects);
        assert_eq!(roles[0].role, "mobile");
    }

    #[test]
    fn infer_role_infra() {
        let mut projects = HashMap::new();
        projects.insert("infra".into(), Project {
            repo_id: "infra".into(), name: "deploy-infra".into(), path: "/infra".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec![], libs: vec![],
            tags: vec![], status: "active".into(),
        });
        let roles = infer_roles(&projects);
        assert_eq!(roles[0].role, "infra");
    }

    #[test]
    fn infer_role_docs() {
        let mut projects = HashMap::new();
        projects.insert("docs".into(), Project {
            repo_id: "docs".into(), name: "project-docs".into(), path: "/docs".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec![], libs: vec![],
            tags: vec![], status: "active".into(),
        });
        let roles = infer_roles(&projects);
        assert_eq!(roles[0].role, "docs");
    }

    #[test]
    fn infer_role_fallback_service() {
        let mut projects = HashMap::new();
        projects.insert("x".into(), Project {
            repo_id: "x".into(), name: "unknown-thing".into(), path: "/x".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec!["rust".into()], libs: vec![],
            tags: vec![], status: "active".into(),
        });
        let roles = infer_roles(&projects);
        assert_eq!(roles[0].role, "service");
        assert!(roles[0].confidence < 0.5);
    }
}
