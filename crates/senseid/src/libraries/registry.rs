//! Per-source "latest available version" resolution for the update scheduler (F v0).
//!
//! Pure URL/extract helpers (unit-tested against sample bodies, no network) + a
//! [`VersionSource`] trait so the scheduler tick is testable with a stub. Covers the
//! big-3 registries + go proxy + local manifests in v0; other ecosystems (maven/
//! nuget/rubygems/composer/swiftpm) and GitHub Releases are deferred. `docs` has no
//! semver and is out of the version-drift path entirely.
//!
//! FAIL-CLOSED: any fetch/parse miss returns `None` — the caller sends no notice and
//! never fabricates a latest version.

use async_trait::async_trait;

/// What one registry lookup yielded: the latest version, and the URLs the SAME
/// response carried (02b S8).
///
/// Returned together because they arrive together. Fetching the version and
/// then fetching the URLs would be two calls for one body — and the reason
/// 1,119 of 1,121 library rows had no URL is precisely that this response was
/// already being downloaded and the URLs thrown away.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LatestInfo {
    pub version: String,
    pub urls: RegistryUrls,
}

impl LatestInfo {
    /// A version with no URLs — what a local manifest read yields.
    pub fn version_only(version: String) -> Self {
        Self { version, urls: RegistryUrls::default() }
    }
}

/// Resolves the latest published version of a library. A trait so the scheduler can
/// be driven by a stub in tests. `None` = couldn't determine (→ no notice).
#[async_trait]
pub trait VersionSource: Send + Sync {
    async fn latest(
        &self,
        ecosystem: &str,
        name: &str,
        local_path: Option<&str>,
    ) -> Option<LatestInfo>;
}

/// The registry endpoint returning the latest version for `ecosystem`/`name`, or
/// `None` for an ecosystem v0 doesn't cover. Pure.
pub fn registry_latest_url(ecosystem: &str, name: &str) -> Option<String> {
    match ecosystem {
        "npm" => Some(format!("https://registry.npmjs.org/{name}/latest")),
        "cargo" => Some(format!("https://crates.io/api/v1/crates/{name}")),
        "pypi" => Some(format!("https://pypi.org/pypi/{name}/json")),
        "go" => Some(format!("https://proxy.golang.org/{}/@latest", escape_go_module(name))),
        _ => None,
    }
}

