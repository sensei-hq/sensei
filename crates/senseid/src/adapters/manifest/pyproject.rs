//! `PyprojectManifestAdapter` — parses `pyproject.toml`.
//!
//! Delegates dep parsing to `indexer::lib_indexer::parse_pyproject_deps`
//! (PEP 621 `[project.dependencies]` for now). Workspace detection recognises
//! uv's `[tool.uv.workspace]` — the closest thing to npm/cargo workspaces in
//! the Python ecosystem. Poetry's `[tool.poetry.packages]` describes packaging
//! structure, not a workspace root, so it does not qualify.
//!
//! Follow-ups (issue #89): `[tool.poetry.dependencies]` + `[tool.uv.sources]`
//! local-source parsing land in a later chunk once the trait shape is stable.

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::{DepVersion, parse_pyproject_deps};

pub struct PyprojectManifestAdapter;

impl ManifestAdapter for PyprojectManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["pyproject.toml"]
    }

    fn ecosystem(&self) -> &'static str {
        "pypi"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let Ok(pyp) = content.parse::<toml::Value>() else {
            return Vec::new();
        };
        parse_pyproject_deps(&pyp)
    }

    fn is_workspace_root(&self, content: &str) -> bool {
        content
            .parse::<toml::Value>()
            .ok()
            .and_then(|v| {
                v.get("tool").and_then(|t| t.get("uv")).and_then(|u| u.get("workspace")).cloned()
            })
            .is_some()
    }

    /// PEP 621's `[project]` first, then Poetry's `[tool.poetry]`.
    ///
    /// BOTH, because reading only the first left a large share of the ecosystem
    /// nameless. Poetry predates PEP 621 and states the same three fields in its
    /// own table; a project that has migrated carries `[project]` and often
    /// leaves the old table behind, so PEP 621 wins where both are present.
    ///
    /// MEASURED: a 103-file Poetry checkout produced no package name for any
    /// file. The package is the SECOND SEGMENT of every fqn a file declares, so
    /// that is not a missing label — it is 103 files with no identity.
    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        let Ok(pyp) = content.parse::<toml::Value>() else {
            return ParsedManifest::default();
        };
        let pep_621 = pyp.get("project");
        let poetry = pyp.get("tool").and_then(|t| t.get("poetry"));
        // The tables are consulted per FIELD rather than picked once, so a
        // manifest that states its name in one and its description in the other
        // yields both instead of whichever table won.
        let field = |key: &str| {
            pep_621
                .and_then(|t| t.get(key))
                .or_else(|| poetry.and_then(|t| t.get(key)))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        ParsedManifest {
            name: field("name"),
            version: field("version"),
            description: field("description"),
        }
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["python"]
    }

    /// Read command entries from the four common script sections. Multiple
    /// tools coexist in the wild — pdm, poetry, and hatch all define script
    /// runners inside pyproject.toml and each uses a distinct table:
    ///
    /// - `[tool.pdm.scripts]`   — pdm run <name>
    /// - `[tool.poetry.scripts]`— poetry run <name> (or the entry-point name directly)
    /// - `[project.scripts]`    — PEP 621 entry points
    /// - `[tool.hatch.envs.default.scripts]` — hatch run <name>
    ///
    /// Each entry becomes one [`super::DiscoveredCommand`]. Category is
    /// derived from the script name via the shared classifier.
    fn parse_commands(&self, content: &str) -> Vec<super::DiscoveredCommand> {
        let Ok(pyp) = content.parse::<toml::Value>() else { return Vec::new() };
        let mut out = Vec::new();
        let mut emit = |section: Option<&toml::Value>, runner: &str| {
            let Some(scripts) = section.and_then(|v| v.as_table()) else { return };
            for name in scripts.keys() {
                if name.is_empty() {
                    continue;
                }
                out.push(super::DiscoveredCommand {
                    raw_name: name.clone(),
                    command_line: format!("{runner} {name}"),
                    category: super::command_category::categorise(name),
                });
            }
        };
        emit(pyp.get("tool").and_then(|t| t.get("pdm")).and_then(|p| p.get("scripts")), "pdm run");
        emit(
            pyp.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("scripts")),
            "poetry run",
        );
        emit(pyp.get("project").and_then(|p| p.get("scripts")), "python -m");
        emit(
            pyp.get("tool")
                .and_then(|t| t.get("hatch"))
                .and_then(|h| h.get("envs"))
                .and_then(|e| e.get("default"))
                .and_then(|d| d.get("scripts")),
            "hatch run",
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        assert_eq!(PyprojectManifestAdapter.ecosystem(), "pypi");
        assert_eq!(PyprojectManifestAdapter.manifest_filenames(), &["pyproject.toml"]);
    }

    #[test]
    fn parse_dependencies_reads_pep621_project_dependencies() {
        let src = r#"
            [project]
            name = "example"
            dependencies = [
                "requests>=2.28",
                "flask",
            ]
        "#;
        let deps = PyprojectManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].lib_name, "requests");
        assert_eq!(deps[1].lib_name, "flask");
    }

    #[test]
    fn parse_dependencies_empty_for_invalid_toml() {
        assert!(PyprojectManifestAdapter.parse_dependencies("not toml [").is_empty());
    }

    #[test]
    fn is_workspace_root_true_for_uv_workspace_section() {
        let src = r#"
            [tool.uv.workspace]
            members = ["packages/*"]
        "#;
        assert!(PyprojectManifestAdapter.is_workspace_root(src));
    }

    #[test]
    fn is_workspace_root_false_for_project_only_manifest() {
        let src = r#"
            [project]
            name = "x"
            version = "1.0"
        "#;
        assert!(!PyprojectManifestAdapter.is_workspace_root(src));
    }

    #[test]
    fn is_workspace_root_false_for_poetry_packages_section() {
        // Poetry's [tool.poetry.packages] describes packaging structure, not
        // a workspace — don't misclassify.
        let src = r#"
            [tool.poetry]
            name = "x"
            [tool.poetry.packages]
            [[tool.poetry.packages]]
            include = "x"
        "#;
        assert!(!PyprojectManifestAdapter.is_workspace_root(src));
    }

    /// **POETRY STATES ITS METADATA IN `[tool.poetry]`, NOT `[project]`.**
    ///
    /// MEASURED: reading only PEP 621 left every file of a 103-file Poetry
    /// checkout with no package name at all, which means no identity — the
    /// package is the second segment of every fqn a file declares. Poetry
    /// predates PEP 621 and a large share of the ecosystem still ships this
    /// shape, so it is the common case rather than a legacy corner.
    #[test]
    fn a_poetry_manifest_states_its_name_where_poetry_puts_it() {
        let poetry = "[tool.poetry]\nname = \"ai-hedge-fund\"\nversion = \"1.0.0\"\n\
                      description = \"an example\"\n";
        let parsed = PyprojectManifestAdapter.parse_manifest(poetry);
        assert_eq!(parsed.name.as_deref(), Some("ai-hedge-fund"));
        assert_eq!(parsed.version.as_deref(), Some("1.0.0"));
        assert_eq!(parsed.description.as_deref(), Some("an example"));
    }

    /// PEP 621 WINS when a manifest carries both, because a project that has
    /// migrated states the current answer in `[project]` and leaves the old
    /// table behind for tooling that has not caught up.
    #[test]
    fn pep_621_wins_over_poetry_when_a_manifest_carries_both() {
        let both = "[project]\nname = \"new-name\"\n\n[tool.poetry]\nname = \"old-name\"\n";
        assert_eq!(PyprojectManifestAdapter.parse_manifest(both).name.as_deref(), Some("new-name"));
    }

    /// A manifest with neither table names nothing — and must not be given a
    /// fabricated name, which would file every file under a package no
    /// dependency edge spells.
    #[test]
    fn a_manifest_with_neither_table_names_nothing() {
        let neither = "[build-system]\nrequires = [\"setuptools\"]\n";
        assert_eq!(PyprojectManifestAdapter.parse_manifest(neither).name, None);
    }

    #[test]
    fn parse_manifest_extracts_project_metadata() {
        let src = r#"
            [project]
            name = "sensei-py"
            version = "0.1.0"
            description = "A Python helper"
        "#;
        let p = PyprojectManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("sensei-py"));
        assert_eq!(p.version.as_deref(), Some("0.1.0"));
        assert_eq!(p.description.as_deref(), Some("A Python helper"));
    }

    #[test]
    fn stack_labels_always_python() {
        assert_eq!(PyprojectManifestAdapter.stack_labels(""), vec!["python"]);
    }
}
