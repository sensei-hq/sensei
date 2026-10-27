//! The repositories a scanned repo's DEPENDENCIES come from — where to look for
//! their `sensei.library.json` (#341).
//!
//! A published package rarely carries its library's manifest (`@rokkit/ui` on npm
//! does not); its REPOSITORY does, at the root. So a dependency is followed to its
//! repository, read off what is already on disk:
//!
//! - npm: the installed package's `package.json` `repository` field;
//! - cargo: a `git+https://…` source in `Cargo.lock`, with the tag or revision
//!   it is pinned at.
//!
//! GitHub repositories only: that is where a manifest can be read without
//! cloning. Nothing here touches the network.

use std::collections::BTreeMap;
use std::path::Path;

/// A dependency's repository, and the ref the dependent is pinned to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DependencyRepo {
    pub owner: String,
    pub repo: String,
    /// A tag or a commit; `None` reads the default branch.
    pub git_ref: Option<String>,
    pub ecosystem: String,
    /// The dependencies that led here (`@rokkit/ui`, `@rokkit/core`), sorted —
    /// linked to the library it turns out to be, since a manifest read from the
    /// repository need not declare its packages.
    pub packages: Vec<String>,
}

impl DependencyRepo {
    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.repo)
    }

    /// `owner/repo` — what a discovery task is keyed by.
    pub fn key(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

/// The GitHub `(owner, repo)` an npm `repository` field names, in any spelling
/// npm allows: a URL (`git+https://…`, `git@…:`, a `/tree/…` path), `github:o/r`,
/// the `o/r` shorthand, or `{"url": …}`. `None` for anything else.
pub fn github_repository(value: &serde_json::Value) -> Option<(String, String)> {
    let raw = match value {
        serde_json::Value::String(s) => s.as_str(),
        serde_json::Value::Object(o) => o.get("url")?.as_str()?,
        _ => return None,
    };
    let rest = if let Some(r) = raw.strip_prefix("github:") {
        r
    } else if let Some(i) = raw.find("github.com") {
        raw[i + "github.com".len()..].trim_start_matches([':', '/'])
    } else if !raw.contains(':') && raw.split('/').count() == 2 {
        raw // the `owner/repo` shorthand
    } else {
        return None;
    };
    let mut parts = rest.split('/').filter(|p| !p.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?.trim_end_matches(".git");
    (!owner.is_empty() && !repo.is_empty()).then(|| (owner.to_string(), repo.to_string()))
}

/// The GitHub git sources in a `Cargo.lock`, as `(package, owner, repo, ref)`:
/// the `tag=`/`rev=`/`branch=` it is pinned at, else the commit after `#`.
pub fn cargo_lock_git_sources(lock: &str) -> Vec<(String, String, String, Option<String>)> {
    let mut out = Vec::new();
    let mut package = String::new();
    for line in lock.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("name = \"") {
            package = name.trim_end_matches('"').to_string();
            continue;
        }
        let Some(src) = line.strip_prefix("source = \"git+") else { continue };
        let src = src.trim_end_matches('"');
        let (url, commit) = src.split_once('#').map_or((src, None), |(u, c)| (u, Some(c)));
        let (url, query) = url.split_once('?').map_or((url, None), |(u, q)| (u, Some(q)));
        let Some((owner, repo)) = github_repository(&serde_json::Value::String(url.to_string()))
        else {
            continue;
        };
        let pinned = query
            .and_then(|q| {
                q.split('&').find_map(|kv| {
                    let (k, v) = kv.split_once('=')?;
                    matches!(k, "tag" | "rev" | "branch").then(|| v.to_string())
                })
            })
            .or_else(|| commit.map(str::to_string));
        out.push((package.clone(), owner, repo, pinned));
    }
    out
}

/// Every GitHub repository a repo's dependencies come from, once each (by
/// owner/repo, first sighting wins), from what is on disk: the `package.json`s
/// given (each dependency's installed `package.json`, beside the manifest or at
/// the repo root) and the repo root's `Cargo.lock`.
pub fn dependency_repos(
    repo_root: &Path,
    package_jsons: &[std::path::PathBuf],
) -> Vec<DependencyRepo> {
    // Keyed by owner/repo; the first sighting fixes the ref and ecosystem, and
    // every dependency that leads there is collected.
    let mut by_repo: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut out: Vec<DependencyRepo> = Vec::new();
    let mut add = |package: String,
                   owner: String,
                   repo: String,
                   git_ref: Option<String>,
                   ecosystem: &'static str| {
        let i = *by_repo.entry((owner.clone(), repo.clone())).or_insert_with(|| {
            out.push(DependencyRepo {
                owner,
                repo,
                git_ref,
                ecosystem: ecosystem.to_string(),
                packages: Vec::new(),
            });
            out.len() - 1
        });
        if !out[i].packages.contains(&package) {
            out[i].packages.push(package);
            out[i].packages.sort();
        }
    };
    for manifest in package_jsons {
        let Ok(text) = std::fs::read_to_string(manifest) else { continue };
        let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let dir = manifest.parent().unwrap_or(repo_root);
        let names = ["dependencies", "devDependencies", "peerDependencies"]
            .iter()
            .filter_map(|k| pkg.get(*k).and_then(|d| d.as_object()))
            .flat_map(|d| d.keys().cloned().collect::<Vec<_>>());
        for name in names {
            let installed = [dir, repo_root]
                .iter()
                .map(|d| d.join("node_modules").join(&name).join("package.json"))
                .find(|p| p.is_file());
            let Some(installed) = installed else { continue };
            let Some((owner, repo)) = std::fs::read_to_string(&installed)
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|p| p.get("repository").and_then(github_repository))
            else {
                continue;
            };
            add(name.clone(), owner, repo, None, "npm");
        }
    }
    if let Ok(lock) = std::fs::read_to_string(repo_root.join("Cargo.lock")) {
        for (package, owner, repo, git_ref) in cargo_lock_git_sources(&lock) {
            add(package, owner, repo, git_ref, "cargo");
        }
    }
    out
}

