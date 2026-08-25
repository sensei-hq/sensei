/// Fetch library content from URL (async part).
pub async fn fetch_lib_url(url: &str) -> Result<String, String> {
    fetch_url(url).await
}

/// Index pre-fetched library content (sync part — safe to hold mutex).
/// Legacy single-URL path; superseded by [`resolve_library_pages`] for the
/// multi-source component-derivation flow, retained for the direct-URL API.
#[allow(dead_code)]
pub fn index_lib_content(
    lib_name: &str,
    url: &str,
    content: &str,
    version: Option<&str>,
) -> Result<LibIndexResult, String> {
    let source_type = detect_source_type(url, content);

    let docs = match source_type.as_str() {
        "llms.txt" => parse_llms_txt(content, lib_name),
        "markdown" => parse_markdown(content, lib_name, url),
        _ => parse_markdown(content, lib_name, url),
    };

    let indexed = docs.len() as u32;

    Ok(LibIndexResult {
        lib_name: lib_name.to_string(),
        docs_indexed: indexed,
        source_type,
        version: version.map(|v| v.to_string()),
    })
}

/// Fetch and store dependency versions from a project's manifest files.
///
/// Dispatches through the `ManifestAdapter` registry: any registered ecosystem
/// (`npm`, `cargo`, `pypi`, `go`) contributes its dep entries. New ecosystems
/// only require a new adapter — this function never grows another branch.
pub fn extract_dep_versions(_repo_id: &str, repo_path: &str) -> Result<Vec<DepVersion>, String> {
    let repo = std::path::Path::new(repo_path);
    let mut deps = Vec::new();

    for filename in ["package.json", "Cargo.toml", "pyproject.toml", "go.mod"] {
        let Ok(content) = std::fs::read_to_string(repo.join(filename)) else {
            continue;
        };
        let Some(adapter) = crate::adapters::manifest::manifest_adapter_for_filename(filename)
        else {
            continue;
        };
        deps.extend(adapter.parse_dependencies(&content));
    }

    Ok(deps)
}

/// Detect the local-dep marker in an npm version string.
///
/// Returns the payload after the protocol prefix for:
///   - `link:../foo` → `Some("../foo")` — local sibling folder
///   - `file:./local-tgz.tgz` → `Some("./local-tgz.tgz")` — local tarball
///   - `workspace:*` / `workspace:^1.2.3` → `Some("*")` / `Some("^1.2.3")` — resolved by dep NAME within the workspace
///
/// Git / http / npm-registry versions return `None`.
pub(crate) fn npm_local_source(version: &str) -> Option<String> {
    for prefix in ["link:", "workspace:", "file:"] {
        if let Some(rest) = version.strip_prefix(prefix) {
            return Some(rest.to_string());
        }
    }
    None
}

/// Detect the local-dep marker in a Cargo dependency table.
///
/// Cargo local deps use `{ path = "../sibling" }`. The `path` key alone is
/// enough — `version` may be present alongside `path` (for publishing a
/// hybrid dep) but the resolution still goes local first, so we treat it as
/// local for the folder→project rollup.
///
/// Returns `Some(path_string)` when a `path` key is present, `None` otherwise.
pub(crate) fn cargo_local_source(dep_value: &toml::Value) -> Option<String> {
    dep_value.as_table().and_then(|t| t.get("path")).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Parse npm dependency sections from a `package.json` value.
///
/// Emits one `DepVersion` per (name, section) pair across `dependencies`,
/// `devDependencies`, and `peerDependencies`. Local-protocol versions
/// (`link:`, `workspace:`, `file:`) are tagged via `local_source` so the
/// caller can route them to `project_dependencies` instead of writing them
/// as external libraries.
pub(crate) fn parse_npm_deps(pkg: &serde_json::Value) -> Vec<DepVersion> {
    let mut out = Vec::new();
    for section in &["dependencies", "devDependencies", "peerDependencies"] {
        let Some(obj) = pkg.get(section).and_then(|v| v.as_object()) else { continue };
        for (name, ver) in obj {
            let version = ver.as_str().unwrap_or("*").to_string();
            let local_source = npm_local_source(&version);
            out.push(DepVersion {
                lib_name: name.clone(),
                version: clean_version(&version),
                raw_version: version,
                source: "package.json".into(),
                dev: *section != "dependencies",
                local_source,
            });
        }
    }
    out
}

/// Parse Cargo dependency sections from a `Cargo.toml` value.
///
/// Emits one `DepVersion` per (name, section) pair across `dependencies`,
/// `dev-dependencies`, and `build-dependencies`. Deps declared with `path = "..."`
/// are tagged via `local_source` for local rollup.
pub(crate) fn parse_cargo_deps(cargo: &toml::Value) -> Vec<DepVersion> {
    let mut out = Vec::new();
    for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(obj) = cargo.get(section).and_then(|v| v.as_table()) else { continue };
        for (name, ver) in obj {
            let version = match ver {
                toml::Value::String(s) => s.clone(),
                toml::Value::Table(t) => {
                    t.get("version").and_then(|v| v.as_str()).unwrap_or("*").to_string()
                }
                _ => "*".into(),
            };
            let local_source = cargo_local_source(ver);
            out.push(DepVersion {
                lib_name: name.clone(),
                version: clean_version(&version),
                raw_version: version,
                source: "Cargo.toml".into(),
                dev: *section != "dependencies",
                local_source,
            });
        }
    }
    out
}

/// Parse `[project.dependencies]` from a `pyproject.toml` value.
///
/// PEP 621 only. `[tool.poetry.dependencies]` and `[tool.uv.sources]` local
/// deps are on the 1b/1c roadmap.
pub(crate) fn parse_pyproject_deps(pyp: &toml::Value) -> Vec<DepVersion> {
    let mut out = Vec::new();
    let Some(deps_arr) =
        pyp.get("project").and_then(|v| v.get("dependencies")).and_then(|v| v.as_array())
    else {
        return out;
    };
    for dep in deps_arr {
        if let Some(s) = dep.as_str() {
            let (name, ver) = parse_pep508(s);
            out.push(DepVersion {
                lib_name: name,
                version: ver.clone(),
                raw_version: ver,
                source: "pyproject.toml".into(),
                dev: false,
                local_source: None,
            });
        }
    }
    out
}

