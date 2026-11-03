//! `DbdManifestAdapter` — `design.yaml`.
//!
//! The manifest a dbd-managed database has. It names the project, and — unlike
//! every other manifest in this registry — it STATES THE DIALECT of the source
//! beneath it:
//!
//! ```yaml
//! project:
//!   name: sensei
//!   note: Database structure for sensei the app.
//! source:
//!   dialect: postgresql
//! ```
//!
//! # Why the stated dialect matters
//!
//! SQL is the one language here whose reader cannot be chosen from the file
//! extension: `.sql` and `.ddl` hold T-SQL, PostgreSQL, MySQL and SQLite, and
//! they need different readers. `lang::sql::Dialect::detect` scores markers out
//! of the text, which works but is inference. A manifest saying `postgresql`
//! is the source STATING it, and a statement outranks an inference — the same
//! order Java's `package` line outranks a directory.
//!
//! [`DbdManifestAdapter::stated_dialect`] is that reader. It answers `None`
//! when the manifest states nothing, rather than assuming: 4 of the 9
//! `design.yaml` files in the watched roots predate the `source:` key, and
//! filling one in from dbd's current Postgres-only-ness would be a fact about
//! the tool rather than about the project.
//!
//! # Dependencies are EXTENSIONS
//!
//! A schema's dependencies are the Postgres extensions it requires —
//! `uuid-ossp`, `vector`, `postgis`. They are declared in two shapes across the
//! corpus, `target.<engine>.extensions` and a top-level `extensions`, and both
//! are read.
//!
//! No version is recorded, because no `design.yaml` in the corpus carries one
//! for an extension or for the project. `*` is this registry's spelling for
//! "the manifest stated no constraint".

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;
use dbd_core::config::DesignConfig;

pub struct DbdManifestAdapter;

/// The design, read as dbd reads it (#190), or nothing. A `design.yaml` dbd
/// refuses — not YAML, no `project` block, the legacy `project: <name>` string
/// — states nothing to any reader below: no name, no dialect, no extensions.
fn design(content: &str) -> Option<DesignConfig> {
    dbd_core::config::parse(content).ok()
}

impl DbdManifestAdapter {
    /// The dialect this project's SQL is written in, when the manifest says.
    ///
    /// `None` means the manifest stated nothing — NOT that it is Postgres.
    /// dbd falls back to `postgresql` and says whether it did
    /// (`dialect_stated`, dbd#109); that fallback is a fact about dbd rather
    /// than about the project, and `lang::sql::Dialect::detect` already answers
    /// from the source for the manifests that predate the `source:` key.
    ///
    /// The string is returned rather than a `Dialect`, so this module does not
    /// depend on the indexer's language vocabulary — `Dialect::from_label`
    /// owns the mapping and is the one place a label becomes a dialect.
    pub fn stated_dialect(content: &str) -> Option<String> {
        let source = design(content)?.source;
        let dialect = source.dialect.trim();
        // An EMPTY value is not a statement.
        (source.dialect_stated && !dialect.is_empty()).then(|| dialect.to_string())
    }
}