/// proxy.golang.org requires uppercase letters in a module path to be escaped as
/// `!` + lowercase (e.g. `github.com/BurntSushi` → `github.com/!burnt!sushi`); a
/// plain path 404s for any module with capitals.
fn escape_go_module(module: &str) -> String {
    let mut out = String::with_capacity(module.len());
    for c in module.chars() {
        if c.is_ascii_uppercase() {
            out.push('!');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Extract the latest version from a registry JSON body. Pure; `None` on a shape miss.
/// crates.io falls back from `max_stable_version` (null when every release is a
/// prerelease/yanked) to `max_version`.
pub fn extract_latest(ecosystem: &str, body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let s = match ecosystem {
        "npm" => v.get("version").and_then(|x| x.as_str()),
        "cargo" => v.get("crate").and_then(|c| {
            c.get("max_stable_version")
                .and_then(|x| x.as_str())
                .or_else(|| c.get("max_version").and_then(|x| x.as_str()))
        }),
        "pypi" => v.get("info").and_then(|i| i.get("version")).and_then(|x| x.as_str()),
        "go" => v.get("Version").and_then(|x| x.as_str()),
        _ => None,
    }?;
    let s = s.trim();
    if s.is_empty() { None } else { Some(s.to_string()) }
}

/// The URLs a registry response carries about a package (02b S8).
///
/// Every field is `Option` and a miss is `None` — NEVER a URL derived from the
/// package name. `github.com/<name>/<name>` is wrong far more often than it is
/// right, and a fabricated URL is worse than none because something will fetch
/// it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegistryUrls {
    /// Where the source lives. The input the github docs route needs.
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub docs: Option<String>,
}

impl RegistryUrls {
    pub fn is_empty(&self) -> bool {
        self.repository.is_none() && self.homepage.is_none() && self.docs.is_none()
    }
}

/// Unwrap a registry's PACKAGING of a URL: a leading `git+` and a trailing
/// `.git`.
///
/// npm returns `git+https://github.com/sveltejs/svelte.git` — measured, not
/// assumed. Those two affixes are packaging around an otherwise-valid URL, so
/// removing them is unwrapping, not rewriting. Everything else is left
/// verbatim: an `ssh://` remote stays `ssh://` rather than being "helpfully"
/// converted to https, which would be inventing a URL that may not serve.
///
/// npm's `github:user/repo` shorthand is NOT expanded here. It is unambiguous
/// and could be, but the github route that would consume it is not built yet
/// (S9); recording it verbatim keeps the evidence without guessing what to do
/// with it.
fn unwrap_scm_url(raw: &str) -> Option<String> {
    let s = raw.trim().trim_start_matches("git+");
    let s = s.strip_suffix(".git").unwrap_or(s);
    let s = s.trim();
    if s.is_empty() { None } else { Some(s.to_string()) }
}

fn non_empty(v: Option<&str>) -> Option<String> {
    let s = v?.trim();
    if s.is_empty() { None } else { Some(s.to_string()) }
}

/// Case-insensitive lookup over pypi's `project_urls`, which is a free-form map
/// whose keys authors capitalise however they like — measured on `requests`:
/// `{"Documentation": ..., "Source": ...}`, with `home_page` NULL.
fn project_url<'a>(map: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    let obj = map.as_object()?;
    for want in keys {
        for (k, v) in obj {
            if k.eq_ignore_ascii_case(want) {
                if let Some(s) = v.as_str() {
                    return Some(s);
                }
            }
        }
    }
    None
}

/// Extract the repository / homepage / docs URLs from a registry JSON body.
/// Pure; a shape miss yields `None` per field (02b S8).
///
/// **These responses are already being downloaded and thrown away.**
/// [`registry_latest_url`] fetches exactly these bodies for the update
/// scheduler and reads one field out of each. Measured before this: 2 of 1,121
/// `libraries` rows carried any URL at all, which is why the github route, the
/// website route's trigger and R11.2's upstreaming were all blocked — there
/// was nothing to file against for 1,119 of them.
///
/// Shapes are MEASURED against live responses, not assumed:
/// - npm    `repository` is an OBJECT `{url: "git+https://….git"}` (it may also
///          be a bare string), `homepage` a plain string, no docs field.
/// - cargo  `crate.repository` / `.homepage` / `.documentation`, clean URLs.
/// - pypi   `info.project_urls` — a MAP with author-chosen capitalisation.
///          `info.home_page` is deprecated and was NULL on `requests`, so
///          reading it alone would have silently returned nothing.
/// - go     the proxy's `@latest` returns `Version`/`Time` only. No URLs, and
///          saying so beats inventing one.
pub fn extract_urls(ecosystem: &str, body: &str) -> RegistryUrls {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return RegistryUrls::default();
    };
    match ecosystem {
        "npm" => {
            let repo = v.get("repository").and_then(|r| {
                // Object form `{type, url}` first, bare-string form second.
                r.get("url").and_then(|u| u.as_str()).or_else(|| r.as_str())
            });
            RegistryUrls {
                repository: repo.and_then(unwrap_scm_url),
                homepage: non_empty(v.get("homepage").and_then(|x| x.as_str())),
                docs: None,
            }
        }
        "cargo" => {
            let c = v.get("crate");
            RegistryUrls {
                repository: c
                    .and_then(|c| c.get("repository"))
                    .and_then(|x| x.as_str())
                    .and_then(unwrap_scm_url),
                homepage: non_empty(c.and_then(|c| c.get("homepage")).and_then(|x| x.as_str())),
                docs: non_empty(c.and_then(|c| c.get("documentation")).and_then(|x| x.as_str())),
            }
        }
        "pypi" => {
            let info = v.get("info");
            let urls = info.and_then(|i| i.get("project_urls"));
            let pick = |keys: &[&str]| urls.and_then(|m| project_url(m, keys)).map(str::to_string);
            RegistryUrls {
                repository: pick(&["Source", "Repository", "Source Code", "Code"])
                    .as_deref()
                    .and_then(unwrap_scm_url),
                homepage: pick(&["Homepage", "Home"]).or_else(|| {
                    non_empty(info.and_then(|i| i.get("home_page")).and_then(|x| x.as_str()))
                }),
                docs: pick(&["Documentation", "Docs"]),
            }
        }
        _ => RegistryUrls::default(),
    }
}