#[allow(dead_code)] // legacy single-URL index result; see index_lib_content
#[derive(Debug, Clone, serde::Serialize)]
pub struct LibIndexResult {
    pub lib_name: String,
    pub docs_indexed: u32,
    pub source_type: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DepVersion {
    pub lib_name: String,
    pub version: String,
    pub raw_version: String,
    pub source: String,
    pub dev: bool,
    /// When the dep resolves to a local sibling (npm `link:`, `workspace:`,
    /// `file:`; Cargo `path=`), the payload after the protocol prefix (or the
    /// path string for Cargo). `None` for registry / git / http deps. The
    /// writer routes `Some(_)` deps to `project_dependencies` and skips the
    /// external-library upsert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_source: Option<String>,
}

#[allow(dead_code)] // planned: full lib doc API response type
#[derive(Debug, Clone, serde::Serialize)]
pub struct LibDoc {
    pub id: String,
    pub title: String,
    pub url: String,
    pub summary: String,
    pub content: Option<String>,
    pub source_type: String,
    pub component: String,
    pub indexed_at: String,
}

pub struct ParsedDoc {
    pub title: String,
    pub summary: String,
    pub content: String,
    pub component: Option<String>,
}

/// Fetch a URL. Public so MCP handler can use it.
pub async fn fetch_lib_url_with_timeout(url: &str, timeout_secs: u64) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client.get(url).send().await.map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Read body: {}", e))
}

async fn fetch_url(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client.get(url).send().await.map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Read body: {}", e))
}

/// Legacy source-type sniffer for the single-URL path. The multi-source flow
/// uses [`detect_lib_source`] + explicit enum labels; kept for the direct-URL
/// helpers ([`index_lib_content`], [`parse_docs`]).
#[allow(dead_code)]
pub fn detect_source_type(url: &str, content: &str) -> String {
    if url.ends_with("llms.txt") || url.ends_with("llms-full.txt") {
        return "llms.txt".into();
    }
    if url.contains("raw.githubusercontent.com") || url.ends_with(".md") {
        return "markdown".into();
    }
    if content.starts_with("# ") || content.contains("\n## ") {
        return "markdown".into();
    }
    "text".into()
}

/// Parse content into documentation sections, auto-detecting format. Legacy
/// heading-splitter for a single fetched URL; the component-derivation flow
/// uses [`docs_from_source_files`] / [`parse_single_file`] instead.
#[allow(dead_code)]
pub fn parse_docs(content: &str, lib_name: &str, url: &str) -> Vec<ParsedDoc> {
    let source_type = detect_source_type(url, content);
    match source_type.as_str() {
        "llms.txt" => parse_llms_txt(content, lib_name),
        _ => parse_markdown(content, lib_name, url),
    }
}

/// Parse llms.txt format — sections delimited by `# heading` with content.
#[allow(dead_code)] // legacy heading-splitter; used by index_lib_content/parse_docs + tests
fn parse_llms_txt(content: &str, lib_name: &str) -> Vec<ParsedDoc> {
    let mut docs = Vec::new();
    let mut current_title = String::new();
    let mut current_content = String::new();

    for line in content.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            // Flush previous section
            if !current_title.is_empty() && !current_content.trim().is_empty() {
                let summary = current_content.lines().take(3).collect::<Vec<_>>().join(" ");
                docs.push(ParsedDoc {
                    title: current_title.clone(),
                    summary: summary.chars().take(200).collect(),
                    content: current_content.trim().to_string(),
                    component: None,
                });
            }
            current_title = title.trim().to_string();
            current_content.clear();
        } else if let Some(title) = line.strip_prefix("## ") {
            // Sub-section becomes component
            if !current_title.is_empty() && !current_content.trim().is_empty() {
                let summary = current_content.lines().take(3).collect::<Vec<_>>().join(" ");
                docs.push(ParsedDoc {
                    title: current_title.clone(),
                    summary: summary.chars().take(200).collect(),
                    content: current_content.trim().to_string(),
                    component: None,
                });
            }
            current_title = title.trim().to_string();
            current_content.clear();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Flush last section
    if !current_title.is_empty() && !current_content.trim().is_empty() {
        let summary = current_content.lines().take(3).collect::<Vec<_>>().join(" ");
        docs.push(ParsedDoc {
            title: current_title,
            summary: summary.chars().take(200).collect(),
            content: current_content.trim().to_string(),
            component: None,
        });
    }

    if docs.is_empty() {
        // Treat entire content as one doc
        let summary = content.lines().take(3).collect::<Vec<_>>().join(" ");
        docs.push(ParsedDoc {
            title: lib_name.to_string(),
            summary: summary.chars().take(200).collect(),
            content: content.to_string(),
            component: None,
        });
    }

    docs
}

#[allow(dead_code)] // legacy markdown splitter; used by index_lib_content/parse_docs
fn parse_markdown(content: &str, lib_name: &str, _url: &str) -> Vec<ParsedDoc> {
    // Same as llms_txt for now — split by headings
    parse_llms_txt(content, lib_name)
}

// ── Component-derivation & multi-source llms ingestion ──────────────────────
//
// Populates `library_pages` with PER-COMPONENT pages so `get_lib_docs(name,
// component)` resolves to the right page. Two layouts are supported:
//   1. per-file `components/<name>.txt` + top-level `.txt` (e.g. rokkit)
//   2. single-file `llms-full.txt` with `##`/`### <lib> <cmd>` sections (dbd)
// …across three sources: local dir, website llms URL, GitHub tree URL.

