//! `DotnetManifestAdapter` — `.csproj` / `.fsproj` / `.sln` (#88).
//!
//! Two file shapes:
//! - `.csproj` / `.fsproj` are XML. Dependencies live in
//!   `<PackageReference Include="..." Version="..." />` self-closing
//!   elements — attributes, NOT text.
//! - `.sln` is a plain-text format (not XML) with lines like
//!   `Project("{GUID}") = "Name", "path\to\file.csproj", "{GUID}"`.
//!   These enumerate the projects in a solution.
//!
//! Ecosystem slug is `"nuget"` — that's what the NuGet feed calls itself,
//! and it's what the runtime sees when it resolves a package. Kept
//! distinct from the display name "dotnet" so future non-nuget .NET
//! package sources (e.g. Paket) can register as their own ecosystem.

use super::xml::{XmlEvent, XmlPath, attr, walk};
use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;
use crate::types::PackageInfo;
use std::path::Path;

pub struct DotnetManifestAdapter;

impl ManifestAdapter for DotnetManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &[]
    }

    fn manifest_extensions(&self) -> &[&'static str] {
        &["csproj", "fsproj", "sln"]
    }

    fn ecosystem(&self) -> &'static str {
        "nuget"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        // Only .csproj / .fsproj carry PackageReference; .sln doesn't. This
        // adapter can't tell WHICH file it's parsing from the content alone,
        // so it always tries the XML walk. If content is a .sln (not XML at
        // all), quick-xml returns an early Err on the header and we return
        // empty.
        let mut deps: Vec<DepVersion> = Vec::new();
        let _ = walk(content, |path: &XmlPath<'_>, evt: XmlEvent<'_>| {
            // <PackageReference> can nest under <ItemGroup> at various depths.
            let at_pkg_ref = path.0.last().map(|s| s == "PackageReference").unwrap_or(false);
            if !at_pkg_ref {
                return;
            }
            let XmlEvent::Enter(attrs) = evt else { return };
            let Some(name) = attr(attrs, "Include") else { return };
            // Version can be an attribute OR a child <Version> element. The
            // attribute form covers ~all real-world usage; the child form
            // (used with `PrivateAssets`) is a rare Central Package
            // Management style — surfaced as "*" here and left to the
            // follow-up that adds property interpolation.
            let version = attr(attrs, "Version").unwrap_or("*").to_string();
            deps.push(DepVersion {
                lib_name: name.to_string(),
                version: version.trim().to_string(),
                raw_version: version,
                source: "csproj".into(),
                dev: false, // .NET has no test/dev scope at the reference level.
                local_source: None,
            });
        });
        deps
    }

    fn is_workspace_root(&self, content: &str) -> bool {
        // .sln files are the .NET workspace root. Detect by the Microsoft
        // Visual Studio Solution File signature — that header is stable
        // across VS/MSBuild versions since 2005.
        content.contains("Microsoft Visual Studio Solution File")
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        // Identity comes from <AssemblyName> if present, else <RootNamespace>.
        // .csproj has no dedicated "name" tag — the file's basename is
        // conventionally the assembly name, but we don't have the filename
        // here.
        let mut name: Option<String> = None;
        let mut version: Option<String> = None;
        let mut description: Option<String> = None;
        let _ = walk(content, |path: &XmlPath<'_>, evt: XmlEvent<'_>| {
            let XmlEvent::Leaf(text) = evt else { return };
            if text.is_empty() {
                return;
            }
            // The exact PropertyGroup nesting depth varies (top-level or
            // per-config), so match by ends_with instead of exact path.
            // AssemblyName wins over RootNamespace when both are present;
            // whichever comes first in the file sets `name`.
            if (path.ends_with(&["AssemblyName"]) || path.ends_with(&["RootNamespace"]))
                && name.is_none()
            {
                name = Some(text.to_string());
            } else if path.ends_with(&["Version"]) && version.is_none() {
                // Guard against the PackageReference Version child — that's
                // 4 levels deep under an ItemGroup and shouldn't be the
                // project version. We only accept a Version leaf under a
                // PropertyGroup (2 levels above <Version>).
                let two_up = path.0.iter().rev().nth(1).map(String::as_str);
                if two_up != Some("PackageReference") {
                    version = Some(text.to_string());
                }
            } else if path.ends_with(&["Description"]) && description.is_none() {
                description = Some(text.to_string());
            }
        });
        ParsedManifest { name, version, description }
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["dotnet"]
    }

    fn detect_workspace_members(&self, repo_root: &Path) -> Vec<PackageInfo> {
        // Find the first .sln under repo_root (top-level only — nested
        // solutions live in their own reactors).
        let Ok(entries) = std::fs::read_dir(repo_root) else {
            return Vec::new();
        };
        let sln = entries.flatten().find_map(|e| {
            let p = e.path();
            let is_sln =
                p.extension().and_then(|x| x.to_str()).map(str::to_ascii_lowercase).as_deref()
                    == Some("sln");
            if is_sln { Some(p) } else { None }
        });
        let Some(sln_path) = sln else { return Vec::new() };
        let Ok(content) = std::fs::read_to_string(&sln_path) else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for line in content.lines() {
            let Some(project) = parse_sln_project_line(line) else { continue };
            // Normalise Windows-style backslashes so the join actually finds
            // the file on unix filesystems.
            let rel = project.path.replace('\\', "/");
            let child = repo_root.join(&rel);
            if !child.exists() {
                continue;
            }
            // Only include actual per-project files (skip Solution Folders,
            // which are containers with no project file on disk).
            let ext = child.extension().and_then(|x| x.to_str()).map(str::to_ascii_lowercase);
            if !matches!(ext.as_deref(), Some("csproj") | Some("fsproj") | Some("vbproj")) {
                continue;
            }
            let dir = child
                .parent()
                .and_then(|p| p.strip_prefix(repo_root).ok())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| project.name.clone());
            let pom = std::fs::read_to_string(&child).unwrap_or_default();
            let parsed = self.parse_manifest(&pom);
            out.push(PackageInfo {
                name: parsed.name.unwrap_or(project.name),
                path: dir,
                version: parsed.version,
                pkg_type: "dotnet_project".to_string(),
                private: false,
            });
        }
        out
    }

    /// Conventional dotnet CLI verbs. `test` / `build` / `run` / `publish`
    /// / `restore` cover the canonical loop.
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        super::conventional_commands(
            "dotnet",
            &[
                ("test", "test"),
                ("build", "build"),
                ("run", "run"),
                ("publish", "build"),
                ("restore", "run"),
            ],
        )
    }
}