/// Read a local library's own version by parsing the first known manifest in its
/// source dir — reuses the `ManifestAdapter` parsers. `None` if no parseable manifest.
pub fn local_latest(local_path: &str) -> Option<String> {
    let dir = std::path::Path::new(local_path);
    for fname in crate::adapters::manifest::all_manifest_filenames() {
        let p = dir.join(fname);
        if let Ok(content) = std::fs::read_to_string(&p)
            && let Some(adapter) = crate::adapters::manifest::manifest_adapter_for_filename(fname)
            && let Some(ver) = adapter.parse_manifest(&content).version
        {
            return Some(ver);
        }
    }
    None
}

/// Production [`VersionSource`] — reads a local manifest when the library has a local
/// source, else hits the registry over HTTP (reusing the daemon's reqwest + UA).
pub struct HttpVersionSource;

#[async_trait]
impl VersionSource for HttpVersionSource {
    async fn latest(
        &self,
        ecosystem: &str,
        name: &str,
        local_path: Option<&str>,
    ) -> Option<LatestInfo> {
        // A local manifest gives a version and no URLs — it is the library's
        // own file, not the registry's record of it.
        if let Some(lp) = local_path
            && let Some(v) = local_latest(lp)
        {
            return Some(LatestInfo::version_only(v));
        }
        let url = registry_latest_url(ecosystem, name)?;
        let client = reqwest::Client::builder().build().ok()?;
        let resp = client.get(&url).header("User-Agent", "sensei-daemon").send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body = resp.text().await.ok()?;
        // ONE body, BOTH answers (S8). The URLs were being discarded here.
        Some(LatestInfo {
            version: extract_latest(ecosystem, &body)?,
            urls: extract_urls(ecosystem, &body),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_urls_per_ecosystem() {
        assert_eq!(
            registry_latest_url("npm", "svelte").unwrap(),
            "https://registry.npmjs.org/svelte/latest"
        );
        assert_eq!(
            registry_latest_url("cargo", "serde").unwrap(),
            "https://crates.io/api/v1/crates/serde"
        );
        assert_eq!(
            registry_latest_url("pypi", "requests").unwrap(),
            "https://pypi.org/pypi/requests/json"
        );
        assert_eq!(
            registry_latest_url("go", "github.com/BurntSushi/toml").unwrap(),
            "https://proxy.golang.org/github.com/!burnt!sushi/toml/@latest",
            "go module capitals are escaped"
        );
        assert!(registry_latest_url("maven", "x").is_none(), "deferred ecosystem → None");
        assert!(registry_latest_url("docs", "x").is_none());
    }

    /// The WHOLE S8 path against a live registry: fetch, extract, persist.
    ///
    /// `#[ignore]` — network + database. The unit tests cover the parse and the
    /// writer separately; this is the one that proves they are connected, which
    /// is the failure mode this plan keeps hitting (a capability built and
    /// never reached).
    ///
    /// `cargo test -p senseid --bin senseid s8_end_to_end -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn s8_end_to_end_a_real_lookup_persists_a_repository_url() {
        let pg = crate::db::pg_store::PgStore::connect_test().await.expect("connect");
        let lib = pg
            .upsert_library("_test:s8", "cargo", Some("1.0.0"), None, None, None)
            .await
            .expect("library");

        // The production source, no stub: one real request to crates.io.
        let info = HttpVersionSource
            .latest("cargo", "serde", None)
            .await
            .expect("crates.io returned nothing — network down, or the shape moved");
        assert!(!info.version.is_empty());
        assert!(info.urls.repository.is_some(), "the response carried no repository URL");

        pg.set_library_urls(&lib, &info.urls).await.expect("persist");

        let row: (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT repository_url, homepage_url FROM sensei.libraries WHERE id = $1",
        )
        .bind(lib)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        println!("persisted repository_url={:?} homepage_url={:?}", row.0, row.1);
        assert_eq!(row.0, info.urls.repository);

        pg.delete_library(&lib).await.ok();
    }

    /// Fetch the three registries FOR REAL and check the shapes still hold.
    ///
    /// `#[ignore]` because it needs network. The unit tests above assert
    /// against bodies copied from these exact endpoints, and a registry
    /// changing its response shape would leave those passing while production
    /// silently extracted nothing — this is what catches that.
    ///
    /// `cargo test -p senseid --bin senseid registry_shapes -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn registry_shapes_are_still_what_the_unit_tests_assume() {
        let client = reqwest::Client::new();
        for (eco, name) in [("npm", "svelte"), ("cargo", "serde"), ("pypi", "requests")] {
            let url = registry_latest_url(eco, name).unwrap();
            let body = client
                .get(&url)
                .header("User-Agent", "sensei-daemon")
                .send()
                .await
                .expect("fetch")
                .text()
                .await
                .expect("body");
            let u = extract_urls(eco, &body);
            println!("{eco:6} {name:10} repo={:?}", u.repository);
            println!("{:24} home={:?} docs={:?}", "", u.homepage, u.docs);
            assert!(u.repository.is_some(), "{eco}: no repository URL — the response shape moved");
            assert!(
                !u.repository.as_deref().unwrap().starts_with("git+"),
                "{eco}: packaging left unwrapped"
            );
            assert!(
                !u.repository.as_deref().unwrap().ends_with(".git"),
                "{eco}: packaging left unwrapped"
            );
        }
    }

    #[test]
    fn npm_repository_is_an_object_whose_url_is_git_wrapped() {
        // MEASURED against registry.npmjs.org/svelte/latest. Reading
        // `repository` as a string returns nothing; keeping the `git+` prefix
        // and `.git` suffix yields a URL nothing can fetch.
        let body = r#"{"version":"5.57.0",
            "repository":{"url":"git+https://github.com/sveltejs/svelte.git","type":"git"},
            "homepage":"https://svelte.dev"}"#;
        let u = extract_urls("npm", body);
        assert_eq!(u.repository.as_deref(), Some("https://github.com/sveltejs/svelte"));
        assert_eq!(u.homepage.as_deref(), Some("https://svelte.dev"));
        assert_eq!(u.docs, None, "npm has no documentation field");
    }