/// Where a library's llms docs come from. Detected from the trigger `url`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibSource {
    /// A local directory on disk (may be the llms root or a repo root).
    LocalDir(String),
    /// A `github.com/owner/repo/tree/branch/path` docs directory.
    GitHubTree { owner: String, repo: String, branch: String, path: String },
    /// A website llms URL (index or single-file), e.g. `https://x/llms/index.txt`.
    Website(String),
}

/// A single fetched source file (its stem/name + raw content). `stem` is the
/// filename without extension (kebab), used to derive the component when a page
/// maps 1:1 to a file (per-file layout).
pub struct SourceFile {
    /// Filename stem, kebab (e.g. `list`, `cli`, `index`, `llms-full`).
    pub stem: String,
    /// Raw file content.
    pub content: String,
    /// Origin URL or local path for the page's `url`/`local_path` column.
    pub location: String,
    /// True when this file lives under a `components/` directory.
    pub is_component_file: bool,
}

/// Slugify a heading into a component id: lowercase, spaces/punct → `-`,
/// collapse repeats, trim leading/trailing `-`.
pub fn slug(heading: &str) -> String {
    let mut out = String::with_capacity(heading.len());
    let mut prev_dash = false;
    for ch in heading.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Derive a component id from a section heading, stripping a leading
/// library-name token so `### dbd deploy` → `deploy`, `## Commands` →
/// `commands`, `### Global options` → `global-options`. When the heading is a
/// single token equal to the lib name, keeps it (avoids an empty component).
pub fn component_from_heading(heading: &str, lib_name: &str) -> String {
    let full = slug(heading);
    let lib = slug(lib_name);
    if lib.is_empty() {
        return full;
    }
    // Strip a leading `<lib>-` prefix (the "dbd" in "### dbd deploy").
    if let Some(rest) = full.strip_prefix(&format!("{lib}-"))
        && !rest.is_empty()
    {
        return rest.to_string();
    }
    full
}

/// Component id for a per-file page from its filename stem. `index`, `llms`,
/// `llms-full` map to the overview (returns `None`); anything else → the stem.
pub fn component_from_stem(stem: &str) -> Option<String> {
    let s = slug(stem);
    match s.as_str() {
        "" | "index" | "llms" | "llms-full" => None,
        _ => Some(s),
    }
}

/// Build a `ParsedDoc` with a 200-char summary from the first lines of content.
fn make_doc(title: String, content: String, component: Option<String>) -> ParsedDoc {
    let summary: String = content.lines().take(3).collect::<Vec<_>>().join(" ");
    ParsedDoc {
        title,
        summary: summary.chars().take(200).collect(),
        content: content.trim().to_string(),
        component,
    }
}

/// Split a single-file llms doc (`llms-full.txt`) into per-section pages.
///
/// The `#` title + any preamble before the first `##`/`###` heading becomes the
/// overview page (`component = None`). Every `##` and `###` heading starts a new
/// page whose `component` is derived via [`component_from_heading`]. Deeper
/// headings (`####`+) stay inside the current section's body.
pub fn parse_single_file(content: &str, lib_name: &str) -> Vec<ParsedDoc> {
    let mut docs = Vec::new();
    let mut overview_title = lib_name.to_string();
    let mut preamble = String::new();
    let mut cur_heading: Option<String> = None;
    let mut cur_body = String::new();
    let mut seen_section = false;

    // Emit a page per heading even when its body is empty: a `## Commands`
    // section whose body is just deeper sub-sections is still a real page
    // (e.g. dbd's `## Commands` → `### dbd deploy`). Only a blank heading is
    // skipped.
    let flush = |docs: &mut Vec<ParsedDoc>, heading: &str, body: &str| {
        if heading.trim().is_empty() {
            return;
        }
        let component = component_from_heading(heading, lib_name);
        let component = if component.is_empty() { None } else { Some(component) };
        // Fall back to the heading as content when the section body is empty so
        // the page is never stored with empty content.
        let content =
            if body.trim().is_empty() { heading.trim().to_string() } else { body.to_string() };
        docs.push(make_doc(heading.trim().to_string(), content, component));
    };

    for line in content.lines() {
        // Section boundary: `## ` or `### ` (but not `#### `+).
        let is_section = (line.starts_with("## ") && !line.starts_with("### "))
            || (line.starts_with("### ") && !line.starts_with("#### "));

        if let Some(title) = line.strip_prefix("# ").filter(|_| !line.starts_with("## ")) {
            // Top-level title for the overview.
            if !seen_section {
                overview_title = title.trim().to_string();
            }
            continue;
        }

        if is_section {
            if let Some(h) = cur_heading.take() {
                flush(&mut docs, &h, &cur_body);
            }
            let heading = line.trim_start_matches('#').trim().to_string();
            cur_heading = Some(heading);
            cur_body.clear();
            seen_section = true;
        } else if cur_heading.is_some() {
            cur_body.push_str(line);
            cur_body.push('\n');
        } else {
            preamble.push_str(line);
            preamble.push('\n');
        }
    }
    if let Some(h) = cur_heading.take() {
        flush(&mut docs, &h, &cur_body);
    }

    // Overview page from the title + preamble.
    if !preamble.trim().is_empty() {
        docs.insert(0, make_doc(overview_title, preamble, None));
    } else if docs.is_empty() {
        docs.push(make_doc(lib_name.to_string(), content.to_string(), None));
    }
    docs
}

/// Turn a set of fetched source files into per-component pages. Prefers the
/// per-file `components/` layout; if the only meaningful file is a single
/// `llms-full.txt` it is section-split; otherwise each `.txt` stem is a page.
pub fn docs_from_source_files(files: &[SourceFile], lib_name: &str) -> Vec<ParsedDoc> {
    let has_component_dir = files.iter().any(|f| f.is_component_file);

    // Single-file layout: no component dir and a lone llms-full among the files.
    let full = files.iter().find(|f| f.stem == "llms-full");
    if !has_component_dir && let Some(full) = full {
        let non_index: Vec<&SourceFile> = files
            .iter()
            .filter(|f| f.stem != "llms" && f.stem != "index" && f.stem != "llms-full")
            .collect();
        if non_index.is_empty() {
            return parse_single_file(&full.content, lib_name);
        }
    }

    // Per-file layout (components/ or plain top-level stems).
    let mut docs = Vec::new();
    for f in files {
        // Overview from index/llms; skip llms-full when other files exist.
        if f.stem == "llms-full" {
            continue;
        }
        let component =
            if f.is_component_file { Some(slug(&f.stem)) } else { component_from_stem(&f.stem) };
        let title = match &component {
            Some(c) => c.clone(),
            None => lib_name.to_string(),
        };
        docs.push(make_doc(title, f.content.clone(), component));
    }
    if docs.is_empty()
        && let Some(full) = full
    {
        return parse_single_file(&full.content, lib_name);
    }
    docs
}

// ── Source detection ────────────────────────────────────────────────────────

/// Classify the trigger `url`: an existing path on disk → local; a github.com
/// host → GitHub tree; otherwise a website llms URL.
pub fn detect_lib_source(url: &str) -> LibSource {
    if std::path::Path::new(url).exists() {
        return LibSource::LocalDir(url.to_string());
    }
    if let Some(gh) = parse_github_tree_url(url) {
        return gh;
    }
    LibSource::Website(url.to_string())
}

/// Parse a `https://github.com/{owner}/{repo}/tree/{branch}/{path}` URL.
/// Returns `None` when the host is not github.com or the shape doesn't match.
pub fn parse_github_tree_url(url: &str) -> Option<LibSource> {
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("github.com/"))?;
    let mut parts = rest.split('/');
    let owner = parts.next().filter(|s| !s.is_empty())?.to_string();
    let repo = parts.next().filter(|s| !s.is_empty())?.to_string();
    // Accept `tree` or `blob` as the ref marker.
    match parts.next() {
        Some("tree") | Some("blob") => {}
        _ => return None,
    }
    let branch = parts.next().filter(|s| !s.is_empty())?.to_string();
    let path = parts.collect::<Vec<_>>().join("/");
    let path = path.trim_end_matches('/').to_string();
    Some(LibSource::GitHubTree { owner, repo, branch, path })
}

