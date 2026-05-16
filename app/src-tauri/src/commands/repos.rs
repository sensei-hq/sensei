//! Repository scanning, analysis, and dependency detection.

use std::time::SystemTime;

#[derive(serde::Serialize, Clone)]
pub struct AnalyzedRepo {
    name: String,
    path: String,
    remote: Option<String>,
    description: Option<String>,
    categories: Vec<String>,
    status: String,
    last_commit_days: Option<u32>,
    tech_stack: Vec<String>,
    commit_count: u32,
    duplicate_of: Option<String>,
    variant_group: Option<String>,
}

#[tauri::command]
pub fn analyze_folder(root: String) -> Vec<AnalyzedRepo> {
    let root = shellexpand::tilde(&root).to_string();
    let root_path = std::path::Path::new(&root);
    let mut repos = vec![];
    scan_dir(root_path, 0, 4, &mut repos);
    find_duplicates(&mut repos);
    find_variants(&mut repos);
    repos
}

#[tauri::command]
pub fn get_repo_id(path: String) -> Option<String> {
    let config_path = std::path::PathBuf::from(&path).join(".sensei/config.yaml");
    let content = std::fs::read_to_string(config_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("repo_id:") {
            let val = trimmed.trim_start_matches("repo_id:").trim().trim_matches('"').trim_matches('\'');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

#[derive(serde::Serialize, Clone)]
pub struct DetectedDep {
    name: String,
    file_count: usize,
    total_files: usize,
    usage_pct: f32,
}

#[tauri::command]
pub fn detect_dependencies(path: String) -> Vec<DetectedDep> {
    let path = shellexpand::tilde(&path).to_string();
    let root = std::path::Path::new(&path);
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total: usize = 0;
    collect_imports(root, &mut counts, &mut total);
    if total == 0 { return vec![]; }
    let mut deps: Vec<DetectedDep> = counts
        .into_iter()
        .map(|(name, file_count)| DetectedDep {
            usage_pct: ((file_count as f32 / total as f32) * 1000.0).round() / 10.0,
            name,
            file_count,
            total_files: total,
        })
        .collect();
    deps.sort_by_key(|d| std::cmp::Reverse(d.file_count));
    deps.truncate(60);
    deps
}

// ── Internal helpers ────────────────────────────────────────────────────────

fn scan_dir(dir: &std::path::Path, depth: u32, max_depth: u32, out: &mut Vec<AnalyzedRepo>) {
    if depth > max_depth { return; }

    if dir.join(".git").exists() {
        let name = dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let remote = read_git_remote(dir);
        let description = read_readme_summary(dir);
        let last_commit_days = detect_last_commit_days(dir);
        let commit_count = estimate_commit_count(dir);
        let tech_stack = detect_tech_stack(dir);
        let categories = detect_categories(dir, &tech_stack, last_commit_days, commit_count);
        let status = detect_status(last_commit_days, commit_count);

        out.push(AnalyzedRepo {
            name, path: dir.to_string_lossy().to_string(), remote, description,
            categories, status, last_commit_days, tech_stack, commit_count,
            duplicate_of: None, variant_group: None,
        });
        return;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut children: Vec<_> = entries.flatten()
            .filter(|e| { let p = e.path(); p.is_dir() && !is_ignored(&p) })
            .collect();
        children.sort_by_key(|e| e.file_name());
        for entry in children {
            scan_dir(&entry.path(), depth + 1, max_depth, out);
        }
    }
}

fn is_ignored(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.starts_with('.') || matches!(s, "node_modules" | "target" | "dist" | "build" | ".git"))
        .unwrap_or(false)
}

fn read_git_remote(repo: &std::path::Path) -> Option<String> {
    let config = std::fs::read_to_string(repo.join(".git/config")).ok()?;
    for line in config.lines() {
        let t = line.trim();
        if t.starts_with("url = ") {
            return Some(t.trim_start_matches("url = ").to_string());
        }
    }
    None
}

fn detect_last_commit_days(repo: &std::path::Path) -> Option<u32> {
    for candidate in &[".git/COMMIT_EDITMSG", ".git/index"] {
        let p = repo.join(candidate);
        if let Ok(meta) = std::fs::metadata(&p) {
            if let Ok(modified) = meta.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    return Some((elapsed.as_secs() / 86400) as u32);
                }
            }
        }
    }
    None
}

fn estimate_commit_count(repo: &std::path::Path) -> u32 {
    std::fs::read_to_string(repo.join(".git/logs/HEAD"))
        .map(|s| s.lines().count() as u32)
        .unwrap_or(0)
}

fn read_readme_summary(repo: &std::path::Path) -> Option<String> {
    for name in &["README.md", "readme.md", "Readme.md", "README.txt", "README"] {
        if let Ok(content) = std::fs::read_to_string(repo.join(name)) {
            for line in content.lines() {
                let clean = line.trim().trim_start_matches('#').trim();
                if clean.len() > 10 && !clean.starts_with('[') && !clean.starts_with('!') {
                    return Some(clean.chars().take(120).collect());
                }
            }
        }
    }
    None
}

