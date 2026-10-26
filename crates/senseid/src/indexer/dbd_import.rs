//! The dbd importer (#251 step 4): a dbd-managed database's structure, read
//! off its whole design, as schema facts for the store.
//!
//! Not a file walk. dbd's `SchemaModel` v2 is built from every DDL file of a
//! design at once and carries no file attribution, so the importer works at the
//! level of a `design.yaml` and hands its facts to `persist::write_schema` —
//! the same writer a file walk's `FileFacts.schema` goes through.
//!
//! Identity is the SQL walker's, not a second spelling of it: a fact names its
//! object through `lang::sql::object_identity`, with the package the walker is
//! given for the same files — the `project.name` `design.yaml` states (see
//! `placement::package_named_by`). An importer that minted its own fqn would
//! describe a node the walk never wrote.
//!
//! Spec: `docs/spec/indexer/20-schema-entities.md`.

use std::path::Path;

use dbd_core::schema_model::{SchemaModel, TableNode};

use super::facts::{
    FkAction, Naming, SchemaColumn, SchemaFact, SchemaIndex, SchemaKind, SchemaRef, SchemaSource,
};

/// What one design yielded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    pub facts: Vec<SchemaFact>,
    /// Objects dbd describes that the store has no kind for — today only
    /// triggers — named so the omission is visible rather than silent.
    pub skipped: Vec<Skipped>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub schema: String,
    pub name: String,
    pub kind: String,
}

/// Load the design at `design_yaml` and import it. `package` is the name the
/// manifest states — the caller already holds it from placement.
///
/// Through dbd's `Workspace` (#190): a design with `modules:` is one design per
/// module, and `Design::from_config` refuses it; a design without is a workspace
/// of one, so one path imports both. Each module's model is built on its own —
/// its dependencies' entities are its externals, never its own facts — so no
/// object is imported twice.
///
/// The env label only filters seed-data files, which the schema model never
/// reads.
pub fn import_design(design_yaml: &Path, package: &str) -> Result<Imported, String> {
    let fail = |e: dbd_core::DbdError| format!("dbd import: {}: {e}", design_yaml.display());
    let workspace = dbd_core::design::Workspace::load(design_yaml, "dev", None).map_err(fail)?;
    let mut imported = Imported { facts: Vec::new(), skipped: Vec::new() };
    for module in workspace.modules() {
        let model = dbd_core::schema_model::build(module.design(), None).map_err(fail)?;
        let one = facts_of(&model, package)?;
        imported.facts.extend(one.facts);
        imported.skipped.extend(one.skipped);
    }
    Ok(imported)
}

/// The pure half: a built model, as schema facts.
///
/// An object kind or key action the store's vocabulary does not spell is an
/// ERROR, never a nearest guess: the vocabularies are closed on purpose, and a
/// new dbd spelling is a decision to make, not a value to default.
pub fn facts_of(model: &SchemaModel, package: &str) -> Result<Imported, String> {
    let mut facts = Vec::with_capacity(model.tables.len() + model.entities.len());
    for table in &model.tables {
        facts.push(table_fact(table, package)?);
    }

    let mut skipped = Vec::new();
    for entity in &model.entities {
        // dbd models triggers; `schema_object_kind` does not. Named, so the gap
        // is visible to whoever widens the vocabulary.
        if entity.kind == "trigger" {
            skipped.push(Skipped {
                schema: entity.schema.clone(),
                name: entity.name.clone(),
                kind: entity.kind.clone(),
            });
            continue;
        }
        facts.push(SchemaFact {
            fqn: identity(package, &entity.schema, &entity.name)?,
            kind: kind_of(&entity.kind, &entity.schema, &entity.name)?,
            db_schema: entity.schema.clone(),
            source: SchemaSource::Dbd,
            naming: Naming::Stated,
            note: entity.note.clone(),
            note_md: entity.note_md.clone(),
            columns: Vec::new(),
            indexes: Vec::new(),
            refs: Vec::new(),
        });
    }

    for r in &model.refs {
        let from = identity(package, &r.from.s, &r.from.t)?;
        let on_delete = match r.action.as_deref() {
            None => None,
            Some(a) => Some(FkAction::from_label(a).ok_or_else(|| {
                format!("dbd import: {}.{}: unknown key action {a:?}", r.from.s, r.from.t)
            })?),
        };
        let owner = facts.iter_mut().find(|f| f.fqn == from).ok_or_else(|| {
            format!(
                "dbd import: a key from {}.{}, which the model has no table for",
                r.from.s, r.from.t
            )
        })?;
        owner.refs.push(SchemaRef {
            from_column: r.from.c.clone(),
            to: identity(package, &r.to.s, &r.to.t)?,
            to_column: r.to.c.clone(),
            on_delete,
        });
    }
    Ok(Imported { facts, skipped })
}