/// A dependency's `sensei.library.json`, read from its repository at the ref it
/// is pinned to (the default branch when unpinned), with each skill's and agent's
/// body read from the same repository and ref. `fetch` answers a URL with its
/// text, `None` for a 404. `Ok(None)` when the repository ships no manifest — a
/// repository that is not a library is the common case, not an error.
pub async fn read_remote_manifest<F, Fut>(
    dep: &DependencyRepo,
    fetch: F,
) -> Result<Option<super::ResolvedManifest>, String>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<Option<String>, String>>,
{
    let base = format!(
        "https://raw.githubusercontent.com/{}/{}/{}",
        dep.owner,
        dep.repo,
        dep.git_ref.as_deref().unwrap_or("HEAD")
    );
    let Some(text) = fetch(format!("{base}/{}", super::manifest::MANIFEST_FILENAME)).await? else {
        return Ok(None);
    };
    let m = super::manifest::parse_library_manifest(&text)?;
    let mut bodies = std::collections::HashMap::new();
    let paths = m
        .skills
        .iter()
        .filter(|s| s.body.is_none())
        .filter_map(|s| s.path.clone())
        .chain(m.agents.iter().filter(|a| a.body.is_none()).filter_map(|a| a.path.clone()));
    for path in paths.filter(|p| !p.contains("..")).collect::<Vec<_>>() {
        if let Some(body) = fetch(format!("{base}/{}", path.trim_start_matches('/'))).await? {
            bodies.insert(path, body);
        }
    }
    Ok(Some(super::resolved_from(m, &|p: &str| bodies.get(p).cloned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_field_is_read_in_every_spelling_npm_allows() {
        let gh = |o: &str, r: &str| Some((o.to_string(), r.to_string()));
        for (value, want) in [
            (
                serde_json::json!("git+https://github.com/jerrythomas/rokkit.git"),
                gh("jerrythomas", "rokkit"),
            ),
            (
                serde_json::json!({"type": "git", "url": "git+https://github.com/jerrythomas/rokkit.git"}),
                gh("jerrythomas", "rokkit"),
            ),
            (serde_json::json!("github:sensei-hq/kavach"), gh("sensei-hq", "kavach")),
            (serde_json::json!("sensei-hq/kavach"), gh("sensei-hq", "kavach")),
            (
                serde_json::json!("https://github.com/sensei-hq/dbd/tree/main/site"),
                gh("sensei-hq", "dbd"),
            ),
            (serde_json::json!("git@github.com:sensei-hq/dbd.git"), gh("sensei-hq", "dbd")),
            (serde_json::json!("https://gitlab.com/a/b"), None),
            (serde_json::json!(42), None),
        ] {
            assert_eq!(github_repository(&value), want, "{value}");
        }
    }

    #[test]
    fn a_cargo_lock_git_source_names_its_repository_and_pin() {
        let lock = r#"
[[package]]
name = "dbd-core"
version = "0.27.8"
source = "git+https://github.com/sensei-hq/dbd?tag=v0.27.8#c2823592f4c1cfaf907942a3be26216971f498d4"

[[package]]
name = "gateway-embedded"
source = "git+https://github.com/sensei-hq/gateway?rev=4f2a1b9#4f2a1b9aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[package]]
name = "serde"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;
        let got = cargo_lock_git_sources(lock);
        let row = |p: &str, r: &str, pin: &str| {
            (p.to_string(), "sensei-hq".to_string(), r.to_string(), Some(pin.to_string()))
        };
        assert_eq!(
            got,
            vec![row("dbd-core", "dbd", "v0.27.8"), row("gateway-embedded", "gateway", "4f2a1b9")]
        );
    }

    #[test]
    fn a_repos_dependencies_lead_to_their_repositories_once_each() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        std::fs::write(
            root.join("package.json"),
            r#"{"dependencies": {"@rokkit/ui": "^1"}, "devDependencies": {"@rokkit/core": "^1", "left-pad": "1"}}"#,
        )
        .unwrap();
        for (pkg, repo) in [
            (
                "@rokkit/ui",
                r#"{"repository": {"url": "git+https://github.com/jerrythomas/rokkit.git"}}"#,
            ),
            ("@rokkit/core", r#"{"repository": "github:jerrythomas/rokkit"}"#),
            ("left-pad", r#"{"name": "left-pad"}"#),
        ] {
            let dir = root.join("node_modules").join(pkg);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("package.json"), repo).unwrap();
        }
        std::fs::write(
            root.join("Cargo.lock"),
            "[[package]]\nname = \"dbd-core\"\nsource = \"git+https://github.com/sensei-hq/dbd?tag=v0.27.8#c282359\"\n",
        )
        .unwrap();

        let got = dependency_repos(root, &[root.join("package.json")]);
        assert_eq!(
            got,
            vec![
                DependencyRepo {
                    owner: "jerrythomas".into(),
                    repo: "rokkit".into(),
                    git_ref: None,
                    ecosystem: "npm".into(),
                    packages: vec!["@rokkit/core".into(), "@rokkit/ui".into()],
                },
                DependencyRepo {
                    owner: "sensei-hq".into(),
                    repo: "dbd".into(),
                    git_ref: Some("v0.27.8".into()),
                    ecosystem: "cargo".into(),
                    packages: vec!["dbd-core".into()],
                },
            ],
            "rokkit once, though two of its packages are used; left-pad names no repository"
        );
    }

    /// A dependency's manifest is read from its repository at the pinned ref,
    /// and each skill's body from the same repository and ref.
    #[tokio::test]
    async fn a_dependencys_manifest_is_read_from_its_repository() {
        let dep = DependencyRepo {
            owner: "sensei-hq".into(),
            repo: "dbd".into(),
            git_ref: Some("v0.27.8".into()),
            ecosystem: "cargo".into(),
            packages: vec!["dbd-core".into()],
        };
        let base = "https://raw.githubusercontent.com/sensei-hq/dbd/v0.27.8";
        let files: std::collections::HashMap<String, String> = [
            (
                format!("{base}/sensei.library.json"),
                r#"{"library":"dbd","version":">=0.10","llms":{"path":"docs/llms"},
                    "skills":[{"name":"dbd","focus":"schema","path":"docs/skills/dbd/SKILL.md"},
                              {"name":"escape","focus":"x","path":"../secrets"}]}"#
                    .to_string(),
            ),
            (format!("{base}/docs/skills/dbd/SKILL.md"), "# dbd skill".to_string()),
        ]
        .into_iter()
        .collect();
        let asked = std::sync::Mutex::new(Vec::new());
        let fetch = |url: String| {
            asked.lock().unwrap().push(url.clone());
            let hit = files.get(&url).cloned();
            async move { Ok(hit) }
        };
        let m = read_remote_manifest(&dep, fetch).await.unwrap().expect("a library");
        assert_eq!(m.library, "dbd");
        assert_eq!(m.llms_path.as_deref(), Some("docs/llms"));
        assert_eq!(m.skills[0].body.as_deref(), Some("# dbd skill"));
        assert_eq!(m.skills[1].body, None, "a path escaping the repository is never fetched");
        assert!(!asked.lock().unwrap().iter().any(|u| u.contains("secrets")));
    }

    /// No manifest in the repository: not a library, and not an error.
    #[tokio::test]
    async fn a_repository_without_a_manifest_is_not_a_library() {
        let dep = DependencyRepo {
            owner: "o".into(),
            repo: "r".into(),
            git_ref: None,
            ecosystem: "npm".into(),
            packages: vec![],
        };
        let got = read_remote_manifest(&dep, |_url: String| async { Ok(None) }).await.unwrap();
        assert!(got.is_none());
    }
}
