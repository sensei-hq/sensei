use std::path::PathBuf;
use std::time::SystemTime;
use serde_json::{json, Value};
use tauri::Manager;

fn home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

// ── ACP detection ─────────────────────────────────────────────────────────

#[tauri::command]
fn detect_acps() -> Vec<String> {
    let Some(h) = home() else { return vec![]; };
    let mut found = vec![];

    // Claude Desktop — look for the app bundle, not just the config dir
    if std::path::Path::new("/Applications/Claude.app").exists()
        || h.join("Applications/Claude.app").exists()
    {
        found.push("claude-desktop".to_string());
    }

    // Claude Code — ~/.claude/settings.json is the reliable marker
    if h.join(".claude/settings.json").exists() || h.join(".claude/CLAUDE.md").exists() {
        found.push("claude-code".to_string());
    }

    // Cursor — check the app bundle, not just ~/.cursor (that dir can exist spuriously)
    if std::path::Path::new("/Applications/Cursor.app").exists()
        || h.join("Applications/Cursor.app").exists()
        || which_exists("cursor")
    {
        found.push("cursor".to_string());
    }

    // Windsurf
    if std::path::Path::new("/Applications/Windsurf.app").exists()
        || h.join("Applications/Windsurf.app").exists()
        || which_exists("windsurf")
    {
        found.push("windsurf".to_string());
    }

    // Zed
    if std::path::Path::new("/Applications/Zed.app").exists()
        || h.join("Applications/Zed.app").exists()
        || h.join(".config/zed/settings.json").exists()
    {
        found.push("zed".to_string());
    }

    // Kiro (AWS AI IDE)
    if std::path::Path::new("/Applications/Kiro.app").exists()
        || h.join("Applications/Kiro.app").exists()
        || which_exists("kiro")
    {
        found.push("kiro".to_string());
    }

    // OpenCode — terminal-based AI coding tool by SST
    if which_exists("opencode")
        || h.join(".config/opencode/opencode.json").exists()
        || h.join(".local/bin/opencode").is_file()
    {
        found.push("opencode".to_string());
    }

    found
}

/// Check if a binary is on PATH without shelling out (avoids injection risks).
fn which_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .any(|dir| dir.join(name).is_file())
        })
        .unwrap_or(false)
}

// ── Shared helper: find the sensei CLI binary ─────────────────────────────

fn find_sensei_binary() -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let search_dirs = [
        format!("{home}/.bun/bin"),
        format!("{home}/.local/bin"),
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
    ];
    // Prefer senseid (dedicated daemon binary) over sensei serve.
    // Falls back to sensei if senseid isn't installed yet.
    for name in &["senseid", "sensei"] {
        let preferred = std::path::PathBuf::from(&home).join(format!(".bun/bin/{name}"));
        if preferred.exists() {
            return Some(preferred.to_string_lossy().into_owned());
        }
        if let Some(found) = search_dirs.iter()
            .map(|dir| std::path::PathBuf::from(dir).join(name))
            .find(|p| p.exists())
        {
            return Some(found.to_string_lossy().into_owned());
        }
    }
    None
}

// ── MCP configuration ─────────────────────────────────────────────────────

