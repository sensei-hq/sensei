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

/// Resolves the latest published version of a library. A trait so the scheduler can
/// be driven by a stub in tests. `None` = couldn't determine (→ no notice).
#[async_trait]
pub trait VersionSource: Send + Sync {
    async fn latest(&self, ecosystem: &str, name: &str, local_path: Option<&str>)
    -> Option<String>;
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
    ) -> Option<String> {
        if let Some(lp) = local_path
            && let Some(v) = local_latest(lp)
        {
            return Some(v);
        }
        let url = registry_latest_url(ecosystem, name)?;
        let client = reqwest::Client::builder().build().ok()?;
        let resp = client.get(&url).header("User-Agent", "sensei-daemon").send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body = resp.text().await.ok()?;
        extract_latest(ecosystem, &body)
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
