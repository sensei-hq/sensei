use std::path::Path;

/// Detect workspace members / sub-packages in a monorepo.
/// Returns `(name, relative_path, pkg_type)` for each discovered member.
///
/// Folds over every registered `ManifestAdapter` and asks each one to
/// enumerate its workspace members (npm reads `package.json workspaces` +
/// `pnpm-workspace.yaml`, Cargo reads `[workspace] members` + fallback dirs,
/// Go reads `go.work`). Adding a new ecosystem is one adapter impl, not a new
/// branch here.
pub fn detect_workspace_members(repo_path: &Path) -> Vec<crate::types::PackageInfo> {
    let mut members = Vec::new();
    for adapter in crate::adapters::manifest::registered_adapters() {
        members.extend(adapter.detect_workspace_members(repo_path));
    }
    members
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_npm_workspaces() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"monorepo","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("packages/ui")).unwrap();
        std::fs::write(
            dir.path().join("packages/ui/package.json"),
            r#"{"name":"@repo/ui","version":"1.0.0"}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("packages/api")).unwrap();
        std::fs::write(dir.path().join("packages/api/package.json"), r#"{"name":"@repo/api"}"#)
            .unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 2);
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"@repo/ui"));
        assert!(names.contains(&"@repo/api"));
        assert!(members.iter().all(|m| m.pkg_type == "npm_workspace"));
    }

    #[test]
    fn detect_cargo_workspace() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"crates/*\"]")
            .unwrap();
        std::fs::create_dir_all(dir.path().join("crates/core")).unwrap();
        std::fs::write(
            dir.path().join("crates/core/Cargo.toml"),
            "[package]\nname = \"my-core\"\nversion = \"0.1.0\"",
        )
        .unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "my-core");
        assert_eq!(members[0].pkg_type, "cargo_crate");
    }

    #[test]
    fn detect_pnpm_workspace() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'apps/*'\n  - 'packages/*'",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("apps/web")).unwrap();
        std::fs::write(dir.path().join("apps/web/package.json"), r#"{"name":"@test/web"}"#)
            .unwrap();
        std::fs::create_dir_all(dir.path().join("packages/shared")).unwrap();
        std::fs::write(
            dir.path().join("packages/shared/package.json"),
            r#"{"name":"@test/shared"}"#,
        )
        .unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 2);
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"@test/web"));
        assert!(names.contains(&"@test/shared"));
    }

    #[test]
    fn detect_go_work() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n\t./cmd\n\t./pkg\n)\n")
            .unwrap();
        std::fs::create_dir_all(dir.path().join("cmd")).unwrap();
        std::fs::write(dir.path().join("cmd/go.mod"), "module github.com/test/cmd\n\ngo 1.21")
            .unwrap();
        std::fs::create_dir_all(dir.path().join("pkg")).unwrap();
        std::fs::write(dir.path().join("pkg/go.mod"), "module github.com/test/pkg\n\ngo 1.21")
            .unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 2);
        assert!(members.iter().all(|m| m.pkg_type == "go_module"));
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"github.com/test/cmd"));
        assert!(names.contains(&"github.com/test/pkg"));
    }

    #[test]
    fn detect_go_work_single_line_use() {
        // A `go.work` with a single-line `use ./sub` form (no parens) still
        // registers the workspace member.
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse ./svc\n").unwrap();
        std::fs::create_dir_all(dir.path().join("svc")).unwrap();
        std::fs::write(dir.path().join("svc/go.mod"), "module github.com/test/svc\n").unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "github.com/test/svc");
        assert_eq!(members[0].pkg_type, "go_module");
    }

    #[test]
    fn detect_yarn_style_workspaces() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"yarn-mono","workspaces":{"packages":["packages/*"]}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("packages/lib")).unwrap();
        std::fs::write(dir.path().join("packages/lib/package.json"), r#"{"name":"@yarn/lib"}"#)
            .unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "@yarn/lib");
    }

    #[test]
    fn captures_private_npm_members_and_publishes_the_rest() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"mono","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("packages/pub")).unwrap();
        std::fs::write(
            dir.path().join("packages/pub/package.json"),
            r#"{"name":"@m/pub","version":"1.2.3"}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("packages/secret")).unwrap();
        std::fs::write(
            dir.path().join("packages/secret/package.json"),
            r#"{"name":"@m/secret","private":true}"#,
        )
        .unwrap();
        let members = detect_workspace_members(dir.path());
        let pubm = members.iter().find(|m| m.name == "@m/pub").expect("public member found");
        let secret = members.iter().find(|m| m.name == "@m/secret").expect("private member found");
        assert!(!pubm.private, "a publishable package is public");
        assert_eq!(pubm.version.as_deref(), Some("1.2.3"));
        assert!(secret.private, "private:true package is marked private");
    }

    #[test]
    fn cargo_publish_false_marks_member_private() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"crates/*\"]")
            .unwrap();
        std::fs::create_dir_all(dir.path().join("crates/internal")).unwrap();
        std::fs::write(
            dir.path().join("crates/internal/Cargo.toml"),
            "[package]\nname = \"internal\"\nversion = \"0.1.0\"\npublish = false",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("crates/public")).unwrap();
        std::fs::write(
            dir.path().join("crates/public/Cargo.toml"),
            "[package]\nname = \"public\"\nversion = \"0.1.0\"",
        )
        .unwrap();
        let members = detect_workspace_members(dir.path());
        assert!(
            members.iter().find(|m| m.name == "internal").unwrap().private,
            "publish=false ⇒ private"
        );
        assert!(
            !members.iter().find(|m| m.name == "public").unwrap().private,
            "no publish key ⇒ public"
        );
    }

    #[test]
    fn no_workspace_returns_empty() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"simple-app"}"#).unwrap();
        let members = detect_workspace_members(dir.path());
        assert!(members.is_empty());
    }

    #[test]
    fn cargo_fallback_finds_standalone_crates_without_root_workspace() {
        // A repo where `crates/*` holds Rust packages but the top-level
        // `Cargo.toml` is either missing or not a workspace — the fallback
        // scan must still register each package.
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("crates/a")).unwrap();
        std::fs::write(
            dir.path().join("crates/a/Cargo.toml"),
            "[package]\nname = \"crate-a\"\nversion = \"0.1.0\"",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("crates/b")).unwrap();
        std::fs::write(dir.path().join("crates/b/Cargo.toml"), "[package]\nname = \"crate-b\"")
            .unwrap();
        let members = detect_workspace_members(dir.path());
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"crate-a"));
        assert!(names.contains(&"crate-b"));
    }
}