#[tauri::command]
fn configure_mcp(acps: Vec<String>) -> Result<Vec<String>, String> {
    let h = home().ok_or("Cannot determine home directory")?;
    let sensei_cmd = find_sensei_binary()
        .ok_or_else(|| "sensei binary not found — install sensei globally first (bun install -g sensei)".to_string())?;
    // Use `sensei mcp` as the MCP server command.
    // When Claude Code launches the MCP server, it will call `sensei mcp` from
    // within the user's repo directory, so SENSEI_REPO_PATH falls back to cwd.
    let entry = json!({ "command": sensei_cmd, "args": ["mcp"] });

    let paths: std::collections::HashMap<&str, PathBuf> = [
        ("claude-desktop", h.join("Library/Application Support/Claude/claude_desktop_config.json")),
        ("claude-code",    h.join(".claude/settings.json")),
        ("cursor",         h.join(".cursor/mcp.json")),
        ("windsurf",       h.join(".codeium/windsurf/mcp_config.json")),
        ("zed",            h.join(".config/zed/settings.json")),
        ("kiro",      h.join(".kiro/settings/mcp.json")),
        ("opencode",  h.join(".config/opencode/opencode.json")),
    ].into();

    let mut ok = vec![];
    for id in &acps {
        let Some(path) = paths.get(id.as_str()) else { continue; };
        let mut cfg: Value = if path.exists() {
            let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            serde_json::from_str(&raw).unwrap_or(json!({}))
        } else {
            json!({})
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // OpenCode uses { mcp: { name: { type, command[] } } }; others use mcpServers.
        if id == "opencode" {
            cfg["mcp"]["sensei"] = json!({
                "type": "local",
                "command": [&sensei_cmd, "mcp"],
                "enabled": true
            });
        } else {
            cfg["mcpServers"]["sensei"] = entry.clone();
        }
        let out = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
        std::fs::write(path, out).map_err(|e| e.to_string())?;
        ok.push(id.clone());
    }
    Ok(ok)
}

// ── Repo ID reader ─────────────────────────────────────────────────────────

/// Read the repo_id from .sensei/config.yaml if it exists.
/// Returns None if the repo hasn't been initialized with `sensei init`.
#[tauri::command]
fn get_repo_id(path: String) -> Option<String> {
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

// ── Folder analysis ────────────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct AnalyzedRepo {
    name: String,
    path: String,
    remote: Option<String>,
    description: Option<String>,
    /// All applicable roles: any combination of app | library | tool | idea | unknown
    categories: Vec<String>,
    /// active | recent | stale | archived | abandoned
    status: String,
    last_commit_days: Option<u32>,
    tech_stack: Vec<String>,
    commit_count: u32,
    /// path of the original when this is an exact copy (same remote URL)
    duplicate_of: Option<String>,
    /// stem key shared by repos that appear to be variants of the same concept
    variant_group: Option<String>,
}

/// Scan `root` for all git repos (up to 4 levels deep) and analyze each one.
#[tauri::command]
fn analyze_folder(root: String) -> Vec<AnalyzedRepo> {
    let root = shellexpand::tilde(&root).to_string();
    let root_path = std::path::Path::new(&root);
    let mut repos = vec![];
    scan_dir(root_path, 0, 4, &mut repos);
    find_duplicates(&mut repos);
    find_variants(&mut repos);
    repos
}

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
            name,
            path: dir.to_string_lossy().to_string(),
            remote,
            description,
            categories,
            status,
            last_commit_days,
            tech_stack,
            commit_count,
            duplicate_of: None,
            variant_group: None,
        });
        return; // don't recurse into repos
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut children: Vec<_> = entries.flatten()
            .filter(|e| {
                let p = e.path();
                p.is_dir() && !is_ignored(&p)
            })
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

// ── Git helpers ────────────────────────────────────────────────────────────

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
    // COMMIT_EDITMSG is updated on every commit; .git/index on every stage
    for candidate in &[".git/COMMIT_EDITMSG", ".git/index"] {
        let p = repo.join(candidate);
        if p.exists() {
            if let Ok(meta) = std::fs::metadata(&p) {
                if let Ok(modified) = meta.modified() {
                    if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                        return Some((elapsed.as_secs() / 86400) as u32);
                    }
                }
            }
        }
    }
    None
}

fn estimate_commit_count(repo: &std::path::Path) -> u32 {
    // Each line in logs/HEAD is one operation (commit or checkout); good enough estimate
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

// ── Analysis helpers ───────────────────────────────────────────────────────

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
                ("\"@sveltejs/kit\"",   "sveltekit"),
                ("\"svelte\"",          "svelte"),
                ("\"next\"",            "nextjs"),
                ("\"react\"",           "react"),
                ("\"nuxt\"",            "nuxtjs"),
                ("\"vue\"",             "vue"),
                ("\"solid-js\"",        "solid"),
                ("\"@tauri-apps/api\"", "tauri"),
                ("\"electron\"",        "electron"),
                ("\"@angular/core\"",   "angular"),
            ];
            let mut found = false;
            for (key, label) in &frameworks {
                if src.contains(key) {
                    stack.push(label);
                    found = true;
                    break;
                }
            }
            if !found { stack.push("node"); }
        }
    }

    stack.into_iter().map(|s| s.to_string()).collect()
}

