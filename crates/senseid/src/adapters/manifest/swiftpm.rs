//! `SwiftPmManifestAdapter` — `Package.swift` (#89).
//!
//! Swift Package Manager manifests are Swift source code. Dependencies
//! declared via `.package(url: "...", ...)` or `.package(path: "...")`.
//! Extracting them exhaustively would need a Swift parser; the scanner
//! here handles the common declaration shapes and defers the rest.
//!
//! Not covered in v1:
//! - `.package(path: "...")` local-source deps.
//! - `dependencies:` inside a target — those reference the top-level
//!   package deps by name, not URL, so we'd need a two-pass resolution.

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;
use regex::Regex;
use std::sync::OnceLock;

pub struct SwiftPmManifestAdapter;

impl ManifestAdapter for SwiftPmManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["Package.swift"]
    }

    fn ecosystem(&self) -> &'static str {
        "swiftpm"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let stripped = strip_swift_comments(content);
        let mut out = Vec::new();
        for cap in package_url_re().captures_iter(&stripped) {
            let url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let name = package_name_from_url(url);
            if name.is_empty() {
                continue;
            }
            // Version tokens can be `.upToNextMajor("1.2.3")`,
            // `.exact("1.2.3")`, `from: "1.2.3"`, a range like
            // `"1.2.3"..<"2.0.0"`. Extract the first quoted string in
            // the trailing args.
            let after = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let version = version_re()
                .captures(after)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_else(|| "*".to_string());
            out.push(DepVersion {
                lib_name: name,
                version: version.clone(),
                raw_version: version,
                source: "Package.swift".into(),
                dev: false,
                local_source: None,
            });
        }
        out
    }

    fn is_workspace_root(&self, _content: &str) -> bool {
        false
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        // `Package(name: "Foo", ...)` — extract the first `name:` arg.
        static NAME: OnceLock<Regex> = OnceLock::new();
        let re =
            NAME.get_or_init(|| Regex::new(r#"Package\s*\([^)]*name\s*:\s*"([^"]+)""#).unwrap());
        let name = re.captures(content).map(|c| c.get(1).unwrap().as_str().to_string());
        ParsedManifest { name, version: None, description: None }
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["swift"]
    }

    /// Conventional Swift Package Manager verbs.
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        super::conventional_commands(
            "swift",
            &[("test", "test"), ("build", "build"), ("run", "run")],
        )
    }
}

fn package_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // `.package(url: "https://…", from: "1.2.3")` — capture the URL, then
    // the remainder of the args up to the closing paren so `parse_dependencies`
    // can pull out a version.
    RE.get_or_init(|| Regex::new(r#"\.package\s*\(\s*url\s*:\s*"([^"]+)"([^)]*)\)"#).unwrap())
}

fn version_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#""([^"]+)""#).unwrap())
}

fn strip_swift_comments(content: &str) -> String {
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    static LINE: OnceLock<Regex> = OnceLock::new();
    let block = BLOCK.get_or_init(|| Regex::new(r"(?s)/\*.*?\*/").unwrap());
    let no_block = block.replace_all(content, "");
    // Match `//` only when at line start or preceded by whitespace — so
    // that URL schemes (`https://`) don't get eaten as comments. The
    // preceding whitespace or line-start is preserved via the capture.
    let line = LINE.get_or_init(|| Regex::new(r"(?m)(^|[ \t])//[^\n]*").unwrap());
    line.replace_all(&no_block, "$1").into_owned()
}

/// Extract a package name from a git URL. `https://github.com/apple/swift-log`
/// → `apple/swift-log`. Falls back to the basename with `.git` stripped.
fn package_name_from_url(url: &str) -> String {
    let cleaned = url.trim_end_matches('/').trim_end_matches(".git");
    // Try to get the last two segments of the path so `github.com/vendor/pkg`
    // becomes `vendor/pkg` — matches Composer / rubygems-style names and
    // avoids collisions across forks.
    let after_scheme = cleaned.rsplit_once("://").map(|(_, r)| r).unwrap_or(cleaned);
    let parts: Vec<&str> = after_scheme.split('/').collect();
    if parts.len() >= 2 {
        // Drop the host; keep the last two segments.
        let last = parts.len();
        return format!("{}/{}", parts[last - 2], parts[last - 1]);
    }
    parts.last().copied().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        let a = SwiftPmManifestAdapter;
        assert_eq!(a.ecosystem(), "swiftpm");
        assert_eq!(a.manifest_filenames(), &["Package.swift"]);
    }

    #[test]
    fn parse_manifest_reads_package_name() {
        let src = r#"
            // swift-tools-version:5.9
            import PackageDescription
            let package = Package(
                name: "MyLib",
                products: [ .library(name: "MyLib", targets: ["MyLib"]) ]
            )
        "#;
        let p = SwiftPmManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("MyLib"));
    }

    #[test]
    fn parse_dependencies_reads_package_url_forms() {
        let src = r#"
            let package = Package(
                name: "MyApp",
                dependencies: [
                    .package(url: "https://github.com/apple/swift-log.git", from: "1.5.0"),
                    .package(url: "https://github.com/apple/swift-argument-parser", .upToNextMajor(from: "1.3.0")),
                ]
            )
        "#;
        let deps = SwiftPmManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        let by = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        let log = by("apple/swift-log");
        assert_eq!(log.version, "1.5.0");
        let args = by("apple/swift-argument-parser");
        assert_eq!(args.version, "1.3.0");
    }

    #[test]
    fn parse_dependencies_skips_commented_declarations() {
        let src = r#"
            let package = Package(
                dependencies: [
                    // .package(url: "https://github.com/hidden/one", from: "1.0.0"),
                    .package(url: "https://github.com/real/two", from: "2.0.0"),
                ]
            )
        "#;
        let deps = SwiftPmManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].lib_name, "real/two");
    }

    #[test]
    fn package_name_extraction_strips_dot_git_and_host() {
        assert_eq!(
            package_name_from_url("https://github.com/apple/swift-log.git"),
            "apple/swift-log"
        );
        // Trailing slash tolerated.
        assert_eq!(package_name_from_url("https://gitlab.com/vendor/pkg/"), "vendor/pkg");
    }
}