/// GitHub contents API URL for a directory at a given ref.
pub fn github_contents_url(owner: &str, repo: &str, path: &str, branch: &str) -> String {
    if path.is_empty() {
        format!("https://api.github.com/repos/{owner}/{repo}/contents?ref={branch}")
    } else {
        format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={branch}")
    }
}

// ── Website link parsing ────────────────────────────────────────────────────

/// Extract markdown-link targets that point at `.txt` doc pages from an index.
/// Returns the raw hrefs (e.g. `/llms/components/list.txt`, `cli.txt`).
pub fn extract_doc_links(index: &str) -> Vec<String> {
    let mut links = Vec::new();
    let bytes = index.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Find `](` then read until `)`.
        if bytes[i] == b']' && i + 1 < bytes.len() && bytes[i + 1] == b'(' {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b')' && bytes[j] != b' ' {
                j += 1;
            }
            if j <= bytes.len() {
                let href = &index[start..j.min(index.len())];
                if href.ends_with(".txt") && !href.starts_with("http") {
                    links.push(href.to_string());
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    links.sort();
    links.dedup();
    links
}

/// Resolve a possibly-relative doc href against the index URL. Root-absolute
/// (`/llms/x.txt`) resolves against the origin; relative (`cli.txt`) against the
/// index URL's directory.
pub fn resolve_link(index_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    let origin = origin_of(index_url);
    if let Some(stripped) = href.strip_prefix('/') {
        return format!("{origin}/{stripped}");
    }
    // Relative to the index URL's directory.
    let dir = index_url.rsplit_once('/').map(|(d, _)| d).unwrap_or(index_url);
    format!("{dir}/{href}")
}

/// The scheme+host of a URL (`https://host.tld`), or the input on failure.
fn origin_of(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://") {
        let host = rest.split('/').next().unwrap_or(rest);
        return format!("https://{host}");
    }
    if let Some(rest) = url.strip_prefix("http://") {
        let host = rest.split('/').next().unwrap_or(rest);
        return format!("http://{host}");
    }
    url.to_string()
}

/// Filename stem (kebab) from a URL or path (`/llms/components/list.txt` →
/// `list`).
pub fn stem_from_path(p: &str) -> String {
    let name = p.rsplit('/').next().unwrap_or(p);
    let name = name.split('?').next().unwrap_or(name);
    let stem = name.strip_suffix(".txt").unwrap_or(name);
    slug(stem)
}

// ── Resolvers (fetch + assemble SourceFiles per source) ─────────────────────

/// Find the llms docs root for a local path: the path itself when it ends in
/// `llms` / contains `index.txt` / `llms.txt`, else `<path>/docs/llms`.
pub fn resolve_local_llms_root(path: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    let looks_like_root = p.file_name().and_then(|n| n.to_str()) == Some("llms")
        || p.join("index.txt").exists()
        || p.join("llms.txt").exists()
        || p.join("llms-full.txt").exists();
    if looks_like_root {
        return p.to_path_buf();
    }
    p.join("docs").join("llms")
}

/// Read `.txt` files from a local llms root (top level + `components/`).
pub fn read_local_source_files(root: &std::path::Path) -> Result<Vec<SourceFile>, String> {
    if !root.is_dir() {
        return Err(format!("llms root not a directory: {}", root.display()));
    }
    let mut files = Vec::new();
    collect_txt_files(root, false, &mut files);
    let comp = root.join("components");
    if comp.is_dir() {
        collect_txt_files(&comp, true, &mut files);
    }
    if files.is_empty() {
        return Err(format!("no .txt files under {}", root.display()));
    }
    Ok(files)
}

fn collect_txt_files(dir: &std::path::Path, is_component: bool, out: &mut Vec<SourceFile>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, dir = %dir.display(), "read_local_source_files: read_dir failed");
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).map(slug).unwrap_or_default();
        match std::fs::read_to_string(&path) {
            Ok(content) => out.push(SourceFile {
                stem,
                content,
                location: path.to_string_lossy().to_string(),
                is_component_file: is_component,
            }),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "read_local_source_files: read failed")
            }
        }
    }
}