/// A repo can serve multiple roles simultaneously.
/// e.g. a package with both `exports` and `bin` is both a library and a tool.
/// Returns the full set so the UI can show it in every applicable group.
fn detect_categories(
    repo: &std::path::Path,
    tech_stack: &[String],
    last_commit_days: Option<u32>,
    commit_count: u32,
) -> Vec<String> {
    let mut cats: Vec<&str> = vec![];

    // --- Rust: [lib] and/or [[bin]] — both can coexist
    if let Ok(cargo) = std::fs::read_to_string(repo.join("Cargo.toml")) {
        if cargo.contains("[lib]")   { cats.push("library"); }
        if cargo.contains("[[bin]]") { cats.push("tool"); }
        if !cats.is_empty() {
            return cats.into_iter().map(|s| s.to_string()).collect();
        }
    }

    // --- Go: cmd/ subdirectory = CLI tool; importable package = library
    if repo.join("go.mod").exists() {
        let has_cmd = repo.join("cmd").exists();
        // If there are .go files at the root it's importable as a library
        let has_lib_root = std::fs::read_dir(repo)
            .map(|d| d.flatten().any(|e| {
                e.path().extension().map(|x| x == "go").unwrap_or(false)
            }))
            .unwrap_or(false);
        if has_lib_root { cats.push("library"); }
        if has_cmd      { cats.push("tool"); }
        if !cats.is_empty() {
            return cats.into_iter().map(|s| s.to_string()).collect();
        }
    }

    // --- Node/TS: check library/tool BEFORE app-framework.
    //     "exports"/"main" → published API (library).
    //     "bin"            → CLI entry points (tool).
    //     Both can coexist with each other and with an app demo (src/routes).
    if let Ok(pkg) = std::fs::read_to_string(repo.join("package.json")) {
        let has_bin = pkg.contains("\"bin\"");
        // Published-API indicators: standard JS/TS export fields, plus the Svelte
        // package field used by @sveltejs/package. Any of these signals a library.
        let has_exports = pkg.contains("\"exports\"")
            || pkg.contains("\"main\"")
            || pkg.contains("\"module\"")
            || pkg.contains("\"svelte\"");
        // svelte-package is the SvelteKit library build tool — its presence confirms
        // this is a library even if the export fields haven't been generated yet.
        let is_svelte_pkg = pkg.contains("svelte-package")
            || pkg.contains("@sveltejs/package");
        // App routes = the repo also ships a runnable UI
        let has_app_routes = repo.join("src/routes").exists()
            || repo.join("pages").exists()
            || repo.join("app").exists();

        if has_exports || is_svelte_pkg { cats.push("library"); }
        if has_bin { cats.push("tool"); }
        if has_app_routes {
            let is_app_fw = tech_stack.iter().any(|s| {
                matches!(s.as_str(), "sveltekit"|"nextjs"|"nuxtjs"|"tauri"|"electron"|"angular")
            });
            // Only add "app" if it also has a proper framework and is not already a library
            if is_app_fw && !cats.contains(&"library") { cats.push("app"); }
        }
        if !cats.is_empty() {
            return cats.into_iter().map(|s| s.to_string()).collect();
        }
    }

    // --- Pure app frameworks (no package exports)
    let is_app_fw = tech_stack.iter().any(|s| {
        matches!(s.as_str(), "sveltekit"|"nextjs"|"nuxtjs"|"tauri"|"electron"|"angular")
    });
    if is_app_fw {
        return vec!["app".to_string()];
    }

    if tech_stack.iter().any(|s| matches!(s.as_str(), "react"|"vue"|"svelte"|"solid")) {
        return vec!["app".to_string()];
    }

    // Low-activity prototype
    let old = last_commit_days.map(|d| d > 120).unwrap_or(false);
    if old && commit_count < 15 {
        return vec!["idea".to_string()];
    }

    vec!["unknown".to_string()]
}