impl ManifestAdapter for DbdManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["design.yaml"]
    }

    fn ecosystem(&self) -> &'static str {
        "dbd"
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        let Some(design) = design(content) else { return ParsedManifest::default() };
        ParsedManifest {
            name: Some(design.project.name).filter(|n| !n.trim().is_empty()),
            // NO VERSION. A `project.version` is the schema's migration number,
            // not a package version, and inventing `0.0.0` would be a value a
            // caller cannot tell from a version the project actually set.
            version: None,
            description: design.project.note,
        }
    }

    /// The Postgres EXTENSIONS the schema requires: `target.<engine>.extensions`,
    /// as dbd reads them, under whatever engine key (`postgres`, `supabase`).
    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let Some(design) = design(content) else { return Vec::new() };
        let mut names: Vec<String> = design
            .target
            .values()
            .flat_map(|t| t.extensions.iter().map(|e| e.name().to_string()))
            .filter(|n| !n.trim().is_empty())
            .collect();
        names.sort();
        names.dedup();
        names
            .into_iter()
            .map(|name| DepVersion {
                lib_name: name,
                // `*` is this registry's spelling for "no constraint stated",
                // and a `design.yaml` states none for an extension.
                version: "*".to_string(),
                raw_version: "*".to_string(),
                source: "design.yaml".into(),
                // dbd has no dev/test extension section.
                dev: false,
                local_source: None,
            })
            .collect()
    }

    /// A dbd project is ONE database. `schemas:` lists namespaces inside it,
    /// not independently-versioned packages, so reading it as a workspace
    /// would make every non-trivial schema a monorepo.
    fn is_workspace_root(&self, _content: &str) -> bool {
        false
    }

    /// The stated dialect, so a reader can see which SQL this tree is.
    ///
    /// Falls back to `sql` rather than to a dialect: a manifest that states
    /// nothing has said only that this is SQL, and naming a dialect it did not
    /// write would be the inference this whole module exists to avoid.
    fn stack_labels(&self, content: &str) -> Vec<&'static str> {
        match Self::stated_dialect(content).as_deref().map(str::to_ascii_lowercase).as_deref() {
            Some("postgres" | "postgresql" | "pg") => vec!["sql", "postgresql"],
            Some("mssql" | "sqlserver" | "tsql") => vec!["sql", "tsql"],
            Some("mysql" | "mariadb") => vec!["sql", "mysql"],
            Some("sqlite" | "sqlite3") => vec!["sql", "sqlite"],
            _ => vec!["sql"],
        }
    }

    /// dbd's own verbs. `apply` builds the database and `doctor` checks the
    /// project, which are the two the shared vocabulary has a word for.
    ///
    /// `reconcile`, `diff`, `import` and `export` are deliberately absent:
    /// each is a real dbd command with no honest mapping onto
    /// `test/build/lint/run/…`, and inventing one would put a wrong label on
    /// an action a user might run.
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        super::conventional_commands("dbd", &[("apply", "build"), ("doctor", "lint")])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real `design.yaml`, in the shape the corpus actually uses.
    const SENSEI: &str = "project:\n  \
                          name: sensei\n  \
                          note: Database structure for sensei the app.\n\
                          source:\n  \
                          dialect: postgresql\n\
                          target:\n  \
                          postgres:\n    \
                          url: $DATABASE_URL\n    \
                          extensions:\n      \
                          - name: uuid-ossp\n        \
                          schema: extensions\n      \
                          - name: vector\n        \
                          schema: extensions\n\
                          schemas:\n  \
                          - sensei\n  \
                          - activity\n";

    /// The OLDER shape: no `source:` key at all, extensions at the top level.
    /// 4 of the 9 manifests in the watched roots look like this.
    const OLD: &str = "project:\n  \
                       name: burn-e\n\
                       schemas:\n  \
                       - core\n\
                       extensions:\n  \
                       - name: postgis\n";

    /// The manifest NAMES the package, which is what placement needs.
    ///
    /// MUTATION: read `project` as a string rather than a mapping — the name
    /// is `None`, `package_named_by` keeps climbing, and every dbd DDL file is
    /// placed under whatever unrelated manifest sits above it.
    #[test]
    fn the_project_block_names_the_package() {
        let m = DbdManifestAdapter.parse_manifest(SENSEI);
        assert_eq!(m.name.as_deref(), Some("sensei"));
        assert_eq!(m.description.as_deref(), Some("Database structure for sensei the app."));
        // NO VERSION — no `design.yaml` in the corpus carries one.
        assert_eq!(m.version, None);

        assert_eq!(DbdManifestAdapter.parse_manifest(OLD).name.as_deref(), Some("burn-e"));
    }

    /// **THE MANIFEST STATES THE DIALECT, AND SILENCE IS NOT A STATEMENT.**
    ///
    /// MUTATION: default the absent case to `postgresql` — a manifest that
    /// predates the `source:` key starts asserting a dialect it never wrote,
    /// and the reader trusts it over the source text.
    #[test]
    fn the_dialect_is_read_when_stated_and_absent_when_not() {
        assert_eq!(DbdManifestAdapter::stated_dialect(SENSEI).as_deref(), Some("postgresql"));
        // The older shape states nothing, and nothing is the answer.
        assert_eq!(DbdManifestAdapter::stated_dialect(OLD), None);
        assert_eq!(DbdManifestAdapter::stated_dialect("project:\n  name: x\n"), None);
        // An EMPTY value is not a statement either.
        assert_eq!(DbdManifestAdapter::stated_dialect("source:\n  dialect: '  '\n"), None);
    }

    /// The label maps onto the indexer's own dialect vocabulary.
    ///
    /// Checked against `Dialect::from_label`, which is the one place a label
    /// becomes a dialect — so this pins that the two agree rather than
    /// restating the mapping.
    #[test]
    fn a_stated_dialect_is_one_the_indexer_knows() {
        use crate::indexer::lang::sql::Dialect;
        let stated = DbdManifestAdapter::stated_dialect(SENSEI).expect("sensei states one");
        assert_eq!(Dialect::from_label(&stated), Some(Dialect::PostgreSql));
    }

    /// Extensions are the dependencies, read as dbd reads `target.<engine>.extensions`
    /// — a bare name or `{name, schema}`, under whatever engine key (#190).
    ///
    /// MUTATION: read only `target.postgres.extensions` — torii declares its
    /// under `target.supabase`, and loses every dependency.
    #[test]
    fn extensions_are_the_dependencies_dbd_reads() {
        let deps = |content: &str| -> Vec<String> {
            DbdManifestAdapter.parse_dependencies(content).into_iter().map(|d| d.lib_name).collect()
        };
        assert_eq!(deps(SENSEI), ["uuid-ossp", "vector"]);

        // The ENGINE KEY VARIES, and a bare name is an extension too.
        let supabase = "project:\n  name: t\ntarget:\n  supabase:\n    extensions:\n      \
                        - vector\n      - name: postgis\n        schema: extensions\n";
        assert_eq!(deps(supabase), ["postgis", "vector"]);

        assert!(DbdManifestAdapter.parse_dependencies(SENSEI).iter().all(|d| !d.dev));
    }

    /// The legacy top-level `extensions:` is not read: dbd itself ignores it,
    /// and `dbd doctor --fix` moves it under the target. Reading it here would
    /// be a second, older reading of a file dbd owns (#190).
    #[test]
    fn the_legacy_top_level_extensions_are_dbds_to_migrate() {
        assert!(DbdManifestAdapter.parse_dependencies(OLD).is_empty());
    }

    /// The adapter reads `design.yaml` THROUGH dbd, keeping no copy of its
    /// struct to drift from it (#190).
    #[test]
    fn the_manifest_is_read_through_dbd() {
        let src = include_str!("dbd.rs");
        let production = &src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())];
        assert!(production.contains("dbd_core::config::parse"));
        assert!(!production.contains("serde_yaml::from_str"), "a second reading of design.yaml");
    }

    /// The stack says SQL, and names the dialect only when the manifest did.
    #[test]
    fn the_stack_names_the_dialect_only_when_it_was_stated() {
        assert_eq!(DbdManifestAdapter.stack_labels(SENSEI), vec!["sql", "postgresql"]);
        assert_eq!(DbdManifestAdapter.stack_labels(OLD), vec!["sql"]);
    }

    /// **A SCHEMA LIST IS NOT A WORKSPACE.**
    ///
    /// MUTATION: read `schemas:` as a workspace marker — every dbd project
    /// with more than one schema becomes a monorepo.
    #[test]
    fn a_schema_list_is_not_a_workspace() {
        assert!(!DbdManifestAdapter.is_workspace_root(SENSEI));
    }

    /// Malformed YAML states nothing, and says so rather than panicking.
    #[test]
    fn a_manifest_that_does_not_parse_states_nothing() {
        for broken in ["", "\t\tnot: [valid", "- a list, not a mapping\n", "%%%"] {
            assert_eq!(DbdManifestAdapter.parse_manifest(broken).name, None, "{broken:?}");
            assert_eq!(DbdManifestAdapter::stated_dialect(broken), None, "{broken:?}");
            assert!(DbdManifestAdapter.parse_dependencies(broken).is_empty(), "{broken:?}");
        }
    }
}