fn detect_tech_stack(repo: &std::path::Path) -> Vec<String> {
    let mut stack = vec![];
    if repo.join("Cargo.toml").exists()        { stack.push("rust"); }
    if repo.join("go.mod").exists()            { stack.push("go"); }
    if repo.join("Package.swift").exists()     { stack.push("swift"); }
    if repo.join("pubspec.yaml").exists()      { stack.push("dart"); }
    if repo.join("Gemfile").exists()           { stack.push("ruby"); }
    if repo.join("pom.xml").exists()
        || repo.join("build.gradle").exists()  { stack.push("java"); }
    if repo.join("pyproject.toml").exists()
        || repo.join("requirements.txt").exists() { stack.push("python"); }
    if repo.join("package.json").exists() {
        if let Ok(src) = std::fs::read_to_string(repo.join("package.json")) {
            let frameworks = [
                ("\"@sveltejs/kit\"", "sveltekit"), ("\"svelte\"", "svelte"),
                ("\"next\"", "nextjs"), ("\"react\"", "react"),
                ("\"nuxt\"", "nuxtjs"), ("\"vue\"", "vue"),
                ("\"solid-js\"", "solid"), ("\"@tauri-apps/api\"", "tauri"),
                ("\"electron\"", "electron"), ("\"@angular/core\"", "angular"),
            ];
            let mut found = false;
            for (key, label) in &frameworks {
                if src.contains(key) { stack.push(label); found = true; break; }
            }
            if !found { stack.push("node"); }
        }
    }
    stack.into_iter().map(|s| s.to_string()).collect()
}

fn detect_categories(
    repo: &std::path::Path, tech_stack: &[String],
    last_commit_days: Option<u32>, commit_count: u32,
) -> Vec<String> {
    let mut cats: Vec<&str> = vec![];

    if let Ok(cargo) = std::fs::read_to_string(repo.join("Cargo.toml")) {
        if cargo.contains("[lib]")   { cats.push("library"); }
        if cargo.contains("[[bin]]") { cats.push("tool"); }
        if !cats.is_empty() { return cats.into_iter().map(|s| s.to_string()).collect(); }
    }

    if repo.join("go.mod").exists() {
        if repo.join("cmd").exists() { cats.push("tool"); }
        let has_lib_root = std::fs::read_dir(repo)
            .map(|d| d.flatten().any(|e| e.path().extension().map(|x| x == "go").unwrap_or(false)))
            .unwrap_or(false);
        if has_lib_root { cats.push("library"); }
        if !cats.is_empty() { return cats.into_iter().map(|s| s.to_string()).collect(); }
    }

    if let Ok(pkg) = std::fs::read_to_string(repo.join("package.json")) {
        let has_bin = pkg.contains("\"bin\"");
        let has_exports = pkg.contains("\"exports\"") || pkg.contains("\"main\"")
            || pkg.contains("\"module\"") || pkg.contains("\"svelte\"");
        let is_svelte_pkg = pkg.contains("svelte-package") || pkg.contains("@sveltejs/package");
        let has_app_routes = repo.join("src/routes").exists()
            || repo.join("pages").exists() || repo.join("app").exists();

        if has_exports || is_svelte_pkg { cats.push("library"); }
        if has_bin { cats.push("tool"); }
        if has_app_routes {
            let is_app_fw = tech_stack.iter().any(|s| {
                matches!(s.as_str(), "sveltekit"|"nextjs"|"nuxtjs"|"tauri"|"electron"|"angular")
            });
            if is_app_fw && !cats.contains(&"library") { cats.push("app"); }
        }
        if !cats.is_empty() { return cats.into_iter().map(|s| s.to_string()).collect(); }
    }

    let is_app_fw = tech_stack.iter().any(|s| {
        matches!(s.as_str(), "sveltekit"|"nextjs"|"nuxtjs"|"tauri"|"electron"|"angular")
    });
    if is_app_fw { return vec!["app".to_string()]; }
    if tech_stack.iter().any(|s| matches!(s.as_str(), "react"|"vue"|"svelte"|"solid")) {
        return vec!["app".to_string()];
    }

    let old = last_commit_days.map(|d| d > 120).unwrap_or(false);
    if old && commit_count < 15 { return vec!["idea".to_string()]; }

    vec!["unknown".to_string()]
}

fn detect_status(last_commit_days: Option<u32>, commit_count: u32) -> String {
    match last_commit_days {
        None               => "unknown".to_string(),
        Some(d) if d <= 30  => "active".to_string(),
        Some(d) if d <= 90  => "recent".to_string(),
        Some(d) if d <= 365 => "stale".to_string(),
        Some(d) if commit_count < 3 && d > 730 => "abandoned".to_string(),
        Some(_)             => "archived".to_string(),
    }
}