fn detect_status(last_commit_days: Option<u32>, commit_count: u32) -> String {
    match last_commit_days {
        None               => "unknown".to_string(),
        Some(d) if d <= 30  => "active".to_string(),
        Some(d) if d <= 90  => "recent".to_string(),
        Some(d) if d <= 365 => "stale".to_string(),
        // Stable/finished repos are archived, not abandoned.
        // Only "abandoned" for repos that barely started AND sat untouched >2 years.
        Some(d) if commit_count < 3 && d > 730 => "abandoned".to_string(),
        Some(_)             => "archived".to_string(),
    }
}

fn normalize_remote(url: &str) -> String {
    url.trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .replace("git@github.com:", "github.com/")
        .replace("https://github.com/", "github.com/")
        .replace("http://github.com/", "github.com/")
        .replace("git@gitlab.com:", "gitlab.com/")
        .replace("https://gitlab.com/", "gitlab.com/")
        .to_lowercase()
}

fn find_duplicates(repos: &mut Vec<AnalyzedRepo>) {
    // Map normalized remote → first-seen path
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for repo in repos.iter() {
        if let Some(remote) = &repo.remote {
            let key = normalize_remote(remote);
            if !key.is_empty() {
                seen.entry(key).or_insert_with(|| repo.path.clone());
            }
        }
    }
    // Flag repos whose remote is already claimed by an earlier entry
    for repo in repos.iter_mut() {
        if let Some(remote) = &repo.remote {
            let key = normalize_remote(remote);
            if let Some(original) = seen.get(&key) {
                if *original != repo.path {
                    repo.duplicate_of = Some(original.clone());
                }
            }
        }
    }
}

/// Strip version/variant suffixes and trailing digits to get a concept stem.
/// e.g. "auth-v2" → "auth", "myapp-old" → "myapp", "project2" → "project"
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


/// Detect variant clusters using name-stem matching only.
///
/// Repos whose names reduce to the same stem after stripping version/variant
/// suffixes are grouped together: auth, auth-v2, auth-old → group "auth".
///
/// Concept/description-based adjacency is intentionally omitted: it creates
/// false transitive chains (A↔B via "auth", B↔C via "form" ⟹ A+C wrongly grouped).
fn find_variants(repos: &mut Vec<AnalyzedRepo>) {
    use std::collections::HashMap;

    // Collect all repo names (lowercased) for prefix-based matching.
    // If repo B's name starts with repo A's full name + "-" or "_",
    // treat A's name as B's stem (e.g. "fitness" and "fitness-assistant" → "fitness").
    let names_lower: Vec<String> = repos.iter()
        .map(|r| r.name.to_lowercase())
        .collect();

    let stem_for = |name: &str| -> String {
        let suffix_stem = name_stem(name);
        let nl = name.to_lowercase();
        // Check if any other (shorter) repo name is a full prefix of this name
        let prefix_stem = names_lower.iter()
            .filter(|other| {
                other.len() >= 4
                    && other.len() < nl.len()
                    && (nl.starts_with(&format!("{}-", other))
                        || nl.starts_with(&format!("{}_", other)))
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
            for i in group {
                repos[i].variant_group = Some(stem.clone());
            }
        }
    }
}

// ── Dependency detection ───────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct DetectedDep {
    name: String,
    file_count: usize,
    total_files: usize,
    usage_pct: f32,
}