fn table_fact(table: &TableNode, package: &str) -> Result<SchemaFact, String> {
    Ok(SchemaFact {
        fqn: identity(package, &table.schema, &table.name)?,
        kind: kind_of(&table.kind, &table.schema, &table.name)?,
        db_schema: table.schema.clone(),
        source: SchemaSource::Dbd,
        naming: Naming::Stated,
        note: table.note.clone(),
        note_md: table.note_md.clone(),
        columns: table
            .columns
            .iter()
            .map(|c| SchemaColumn {
                name: c.name.clone(),
                sql_type: c.ty.clone(),
                primary_key: c.pk,
                not_null: c.nn,
                unique_key: c.uq,
                foreign_key: c.fk,
                enum_typed: c.en,
                default_expr: c.def.clone(),
                note: c.note.clone(),
            })
            .collect(),
        indexes: table
            .indexes
            .iter()
            .map(|i| SchemaIndex {
                name: i.name.clone(),
                definition: i.def.clone(),
                unique: i.unique,
            })
            .collect(),
        refs: Vec::new(),
    })
}

fn kind_of(label: &str, schema: &str, name: &str) -> Result<SchemaKind, String> {
    SchemaKind::from_label(label)
        .ok_or_else(|| format!("dbd import: {schema}.{name}: unknown object kind {label:?}"))
}