fn normalize_remote(url: &str) -> String {
    url.trim().trim_end_matches('/').trim_end_matches(".git")
        .replace("git@github.com:", "github.com/")
        .replace("https://github.com/", "github.com/")
        .replace("http://github.com/", "github.com/")
        .replace("git@gitlab.com:", "gitlab.com/")
        .replace("https://gitlab.com/", "gitlab.com/")
        .to_lowercase()
}

fn find_duplicates(repos: &mut [AnalyzedRepo]) {
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for repo in repos.iter() {
        if let Some(remote) = &repo.remote {
            let key = normalize_remote(remote);
            if !key.is_empty() { seen.entry(key).or_insert_with(|| repo.path.clone()); }
        }
    }
    for repo in repos.iter_mut() {
        if let Some(remote) = &repo.remote {
            let key = normalize_remote(remote);
            if let Some(original) = seen.get(&key) {
                if *original != repo.path { repo.duplicate_of = Some(original.clone()); }
            }
        }
    }
}

fn name_stem(name: &str) -> String {
    let n = name.to_lowercase();
    const SUFFIXES: &[&str] = &[
        "-backup", "-experiment", "-prototype",
        "-v1", "-v2", "-v3", "-v4", "-v5",
        "-old", "-new", "-bak", "-copy", "-alt", "-next",
        "-poc", "-demo", "-test", "-idea", "-draft",
        "_backup", "_old", "_bak", "_copy", "_new",
    ];
    for s in SUFFIXES {
        if let Some(stem) = n.strip_suffix(s) {
            if stem.len() >= 3 { return stem.to_string(); }
        }
    }
    let t = n.trim_end_matches(|c: char| c.is_ascii_digit());
    let t = t.trim_end_matches(['-', '_']);
    if t.len() >= 3 && t.len() < n.len() { return t.to_string(); }
    n
}

fn find_variants(repos: &mut [AnalyzedRepo]) {
    use std::collections::HashMap;
    let names_lower: Vec<String> = repos.iter().map(|r| r.name.to_lowercase()).collect();
    let stem_for = |name: &str| -> String {
        let suffix_stem = name_stem(name);
        let nl = name.to_lowercase();
        let prefix_stem = names_lower.iter()
            .filter(|other| {
                other.len() >= 4 && other.len() < nl.len()
                    && (nl.starts_with(&format!("{}-", other)) || nl.starts_with(&format!("{}_", other)))
            })
            .max_by_key(|s| s.len())
            .cloned();
        prefix_stem.unwrap_or(suffix_stem)
    };
    let mut stem_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, r) in repos.iter().enumerate() {
        if r.duplicate_of.is_some() { continue; }
        stem_map.entry(stem_for(&r.name)).or_default().push(i);
    }
    for (stem, group) in stem_map {
        if group.len() >= 2 {
            for i in group { repos[i].variant_group = Some(stem.clone()); }
        }
    }
}

fn collect_imports(
    dir: &std::path::Path,
    counts: &mut std::collections::HashMap<String, usize>,
    total: &mut usize,
) {
    let Ok(entries) = std::fs::read_dir(dir) else { return; };
    let mut children: Vec<_> = entries.flatten().collect();
    children.sort_by_key(|e| e.file_name());
    for entry in children {
        let path = entry.path();
        if path.is_dir() {
            if !is_ignored(&path) { collect_imports(&path, counts, total); }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "svelte" | "vue") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    *total += 1;
                    let pkgs = extract_file_imports(&content);
                    let mut seen = std::collections::HashSet::new();
                    for pkg in pkgs {
                        if seen.insert(pkg.clone()) { *counts.entry(pkg).or_insert(0) += 1; }
                    }
                }
            }
        }
    }
}

fn extract_file_imports(content: &str) -> Vec<String> {
    let mut pkgs = vec![];
    for line in content.lines() {
        let l = line.trim();
        if l.starts_with("//") || l.starts_with('*') || l.starts_with("/*") { continue; }
        if let Some(pos) = l.rfind(" from ") {
            if let Some(pkg) = parse_pkg_specifier(l[pos + 6..].trim()) { pkgs.push(pkg); }
        }
        if let Some(pos) = l.find("require(") {
            if let Some(pkg) = parse_pkg_specifier(l[pos + 8..].trim()) { pkgs.push(pkg); }
        }
    }
    pkgs
}

fn parse_pkg_specifier(s: &str) -> Option<String> {
    let s = s.trim();
    let quote = s.chars().next()?;
    if !matches!(quote, '\'' | '"' | '`') { return None; }
    let inner = &s[1..];
    if inner.starts_with('.') || inner.starts_with('/') { return None; }
    if let Some(inner) = inner.strip_prefix('@') {
        let slash = inner.find('/')?;
        let org = &inner[..slash];
        let rest = &inner[slash + 1..];
        let end = rest.find(['\'', '"', '`', '/'])?;
        let name = &rest[..end];
        if !org.is_empty() && !name.is_empty() { return Some(format!("@{org}/{name}")); }
    } else {
        let end = inner.find(['\'', '"', '`', '/'])?;
        let name = &inner[..end];
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.')) {
            return Some(name.to_string());
        }
    }
    None
}