/// Scan a repo's source files and return external package import counts.
#[tauri::command]
fn detect_dependencies(path: String) -> Vec<DetectedDep> {
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
    deps.sort_by(|a, b| b.file_count.cmp(&a.file_count));
    deps.truncate(60);
    deps
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
            if !is_ignored(&path) {
                collect_imports(&path, counts, total);
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "svelte" | "vue") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    *total += 1;
                    let pkgs = extract_file_imports(&content);
                    let mut seen = std::collections::HashSet::new();
                    for pkg in pkgs {
                        if seen.insert(pkg.clone()) {
                            *counts.entry(pkg).or_insert(0) += 1;
                        }
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
        // ESM: import/export ... from 'pkg'
        if let Some(pos) = l.rfind(" from ") {
            if let Some(pkg) = parse_pkg_specifier(l[pos + 6..].trim()) {
                pkgs.push(pkg);
            }
        }
        // CJS: require('pkg')
        if let Some(pos) = l.find("require(") {
            if let Some(pkg) = parse_pkg_specifier(l[pos + 8..].trim()) {
                pkgs.push(pkg);
            }
        }
    }
    pkgs
}

fn parse_pkg_specifier(s: &str) -> Option<String> {
    let s = s.trim();
    let quote = s.chars().next()?;
    if !matches!(quote, '\'' | '"' | '`') { return None; }
    let inner = &s[1..];
    // Relative or absolute paths are not external packages
    if inner.starts_with('.') || inner.starts_with('/') { return None; }
    if inner.starts_with('@') {
        // Scoped: @org/name
        let inner = &inner[1..];
        let slash = inner.find('/')?;
        let org = &inner[..slash];
        let rest = &inner[slash + 1..];
        let end = rest.find(|c: char| matches!(c, '\'' | '"' | '`' | '/'))?;
        let name = &rest[..end];
        if !org.is_empty() && !name.is_empty() {
            return Some(format!("@{}/{}", org, name));
        }
    } else {
        let end = inner.find(|c: char| matches!(c, '\'' | '"' | '`' | '/'))?;
        let name = &inner[..end];
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.')) {
            return Some(name.to_string());
        }
    }
    None
}

// ── ACP config status ──────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct AcpStatus {
    id: String,
    name: String,
    installed: bool,
    mcp_configured: bool,
    config_path: String,
}

#[tauri::command]
fn check_acp_configs() -> Vec<AcpStatus> {
    let Some(h) = home() else { return vec![]; };
    let installed_ids = detect_acps();

    let specs: &[(&str, &str, bool)] = &[
        ("claude-desktop", "Claude Desktop", false),
        ("claude-code",    "Claude Code",    false),
        ("cursor",         "Cursor",         false),
        ("windsurf",       "Windsurf",       false),
        ("zed",            "Zed",            false),
        ("kiro",           "Kiro",           false),
        ("opencode",       "OpenCode",       true),
    ];

    let paths: std::collections::HashMap<&str, std::path::PathBuf> = [
        ("claude-desktop", h.join("Library/Application Support/Claude/claude_desktop_config.json")),
        ("claude-code",    h.join(".claude/settings.json")),
        ("cursor",         h.join(".cursor/mcp.json")),
        ("windsurf",       h.join(".codeium/windsurf/mcp_config.json")),
        ("zed",            h.join(".config/zed/settings.json")),
        ("kiro",           h.join(".kiro/settings/mcp.json")),
        ("opencode",       h.join(".config/opencode/opencode.json")),
    ].into();

    specs.iter().map(|(id, name, is_opencode)| {
        let config_path = paths.get(id).cloned().unwrap_or_default();
        let installed = installed_ids.contains(&id.to_string());
        let mcp_configured = config_path.exists()
            && std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .map(|v| if *is_opencode { v["mcp"]["sensei"].is_object() } else { v["mcpServers"]["sensei"].is_object() })
                .unwrap_or(false);

        AcpStatus {
            id: id.to_string(),
            name: name.to_string(),
            installed,
            mcp_configured,
            config_path: config_path.to_string_lossy().into_owned(),
        }
    }).collect()
}

// ── Tauri entry point ──────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[derive(serde::Serialize)]
struct IndexerStatus {
    online: bool,
    name: Option<String>,
    version: Option<String>,
    indexing: Vec<String>,
    backend: Option<String>,
    ollama_running: bool,
    ollama_model: bool,
}