/// One project entry parsed out of a `.sln` line. Solution Folders (which
/// have a well-known type GUID) are dropped by the extension check on the
/// caller side.
struct SlnProject {
    name: String,
    path: String,
}

/// `.sln` project lines have the shape:
///
/// ```text
/// Project("{TYPE-GUID}") = "Name", "relative\path\to\Name.csproj", "{PROJ-GUID}"
/// ```
///
/// Parse a single line into `(name, relative path)` when it matches, `None`
/// otherwise. Uses simple string scanning rather than a regex so a slightly
/// off line doesn't tank the whole enumeration.
fn parse_sln_project_line(line: &str) -> Option<SlnProject> {
    let trimmed = line.trim();
    if !trimmed.starts_with("Project(") {
        return None;
    }
    // Split the RHS by comma; entries are `"quoted"` strings with optional
    // leading whitespace. The first is the display name, the second is the
    // relative project path.
    let after_eq = trimmed.find('=').map(|i| &trimmed[i + 1..])?;
    let parts: Vec<&str> = after_eq.split(',').collect();
    if parts.len() < 2 {
        return None;
    }
    let name = strip_quotes(parts[0].trim())?;
    let path = strip_quotes(parts[1].trim())?;
    Some(SlnProject { name: name.to_string(), path: path.to_string() })
}