/// Fetch website llms docs: parse the index for `.txt` links; when links exist,
/// fetch each as a component page; otherwise treat the index as a single file.
pub async fn fetch_website_source_files(index_url: &str) -> Result<Vec<SourceFile>, String> {
    let index = fetch_lib_url_with_timeout(index_url, 15).await?;
    let links = extract_doc_links(&index);
    let index_stem = stem_from_path(index_url);

    if links.is_empty() {
        // Single-file: index IS the content (llms.txt / llms-full.txt).
        return Ok(vec![SourceFile {
            stem: if index_stem.is_empty() { "index".into() } else { index_stem },
            content: index,
            location: index_url.to_string(),
            is_component_file: false,
        }]);
    }

    let mut files = vec![SourceFile {
        stem: "index".into(),
        content: index,
        location: index_url.to_string(),
        is_component_file: false,
    }];
    for href in links {
        let full = resolve_link(index_url, &href);
        let is_component = href.contains("/components/");
        match fetch_lib_url_with_timeout(&full, 15).await {
            Ok(content) => files.push(SourceFile {
                stem: stem_from_path(&href),
                content,
                location: full,
                is_component_file: is_component,
            }),
            Err(e) => {
                tracing::warn!(error = %e, url = %full, "fetch_website_source_files: page fetch failed")
            }
        }
    }
    Ok(files)
}

/// Fetch GitHub-tree llms docs via the contents API. Lists the directory (and
/// `components/` subdir) and downloads each `.txt` via its `download_url`.
pub async fn fetch_github_source_files(
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<SourceFile>, String> {
    let mut files = Vec::new();
    fetch_github_dir(owner, repo, branch, path, false, &mut files).await?;
    // Recurse into components/ if present.
    let comp_path =
        if path.is_empty() { "components".to_string() } else { format!("{path}/components") };
    if let Err(e) = fetch_github_dir(owner, repo, branch, &comp_path, true, &mut files).await {
        tracing::warn!(error = %e, path = %comp_path, "fetch_github_source_files: components/ dir skipped");
    }
    if files.is_empty() {
        return Err(format!("no .txt files at github {owner}/{repo}/{path}@{branch}"));
    }
    Ok(files)
}

async fn fetch_github_dir(
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    is_component: bool,
    out: &mut Vec<SourceFile>,
) -> Result<(), String> {
    let api = github_contents_url(owner, repo, path, branch);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;
    let resp = client
        .get(&api)
        .header("User-Agent", "sensei-daemon")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("GitHub API fetch failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API HTTP {} for {api}", resp.status()));
    }
    let body = resp.text().await.map_err(|e| format!("GitHub API read body: {e}"))?;
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(&body).map_err(|e| format!("GitHub API parse: {e}"))?;

    for entry in entries {
        if entry["type"].as_str() != Some("file") {
            continue;
        }
        let name = entry["name"].as_str().unwrap_or("");
        if !name.ends_with(".txt") {
            continue;
        }
        let Some(download_url) = entry["download_url"].as_str() else { continue };
        match client.get(download_url).header("User-Agent", "sensei-daemon").send().await {
            Ok(r) if r.status().is_success() => match r.text().await {
                Ok(content) => out.push(SourceFile {
                    stem: stem_from_path(name),
                    content,
                    location: download_url.to_string(),
                    is_component_file: is_component,
                }),
                Err(e) => {
                    tracing::warn!(error = %e, url = %download_url, "fetch_github_dir: read body failed")
                }
            },
            Ok(r) => {
                tracing::warn!(status = %r.status(), url = %download_url, "fetch_github_dir: raw fetch non-200")
            }
            Err(e) => {
                tracing::warn!(error = %e, url = %download_url, "fetch_github_dir: raw fetch failed")
            }
        }
    }
    Ok(())
}

/// A page ready to store: derived component + title/summary/content + where it
/// came from (a URL for remote pages, a local path for local pages). The
/// `source_type` (`'local'` vs `'llms.txt'`/`'http'`) already distinguishes a
/// local page from a remote one, so no separate flag is carried.
pub struct ResolvedPage {
    pub doc: ParsedDoc,
    /// Origin location — an http(s) URL, or a local filesystem path.
    pub location: String,
    /// `library_source_type` enum value: `'llms.txt' | 'http' | 'local'`.
    pub source_type: &'static str,
}

/// Resolve a [`LibSource`] to a set of storable pages (fetch + parse). Async
/// because website/GitHub sources hit the network; local is filesystem-only.
pub async fn resolve_library_pages(
    source: &LibSource,
    lib_name: &str,
) -> Result<Vec<ResolvedPage>, String> {
    match source {
        LibSource::LocalDir(path) => {
            let root = resolve_local_llms_root(path);
            let files = read_local_source_files(&root)?;
            let docs = docs_from_source_files(&files, lib_name);
            Ok(pages_from(docs, &files, "local"))
        }
        LibSource::Website(url) => {
            let files = fetch_website_source_files(url).await?;
            let docs = docs_from_source_files(&files, lib_name);
            Ok(pages_from(docs, &files, "llms.txt"))
        }
        LibSource::GitHubTree { owner, repo, branch, path } => {
            let files = fetch_github_source_files(owner, repo, branch, path).await?;
            let docs = docs_from_source_files(&files, lib_name);
            Ok(pages_from(docs, &files, "http"))
        }
    }
}