#[tauri::command]
fn check_indexer(port: Option<u16>) -> IndexerStatus {
    let port = port.unwrap_or(7744);
    let offline = IndexerStatus { online: false, name: None, version: None, indexing: vec![], backend: None, ollama_running: false, ollama_model: false };
    if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_err() {
        return offline;
    }
    match ureq::get(&format!("http://127.0.0.1:{port}/health")).call() {
        Ok(resp) => {
            match resp.into_json::<serde_json::Value>() {
                Ok(v) => IndexerStatus {
                    online: true,
                    name: v.get("name").and_then(|x| x.as_str()).map(str::to_owned),
                    version: v.get("version").and_then(|x| x.as_str()).map(str::to_owned),
                    indexing: v.get("indexing")
                        .and_then(|x| x.as_array())
                        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect())
                        .unwrap_or_default(),
                    backend: v.get("backend").and_then(|x| x.as_str()).map(str::to_owned),
                    ollama_running: v.get("ollamaRunning").and_then(|x| x.as_bool()).unwrap_or(false),
                    ollama_model: v.get("ollamaModel").and_then(|x| x.as_bool()).unwrap_or(false),
                },
                Err(_) => offline,
            }
        }
        Err(_) => offline,
    }
}

#[tauri::command]
fn start_indexer(port: Option<u16>) -> Result<(), String> {
    let port = port.unwrap_or(7744);
    let home = std::env::var("HOME").unwrap_or_default();

    let extra_paths = [
        format!("{home}/.bun/bin"),
        format!("{home}/.local/bin"),
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
    ];
    let current_path = std::env::var("PATH").unwrap_or_default();
    let mut all_paths: Vec<String> = extra_paths.to_vec();
    for p in current_path.split(':') {
        if !all_paths.contains(&p.to_string()) {
            all_paths.push(p.to_string());
        }
    }
    let full_path = all_paths.join(":");

    let sensei_path = find_sensei_binary()
        .ok_or_else(|| format!(
            "sensei binary not found. Searched: {}",
            all_paths.join(", ")
        ))?;

    // Log file so startup errors are visible.
    let log_path = "/tmp/sensei-serve.log";
    let log_file = std::fs::OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(log_path)
        .map_err(|e| format!("Cannot open log file: {e}"))?;
    let log_err = log_file.try_clone().map_err(|e| e.to_string())?;

    // If something is already listening on the target port, we're done.
    if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
        return Ok(());
    }

    // senseid takes no subcommand; sensei needs "serve" subcommand.
    let is_daemon_bin = sensei_path.ends_with("senseid");
    let port_str = port.to_string();
    let args: Vec<&str> = if is_daemon_bin {
        vec!["--port", &port_str]
    } else {
        vec!["serve", "--port", &port_str]
    };

    let mut child = std::process::Command::new(&sensei_path)
        .args(&args)
        .env("HOME", &home)
        .env("PATH", &full_path)
        .stdin(std::process::Stdio::null())
        .stdout(log_file)
        .stderr(log_err)
        .spawn()
        .map_err(|e| format!("Failed to spawn '{sensei_path}': {e}"))?;

    // Give the server up to 8 seconds to bind on port 7744.
    for _ in 0..16 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited — read the log for the error message.
                let log = std::fs::read_to_string(log_path).unwrap_or_default();
                return Err(format!(
                    "sensei serve exited ({status}). Log:\n{log}"
                ));
            }
            Ok(None) => {} // still running
            Err(e) => return Err(format!("Error watching child: {e}")),
        }
        // Check if port 7744 is accepting connections.
        if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
            return Ok(());
        }
    }

    // Still not up after 8s — leave it running, caller will poll /health.
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            detect_acps,
            configure_mcp,
            check_acp_configs,
            get_repo_id,
            analyze_folder,
            detect_dependencies,
            start_indexer,
            check_indexer,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                let _ = apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None);
            }

            #[cfg(debug_assertions)]
            window.open_devtools();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running sensei desktop")
}