fn strip_quotes(s: &str) -> Option<&str> {
    let s = s.trim();
    let s = s.strip_prefix('"')?;
    s.strip_suffix('"')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_extensions() {
        let a = DotnetManifestAdapter;
        assert_eq!(a.ecosystem(), "nuget");
        assert!(a.manifest_filenames().is_empty());
        assert_eq!(a.manifest_extensions(), &["csproj", "fsproj", "sln"]);
    }

    #[test]
    fn accepts_extension_variants_case_insensitively() {
        let a = DotnetManifestAdapter;
        assert!(a.accepts("MyApp.csproj"));
        assert!(a.accepts("Solution.SLN"));
        assert!(a.accepts("Library.fsproj"));
        assert!(!a.accepts("package.json"));
        assert!(!a.accepts("plain.txt"));
    }

    #[test]
    fn parse_dependencies_reads_package_references() {
        let src = r#"<Project Sdk="Microsoft.NET.Sdk">
            <ItemGroup>
              <PackageReference Include="Newtonsoft.Json" Version="13.0.3" />
              <PackageReference Include="Serilog" Version="4.0.0" />
            </ItemGroup>
        </Project>"#;
        let deps = DotnetManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        let by = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert_eq!(by("Newtonsoft.Json").version, "13.0.3");
        assert_eq!(by("Serilog").version, "4.0.0");
    }

    #[test]
    fn parse_dependencies_defaults_version_star_when_missing() {
        let src = r#"<Project><ItemGroup>
            <PackageReference Include="Foo" />
        </ItemGroup></Project>"#;
        let deps = DotnetManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].version, "*");
    }

    #[test]
    fn parse_manifest_reads_assembly_name_and_description() {
        let src = r#"<Project>
            <PropertyGroup>
              <AssemblyName>MyApp</AssemblyName>
              <Version>1.4.2</Version>
              <Description>Sample .NET app.</Description>
            </PropertyGroup>
        </Project>"#;
        let p = DotnetManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("MyApp"));
        assert_eq!(p.version.as_deref(), Some("1.4.2"));
        assert_eq!(p.description.as_deref(), Some("Sample .NET app."));
    }

    #[test]
    fn parse_manifest_does_not_pick_package_reference_version_as_project_version() {
        // A PackageReference has a Version too — the identity extractor
        // must not confuse it with the project's own Version.
        let src = r#"<Project>
            <PropertyGroup>
              <AssemblyName>Lib</AssemblyName>
            </PropertyGroup>
            <ItemGroup>
              <PackageReference Include="X">
                <Version>9.9.9</Version>
              </PackageReference>
            </ItemGroup>
        </Project>"#;
        let p = DotnetManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("Lib"));
        assert!(p.version.is_none(), "project version must NOT come from PackageReference");
    }

    #[test]
    fn is_workspace_root_true_for_sln_header() {
        let sln = "Microsoft Visual Studio Solution File, Format Version 12.00\n";
        assert!(DotnetManifestAdapter.is_workspace_root(sln));
        assert!(!DotnetManifestAdapter.is_workspace_root("<Project></Project>"));
    }

    #[test]
    fn parse_sln_project_line_extracts_name_and_path() {
        let line = "Project(\"{9A19103F-16F7-4668-BE54-9A1E7A4F7556}\") = \"Core\", \"src\\Core\\Core.csproj\", \"{ABC}\"";
        let proj = parse_sln_project_line(line).unwrap();
        assert_eq!(proj.name, "Core");
        assert_eq!(proj.path, "src\\Core\\Core.csproj");
    }

    #[test]
    fn parse_sln_project_line_ignores_non_project_lines() {
        assert!(parse_sln_project_line("Microsoft Visual Studio Solution File").is_none());
        assert!(parse_sln_project_line("Global").is_none());
        assert!(parse_sln_project_line("").is_none());
    }

    #[test]
    fn detect_workspace_members_enumerates_sln_projects() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        // Solution file listing two csproj files (real Windows-style paths).
        std::fs::write(
            dir.path().join("App.sln"),
            "Microsoft Visual Studio Solution File, Format Version 12.00\n\
             Project(\"{9A19103F-16F7-4668-BE54-9A1E7A4F7556}\") = \"Core\", \"src\\Core\\Core.csproj\", \"{A1}\"\n\
             EndProject\n\
             Project(\"{9A19103F-16F7-4668-BE54-9A1E7A4F7556}\") = \"Api\",  \"src\\Api\\Api.csproj\",   \"{A2}\"\n\
             EndProject\n\
             Project(\"{2150E333-8FDC-42A3-9474-1A3956D46DE8}\") = \"Docs\", \"docs\", \"{A3}\"\n\
             EndProject\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src/Core")).unwrap();
        std::fs::write(
            dir.path().join("src/Core/Core.csproj"),
            r#"<Project><PropertyGroup><AssemblyName>Core</AssemblyName></PropertyGroup></Project>"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src/Api")).unwrap();
        std::fs::write(
            dir.path().join("src/Api/Api.csproj"),
            r#"<Project><PropertyGroup><AssemblyName>Api</AssemblyName></PropertyGroup></Project>"#,
        )
        .unwrap();

        let members = DotnetManifestAdapter.detect_workspace_members(dir.path());
        // Docs is a Solution Folder (no .csproj) → excluded; Core + Api remain.
        assert_eq!(members.len(), 2);
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"Core"));
        assert!(names.contains(&"Api"));
        assert!(members.iter().all(|m| m.pkg_type == "dotnet_project"));
    }
}