fn identity(package: &str, schema: &str, name: &str) -> Result<super::facts::Fqn, String> {
    super::lang::sql::facts::object_identity(package, schema, name)
        .map_err(|e| format!("dbd import: {schema}.{name}: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dbd_core::schema_model::{Column, EntityNode, ProjectInfo, Ref, RefEnd};
    use std::collections::BTreeSet;

    fn repo_database() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database")
    }

    /// Every fqn the SQL walker mints for this repository's DDL, walked with the
    /// package the manifest states.
    fn walked_fqns(package: &str) -> BTreeSet<String> {
        let root = repo_database();
        let mut out = BTreeSet::new();
        let mut stack = vec![root.join("ddl")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("ddl dir reads") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let rel = path.strip_prefix(&root).unwrap().to_string_lossy().to_string();
                let text = std::fs::read_to_string(&path).expect("ddl reads");
                let source =
                    crate::indexer::lang::Source { package, module: &rel, path: &rel, text: &text };
                let facts = crate::indexer::lang::sql::facts::read(
                    dbd_core::parser::Dialect::PostgreSql,
                    &source,
                    &crate::indexer::lang::TypeHomes::unknown(),
                )
                .unwrap_or_else(|e| panic!("{rel} walks: {e:?}"));
                out.extend(facts.symbols.iter().map(|s| s.fqn.as_str().to_string()));
            }
        }
        out
    }

    /// **THE IMPORTER DESCRIBES NODES THE WALK WROTE.** Run over this
    /// repository's own `database/`: every fact's fqn is one the SQL walker
    /// mints for the same files, so the detail lands on the walked node — with
    /// its span — rather than on a stub beside it.
    ///
    /// Mutations that must break this: mint the fqn with the schema in the
    /// name; take the package from anywhere but the manifest; drop the
    /// views/routines (the count floor).
    #[test]
    fn every_imported_object_is_a_node_the_sql_walk_mints() {
        let imported = import_design(&repo_database().join("design.yaml"), "sensei")
            .expect("this repository's design imports");
        let walked = walked_fqns("sensei");
        let unmatched: Vec<&str> = imported
            .facts
            .iter()
            .map(|f| f.fqn.as_str())
            .filter(|fqn| !walked.contains(*fqn))
            .collect();
        assert_eq!(unmatched, Vec::<&str>::new(), "facts naming no walked node");
        let tables = imported.facts.iter().filter(|f| f.kind == SchemaKind::Table).count();
        let others = imported.facts.len() - tables;
        assert!(tables > 100 && others > 20, "{tables} tables, {others} views/routines");
    }

    /// #190: a WORKSPACE design (`modules:`) imports every module's tables. It
    /// was refused outright — `Design::from_config` takes one design — so a
    /// repository whose design.yaml declares modules imported nothing.
    #[test]
    fn a_workspace_design_imports_every_module() {
        let tmp = tempfile::tempdir().unwrap();
        let write = |rel: &str, body: &str| {
            let path = tmp.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        };
        write(
            "design.yaml",
            "project:\n  name: shop\n\nschemas:\n  - app\n\n\
             modules:\n  shared: {}\n  core:\n    depends_on: [shared]\n",
        );
        write(
            "shared/ddl/table/app/currencies.ddl",
            "set search_path to app;\n\ncreate table if not exists currencies (\n  code text primary key\n);\n",
        );
        write(
            "core/ddl/table/app/orders.ddl",
            "set search_path to app;\n\ncreate table if not exists orders (\n  id integer primary key\n\
             , currency text references currencies (code)\n);\n",
        );

        let imported =
            import_design(&tmp.path().join("design.yaml"), "shop").expect("a workspace imports");
        let mut tables: Vec<&str> = imported
            .facts
            .iter()
            .filter(|f| f.kind == SchemaKind::Table)
            .map(|f| f.fqn.as_str())
            .collect();
        tables.sort();
        assert_eq!(tables.len(), 2, "both modules' tables: {tables:?}");
        assert!(
            tables.iter().any(|t| t.contains("currencies"))
                && tables.iter().any(|t| t.contains("orders"))
        );
    }

    /// The detail is dbd's, field for field — checked against a table whose DDL
    /// is in this repository, so a reader can verify it by opening the file.
    #[test]
    fn a_table_carries_its_columns_indexes_and_keys() {
        let imported = import_design(&repo_database().join("design.yaml"), "sensei").unwrap();
        let fact = imported
            .facts
            .iter()
            .find(|f| f.fqn.as_str().contains("·schema_refs·"))
            .expect("schema_refs is imported");
        assert_eq!(fact.kind, SchemaKind::Table);
        assert_eq!(fact.db_schema, "sensei");
        assert_eq!((fact.source, fact.naming), (SchemaSource::Dbd, Naming::Stated));
        let names: Vec<&str> = fact.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            ["id", "from_node_id", "from_column", "to_node_id", "to_column", "on_delete"]
        );
        let id = &fact.columns[0];
        assert!(id.primary_key && id.sql_type == "bigserial", "{id:?}");
        assert!(fact.columns[1].foreign_key && fact.columns[1].not_null);
        assert!(fact.columns[5].enum_typed, "on_delete is the fk_action enum");
        let indexes: Vec<(Option<&str>, &str, bool)> = fact
            .indexes
            .iter()
            .map(|i| (i.name.as_deref(), i.definition.as_str(), i.unique))
            .collect();
        assert_eq!(
            indexes,
            [
                (Some("schema_refs_from_node_id_idx"), "(from_node_id)", false),
                (Some("schema_refs_to_node_id_idx"), "(to_node_id)", false)
            ],
            "the two indexes schema_refs.ddl creates"
        );
        assert_eq!(fact.refs.len(), 2, "{:?}", fact.refs);
        assert!(fact.refs.iter().all(|r| r.on_delete == Some(FkAction::Cascade)
            && r.to.as_str().contains("·schema_entities·")
            && r.to_column == "node_id"));
        assert!(fact.note.as_deref().is_some_and(|n| n.contains("column PAIR")));
    }

    fn model(tables: Vec<TableNode>, entities: Vec<EntityNode>, refs: Vec<Ref>) -> SchemaModel {
        SchemaModel {
            version: 2,
            project: ProjectInfo { name: "p".into(), db: "postgres".into(), note: None },
            schemas: vec![],
            tables,
            refs,
            stubs: vec![],
            entities,
            deps: vec![],
            history: vec![],
            enums: vec![],
            modules: vec![],
        }
    }

    fn table(name: &str) -> TableNode {
        TableNode {
            schema: "app".into(),
            name: name.into(),
            kind: "table".into(),
            note: None,
            note_md: None,
            columns: vec![Column {
                name: "id".into(),
                ty: "uuid".into(),
                pk: true,
                nn: true,
                en: false,
                def: None,
                fk: false,
                uq: false,
                identity: None,
                generated: None,
                note: None,
            }],
            indexes: vec![],
            checks: vec![],
            module: None,
        }
    }

    fn fk(action: Option<&str>) -> Ref {
        let end = |t: &str| RefEnd { s: "app".into(), t: t.into(), c: "id".into() };
        Ref { from: end("orders"), to: end("customers"), action: action.map(String::from) }
    }

    /// A trigger is named in `skipped`; it is neither a fabricated kind nor a
    /// silent omission.
    #[test]
    fn a_trigger_is_reported_skipped_and_a_view_is_kept() {
        let entity = |name: &str, kind: &str| EntityNode {
            schema: "app".into(),
            name: name.into(),
            kind: kind.into(),
            note: None,
            note_md: None,
            module: None,
        };
        let got = facts_of(
            &model(vec![], vec![entity("audit", "trigger"), entity("totals", "view")], vec![]),
            "p",
        )
        .unwrap();
        assert_eq!(
            got.skipped,
            vec![Skipped { schema: "app".into(), name: "audit".into(), kind: "trigger".into() }]
        );
        assert_eq!(got.facts.len(), 1);
        assert_eq!(got.facts[0].kind, SchemaKind::View);
        assert!(got.facts[0].columns.is_empty());
    }

    /// A key lands on its OWNING table, and an action dbd spells in a way the
    /// vocabulary does not know is an error — never a guessed action.
    #[test]
    fn a_key_lands_on_its_owner_and_an_unknown_action_is_an_error() {
        let both = || vec![table("orders"), table("customers")];
        let got = facts_of(&model(both(), vec![], vec![fk(Some("set_null"))]), "p").unwrap();
        let orders = got.facts.iter().find(|f| f.fqn.as_str().contains("·orders·")).unwrap();
        let customers = got.facts.iter().find(|f| f.fqn.as_str().contains("·customers·")).unwrap();
        assert_eq!(orders.refs.len(), 1);
        assert!(customers.refs.is_empty());
        assert_eq!(orders.refs[0].on_delete, Some(FkAction::SetNull));
        assert_eq!(orders.refs[0].to, customers.fqn);

        let none = facts_of(&model(both(), vec![], vec![fk(None)]), "p").unwrap();
        assert_eq!(none.facts.iter().find(|f| !f.refs.is_empty()).unwrap().refs[0].on_delete, None);

        let err = facts_of(&model(both(), vec![], vec![fk(Some("explode"))]), "p").unwrap_err();
        assert!(err.contains("explode"), "{err}");
        let mut odd = table("orders");
        odd.kind = "hypertable".into();
        assert!(facts_of(&model(vec![odd], vec![], vec![]), "p").is_err());
    }
}