/// Attach a source location to each derived doc. A doc's location is the source
/// file whose derived component matches; falls back to the first file (index).
fn pages_from(
    docs: Vec<ParsedDoc>,
    files: &[SourceFile],
    source_type: &'static str,
) -> Vec<ResolvedPage> {
    let default_loc = files.first().map(|f| f.location.clone()).unwrap_or_default();
    docs.into_iter()
        .map(|doc| {
            let location = doc
                .component
                .as_ref()
                .and_then(|c| {
                    files.iter().find(|f| {
                        let stem_comp = if f.is_component_file {
                            slug(&f.stem)
                        } else {
                            component_from_stem(&f.stem).unwrap_or_default()
                        };
                        &stem_comp == c
                    })
                })
                .map(|f| f.location.clone())
                .unwrap_or_else(|| default_loc.clone());
            ResolvedPage { doc, location, source_type }
        })
        .collect()
}

pub(crate) fn clean_version(v: &str) -> String {
    v.trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches(">=")
        .trim_start_matches("<=")
        .trim_start_matches('>')
        .trim_start_matches('<')
        .trim_start_matches('=')
        .trim()
        .to_string()
}

fn parse_pep508(spec: &str) -> (String, String) {
    // "requests>=2.28" → ("requests", "2.28")
    let parts: Vec<&str> = spec.splitn(2, ['>', '<', '=', '!', '[']).collect();
    let name = parts[0].trim().to_string();
    let version =
        if parts.len() > 1 { clean_version(parts[1].trim_start_matches('=')) } else { "*".into() };
    (name, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_llms_txt_sections() {
        let content = "# Overview\nThis is a test lib.\n\n## Components\nButton, Card\n\n# API\nSome API docs.";
        let docs = parse_llms_txt(content, "test-lib");
        assert!(docs.len() >= 2);
        assert_eq!(docs[0].title, "Overview");
    }

    #[test]
    fn clean_semver() {
        assert_eq!(clean_version("^1.2.3"), "1.2.3");
        assert_eq!(clean_version("~0.8"), "0.8");
        assert_eq!(clean_version(">=2.0"), "2.0");
    }

    #[test]
    fn parse_pep508_spec() {
        let (name, ver) = parse_pep508("requests>=2.28");
        assert_eq!(name, "requests");
        assert_eq!(ver, "2.28");

        let (name, ver) = parse_pep508("flask");
        assert_eq!(name, "flask");
        assert_eq!(ver, "*");
    }

    // ── local-source detection (1a Step 2) ─────────────────────────────
    //
    // Local-protocol deps (npm `link:`/`workspace:`/`file:`, Cargo `path=`)
    // must be tagged so extract_deps routes them to project_dependencies
    // instead of writing them as external libraries.

    #[test]
    fn npm_local_source_recognises_link_workspace_file() {
        assert_eq!(npm_local_source("link:../actions"), Some("../actions".to_string()));
        assert_eq!(npm_local_source("workspace:*"), Some("*".to_string()));
        assert_eq!(npm_local_source("workspace:^1.2.3"), Some("^1.2.3".to_string()));
        assert_eq!(npm_local_source("file:./local-tgz.tgz"), Some("./local-tgz.tgz".to_string()));
    }

    #[test]
    fn npm_local_source_ignores_registry_and_git_versions() {
        assert_eq!(npm_local_source("^1.2.3"), None);
        assert_eq!(npm_local_source("~0.8.0"), None);
        assert_eq!(npm_local_source("*"), None);
        assert_eq!(npm_local_source("github:owner/repo"), None);
        assert_eq!(npm_local_source("git+https://example.com/pkg.git"), None);
    }

    #[test]
    fn cargo_local_source_reads_path_key() {
        let v: toml::Value = toml::from_str(r#"path = "../actions""#).unwrap();
        // toml::from_str returns a table containing the key; wrap so the
        // helper sees the same shape as it would inline in the dep table.
        assert_eq!(cargo_local_source(&v), Some("../actions".to_string()));
    }

    #[test]
    fn cargo_local_source_none_for_registry_string_dep() {
        let v = toml::Value::String("1.2.3".into());
        assert_eq!(cargo_local_source(&v), None);
    }

    #[test]
    fn cargo_local_source_reads_path_even_with_version_present() {
        let src = r#"
            version = "1.2.3"
            path = "../actions"
        "#;
        let v: toml::Value = toml::from_str(src).unwrap();
        assert_eq!(cargo_local_source(&v), Some("../actions".to_string()));
    }

    // ── ecosystem parsers (1a Step 2) ──────────────────────────────────

    #[test]
    fn parse_npm_deps_tags_link_and_workspace_versions() {
        let pkg: serde_json::Value = serde_json::json!({
            "dependencies": {
                "d3": "^7.0.0",
                "@rokkit/actions": "link:../actions",
                "@rokkit/states": "workspace:*",
            },
            "devDependencies": {
                "vitest": "^1.0.0"
            }
        });
        let deps = parse_npm_deps(&pkg);
        assert_eq!(deps.len(), 4);
        let by_name = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert_eq!(by_name("d3").local_source, None);
        assert!(!by_name("d3").dev);
        assert_eq!(by_name("@rokkit/actions").local_source, Some("../actions".into()));
        assert_eq!(by_name("@rokkit/states").local_source, Some("*".into()));
        assert!(by_name("vitest").dev);
    }

    #[test]
    fn parse_cargo_deps_tags_path_dep_and_reads_string_version() {
        let src = r#"
            [dependencies]
            serde = "1.0"
            senseid = { path = "../senseid" }
            gateway = { version = "0.2", path = "../gateway" }
            tracing = { version = "0.1" }

            [dev-dependencies]
            tempfile = "3"
        "#;
        let cargo: toml::Value = toml::from_str(src).unwrap();
        let deps = parse_cargo_deps(&cargo);
        assert_eq!(deps.len(), 5);
        let by_name = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert_eq!(by_name("serde").local_source, None);
        assert!(!by_name("serde").dev);
        assert_eq!(by_name("senseid").local_source, Some("../senseid".into()));
        assert_eq!(by_name("gateway").local_source, Some("../gateway".into()));
        assert_eq!(by_name("tracing").local_source, None);
        assert!(by_name("tempfile").dev);
    }

    #[test]
    fn parse_pyproject_deps_reads_pep621_dependencies() {
        let src = r#"
            [project]
            name = "example"
            dependencies = [
                "requests>=2.28",
                "flask",
            ]
        "#;
        let pyp: toml::Value = toml::from_str(src).unwrap();
        let deps = parse_pyproject_deps(&pyp);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].lib_name, "requests");
        assert_eq!(deps[0].local_source, None);
        assert_eq!(deps[1].lib_name, "flask");
    }

    // ── source_type detection (existing) ────────────────────────────────

    #[test]
    fn detect_llms_txt() {
        // Must be the exact enum label 'llms.txt' (dotted), not "llms-txt".
        assert_eq!(detect_source_type("https://example.com/llms.txt", ""), "llms.txt");
        assert_eq!(detect_source_type("https://example.com/llms-full.txt", ""), "llms.txt");
        assert_eq!(detect_source_type("https://example.com/README.md", "# Hi"), "markdown");
    }

    // ── slug / component derivation ─────────────────────────────────────────

    #[test]
    fn slug_basics() {
        assert_eq!(slug("Global options"), "global-options");
        assert_eq!(slug("dbd deploy"), "dbd-deploy");
        assert_eq!(slug("  File path → entity name "), "file-path-entity-name");
        assert_eq!(slug("Commands"), "commands");
    }

    #[test]
    fn component_from_heading_strips_lib_token() {
        assert_eq!(component_from_heading("dbd deploy", "dbd"), "deploy");
        assert_eq!(
            component_from_heading("### dbd deploy".trim_start_matches('#').trim(), "dbd"),
            "deploy"
        );
        assert_eq!(component_from_heading("Commands", "dbd"), "commands");
        assert_eq!(component_from_heading("Global options", "dbd"), "global-options");
        // A heading that is only the lib name keeps it (never empty).
        assert_eq!(component_from_heading("dbd", "dbd"), "dbd");
    }

    #[test]
    fn component_from_stem_maps_overview_and_files() {
        assert_eq!(component_from_stem("index"), None);
        assert_eq!(component_from_stem("llms"), None);
        assert_eq!(component_from_stem("llms-full"), None);
        assert_eq!(component_from_stem("cli"), Some("cli".to_string()));
        assert_eq!(component_from_stem("List"), Some("list".to_string()));
    }

    // ── single-file section split ───────────────────────────────────────────

    #[test]
    fn single_file_split_yields_component_pages_and_overview() {
        let content = "\
# dbd — Full Reference

> A CLI for schemas as code.

## Commands

### Global options

Flags that apply everywhere.

### dbd deploy

Applies pending changes to the target database.
Use --dry-run to preview.
";
        let docs = parse_single_file(content, "dbd");

        // Overview page (no component) from the title + preamble.
        let overview = docs.iter().find(|d| d.component.is_none()).expect("overview page");
        assert_eq!(overview.title, "dbd — Full Reference");
        assert!(overview.content.contains("CLI for schemas as code"));

        // `### dbd deploy` → component "deploy" with the deploy body.
        let deploy =
            docs.iter().find(|d| d.component.as_deref() == Some("deploy")).expect("deploy page");
        assert!(deploy.content.contains("Applies pending changes"));

        // `## Commands` → "commands", `### Global options` → "global-options".
        assert!(docs.iter().any(|d| d.component.as_deref() == Some("commands")));
        assert!(docs.iter().any(|d| d.component.as_deref() == Some("global-options")));
    }

    // ── per-file layout via SourceFiles ─────────────────────────────────────

    #[test]
    fn per_file_layout_maps_components_and_overview() {
        let files = vec![
            SourceFile {
                stem: "index".into(),
                content: "# Overview\nAll about the lib.".into(),
                location: "/x/index.txt".into(),
                is_component_file: false,
            },
            SourceFile {
                stem: "list".into(),
                content: "# List\nA vertical list.".into(),
                location: "/x/components/list.txt".into(),
                is_component_file: true,
            },
            SourceFile {
                stem: "cli".into(),
                content: "# CLI\nThe CLI.".into(),
                location: "/x/cli.txt".into(),
                is_component_file: false,
            },
        ];
        let docs = docs_from_source_files(&files, "rokkit");

        assert!(docs.iter().any(|d| d.component.is_none()), "index → overview");
        assert_eq!(
            docs.iter()
                .find(|d| d.component.as_deref() == Some("list"))
                .map(|d| d.content.contains("vertical list")),
            Some(true),
        );
        assert!(docs.iter().any(|d| d.component.as_deref() == Some("cli")), "cli.txt → cli");
    }

    #[test]
    fn source_files_prefer_component_dir_over_llms_full() {
        // components/ present ⇒ per-file layout wins even if llms-full exists.
        let files = vec![
            SourceFile {
                stem: "llms-full".into(),
                content: "# X\n## y\nbody".into(),
                location: "/x/llms-full.txt".into(),
                is_component_file: false,
            },
            SourceFile {
                stem: "button".into(),
                content: "# Button\nA button.".into(),
                location: "/x/components/button.txt".into(),
                is_component_file: true,
            },
        ];
        let docs = docs_from_source_files(&files, "rokkit");
        assert!(docs.iter().any(|d| d.component.as_deref() == Some("button")));
        // llms-full is skipped when other files exist.
        assert!(!docs.iter().any(|d| d.component.as_deref() == Some("y")));
    }

    #[test]
    fn source_files_single_llms_full_is_section_split() {
        let files = vec![SourceFile {
            stem: "llms-full".into(),
            content: "# dbd\n> lead\n## Commands\n### dbd deploy\nbody".into(),
            location: "/x/llms-full.txt".into(),
            is_component_file: false,
        }];
        let docs = docs_from_source_files(&files, "dbd");
        assert!(docs.iter().any(|d| d.component.as_deref() == Some("deploy")));
        assert!(docs.iter().any(|d| d.component.is_none()));
    }

    // ── local directory layout (tempfile) ───────────────────────────────────

    #[test]
    fn read_local_source_files_from_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join("index.txt"), "# Overview\nlib docs").unwrap();
        std::fs::write(root.join("cli.txt"), "# CLI\ncli docs").unwrap();
        let comp = root.join("components");
        std::fs::create_dir_all(&comp).unwrap();
        std::fs::write(comp.join("list.txt"), "# List\nlist docs").unwrap();

        let files = read_local_source_files(root).expect("read files");
        let docs = docs_from_source_files(&files, "rokkit");

        // components/list.txt → component "list"
        assert!(docs.iter().any(|d| d.component.as_deref() == Some("list")), "list component");
        // index.txt → overview (None)
        assert!(docs.iter().any(|d| d.component.is_none()), "overview");
        // cli.txt top-level → component "cli"
        assert!(docs.iter().any(|d| d.component.as_deref() == Some("cli")), "cli component");
    }

    #[test]
    fn resolve_local_llms_root_variants() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // A dir with index.txt is itself the root.
        std::fs::write(root.join("index.txt"), "x").unwrap();
        assert_eq!(resolve_local_llms_root(root.to_str().unwrap()), root.to_path_buf());

        // A repo root without llms markers → <path>/docs/llms.
        let repo = tempfile::tempdir().unwrap();
        let expected = repo.path().join("docs").join("llms");
        assert_eq!(resolve_local_llms_root(repo.path().to_str().unwrap()), expected);
    }

    // ── GitHub / website URL parsing (pure, no network) ─────────────────────

    #[test]
    fn parse_github_tree_url_ok() {
        let s = parse_github_tree_url("https://github.com/jerrythomas/rokkit/tree/main/docs/llms");
        assert_eq!(
            s,
            Some(LibSource::GitHubTree {
                owner: "jerrythomas".into(),
                repo: "rokkit".into(),
                branch: "main".into(),
                path: "docs/llms".into(),
            })
        );
        // blob works too; trailing slash trimmed.
        let s = parse_github_tree_url("https://github.com/o/r/blob/dev/a/b/");
        assert_eq!(
            s,
            Some(LibSource::GitHubTree {
                owner: "o".into(),
                repo: "r".into(),
                branch: "dev".into(),
                path: "a/b".into(),
            })
        );
        // Non-github and malformed → None.
        assert_eq!(parse_github_tree_url("https://example.com/x"), None);
        assert_eq!(parse_github_tree_url("https://github.com/only-owner"), None);
    }

    #[test]
    fn github_contents_url_shape() {
        assert_eq!(
            github_contents_url("o", "r", "docs/llms", "main"),
            "https://api.github.com/repos/o/r/contents/docs/llms?ref=main"
        );
        assert_eq!(
            github_contents_url("o", "r", "", "main"),
            "https://api.github.com/repos/o/r/contents?ref=main"
        );
    }

    #[test]
    fn detect_lib_source_classifies() {
        assert!(matches!(
            detect_lib_source("https://github.com/o/r/tree/main/docs/llms"),
            LibSource::GitHubTree { .. }
        ));
        assert!(matches!(
            detect_lib_source("https://rokkit.sensei-hq.com/llms/index.txt"),
            LibSource::Website(_)
        ));
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(detect_lib_source(dir.path().to_str().unwrap()), LibSource::LocalDir(_)));
    }

    #[test]
    fn extract_and_resolve_links() {
        let index = "\
# Index
See [cli.txt](/llms/cli.txt) — CLI reference
- [List](/llms/components/list.txt) — vertical list
- [rel](guides/getting-started.txt) — relative
- [ext](https://other.com/x.txt) — external (ignored, absolute)
- [dir](/llms/packages/) — not a .txt (ignored)
";
        let links = extract_doc_links(index);
        assert!(links.contains(&"/llms/cli.txt".to_string()));
        assert!(links.contains(&"/llms/components/list.txt".to_string()));
        assert!(links.contains(&"guides/getting-started.txt".to_string()));
        assert!(!links.iter().any(|l| l.ends_with("packages/")));

        let base = "https://rokkit.sensei-hq.com/llms/index.txt";
        assert_eq!(
            resolve_link(base, "/llms/components/list.txt"),
            "https://rokkit.sensei-hq.com/llms/components/list.txt"
        );
        assert_eq!(
            resolve_link(base, "guides/getting-started.txt"),
            "https://rokkit.sensei-hq.com/llms/guides/getting-started.txt"
        );
        assert_eq!(resolve_link(base, "https://x/y.txt"), "https://x/y.txt");
    }

    #[test]
    fn stem_from_path_extracts_kebab() {
        assert_eq!(stem_from_path("/llms/components/list.txt"), "list");
        assert_eq!(stem_from_path("cli.txt"), "cli");
        assert_eq!(stem_from_path("https://x/llms/index.txt?ref=1"), "index");
    }

    // Real-corpus smoke checks — ignored by default (path-dependent on local
    // dev checkouts); run with `--ignored` to verify against the actual repos.
    #[test]
    #[ignore]
    fn real_dbd_single_file_has_deploy_component() {
        let root = resolve_local_llms_root("/Users/Jerry/Developer/dbd-rs");
        let files = read_local_source_files(&root).expect("dbd llms files");
        let docs = docs_from_source_files(&files, "dbd");
        let deploy =
            docs.iter().find(|d| d.component.as_deref() == Some("deploy")).expect("deploy page");
        assert!(deploy.content.contains("deploy"), "deploy body present");
        assert!(docs.iter().any(|d| d.component.is_none()), "overview present");
    }

    #[test]
    #[ignore]
    fn real_rokkit_per_file_has_list_component() {
        let root = resolve_local_llms_root("/Users/Jerry/Developer/rokkit");
        let files = read_local_source_files(&root).expect("rokkit llms files");
        let docs = docs_from_source_files(&files, "rokkit");
        assert!(docs.iter().any(|d| d.component.as_deref() == Some("list")), "list component");
        assert!(
            docs.iter().any(|d| d.component.as_deref() == Some("cli")),
            "cli top-level component"
        );
        assert!(docs.iter().any(|d| d.component.is_none()), "index overview");
    }
}