    #[test]
    fn npm_also_accepts_the_bare_string_repository_form() {
        let u = extract_urls("npm", r#"{"repository":"https://github.com/a/b.git"}"#);
        assert_eq!(u.repository.as_deref(), Some("https://github.com/a/b"));
    }

    #[test]
    fn an_ssh_remote_is_unwrapped_but_not_rewritten_to_https() {
        // Removing `git+`/`.git` is unwrapping packaging. Converting the scheme
        // would invent a URL that may not serve (R4).
        let u = extract_urls("npm", r#"{"repository":{"url":"git+ssh://git@github.com/a/b.git"}}"#);
        assert_eq!(u.repository.as_deref(), Some("ssh://git@github.com/a/b"));
    }

    #[test]
    fn cargo_carries_all_three_urls_unwrapped() {
        // MEASURED against crates.io/api/v1/crates/serde.
        let body = r#"{"crate":{"repository":"https://github.com/serde-rs/serde",
            "homepage":"https://serde.rs","documentation":"https://docs.rs/serde"}}"#;
        let u = extract_urls("cargo", body);
        assert_eq!(u.repository.as_deref(), Some("https://github.com/serde-rs/serde"));
        assert_eq!(u.homepage.as_deref(), Some("https://serde.rs"));
        assert_eq!(u.docs.as_deref(), Some("https://docs.rs/serde"));
    }

    #[test]
    fn pypi_reads_project_urls_because_home_page_is_null() {
        // MEASURED against pypi.org/pypi/requests/json: `home_page` is NULL and
        // the real URLs live in `project_urls` under author-chosen keys. An
        // implementation reading `home_page` returns nothing and looks correct.
        let body = r#"{"info":{"home_page":null,"project_urls":{
            "Documentation":"https://requests.readthedocs.io",
            "Source":"https://github.com/psf/requests"}}}"#;
        let u = extract_urls("pypi", body);
        assert_eq!(u.repository.as_deref(), Some("https://github.com/psf/requests"));
        assert_eq!(u.docs.as_deref(), Some("https://requests.readthedocs.io"));
        assert_eq!(u.homepage, None, "this package declares none — not invented");
    }

    #[test]
    fn pypi_project_url_keys_are_matched_case_insensitively() {
        // Authors capitalise these however they like; the map is free-form.
        let u = extract_urls(
            "pypi",
            r#"{"info":{"project_urls":{"source":"https://x/y","HOMEPAGE":"https://h"}}}"#,
        );
        assert_eq!(u.repository.as_deref(), Some("https://x/y"));
        assert_eq!(u.homepage.as_deref(), Some("https://h"));
    }

    #[test]
    fn go_and_unknown_ecosystems_yield_nothing_rather_than_a_guess() {
        // The go proxy's @latest returns Version/Time only. Saying so beats
        // deriving `github.com/<name>` from the module path.
        assert!(extract_urls("go", r#"{"Version":"v1.2.3","Time":"t"}"#).is_empty());
        assert!(extract_urls("maven", r#"{"anything":1}"#).is_empty());
    }

    #[test]
    fn a_shape_miss_or_bad_json_is_empty_never_partial_garbage() {
        assert!(extract_urls("npm", "not json").is_empty());
        assert!(extract_urls("npm", r#"{"nope":1}"#).is_empty());
        // An empty-string field is a miss, not a URL.
        assert!(extract_urls("cargo", r#"{"crate":{"repository":"   "}}"#).is_empty());
    }

    #[test]
    fn extract_latest_per_ecosystem_shape() {
        assert_eq!(
            extract_latest("npm", r#"{"version":"4.2.19","name":"svelte"}"#).as_deref(),
            Some("4.2.19")
        );
        assert_eq!(
            extract_latest("pypi", r#"{"info":{"version":"2.31.0"}}"#).as_deref(),
            Some("2.31.0")
        );
        assert_eq!(
            extract_latest("go", r#"{"Version":"v1.2.3","Time":"..."}"#).as_deref(),
            Some("v1.2.3")
        );
        // crates.io prefers max_stable_version, falls back to max_version.
        assert_eq!(
            extract_latest(
                "cargo",
                r#"{"crate":{"max_stable_version":"1.0.219","max_version":"1.0.220-beta"}}"#
            )
            .as_deref(),
            Some("1.0.219")
        );
        assert_eq!(
            extract_latest(
                "cargo",
                r#"{"crate":{"max_stable_version":null,"max_version":"0.1.0-rc1"}}"#
            )
            .as_deref(),
            Some("0.1.0-rc1"),
            "null max_stable_version falls back to max_version"
        );
        // Shape miss → None (fail-closed).
        assert!(extract_latest("npm", r#"{"nope":1}"#).is_none());
        assert!(extract_latest("cargo", "not json").is_none());
    }
}
