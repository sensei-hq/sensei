//! The Schema diagram's payload (#251 step 5, for #221): the schema store's
//! rows assembled into the ER half of dbd's `SchemaModel` v2 — `tables`,
//! `refs`, `entities`, `deps` — which `@rokkit/graph`'s `toGraphInput(model,
//! 'er' | 'deps')` reads.
//!
//! The parts are dbd-core's own types, so the wire spelling is dbd's contract
//! and cannot drift from it. The WHOLE `SchemaModel` is not: its `workspace` and
//! `schemas` carry a dialect, a workspace note and per-schema enum counts the
//! store does not hold, and rokkit reads neither. They are left out rather
//! than filled with values nobody observed.

use std::collections::HashMap;

use dbd_core::schema_model::{Column, DepEdge, EntityNode, Index, NodeRef, Ref, RefEnd, TableNode};
use serde::Serialize;

use crate::db::pg_store::{StoredDep, StoredSchemaEntity};

/// The `SchemaModel` version these parts are shaped after.
pub const MODEL_VERSION: u32 = 2;

#[derive(Debug, PartialEq, Serialize)]
pub struct SchemaPayload {
    pub version: u32,
    pub tables: Vec<TableNode>,
    pub refs: Vec<Ref>,
    pub entities: Vec<EntityNode>,
    pub deps: Vec<DepEdge>,
}

/// Assemble the payload. An fqn that names no described object, or an
/// occurrence kind with no dbd dep kind, is an error — the store's foreign keys
/// and the read's filter both rule it out, so meeting one means they drifted.
pub fn assemble(
    entities: &[StoredSchemaEntity],
    deps: &[StoredDep],
) -> Result<SchemaPayload, String> {
    let named: HashMap<&str, (&str, &str)> = entities
        .iter()
        .map(|e| (e.fqn.as_str(), (e.db_schema.as_str(), e.name.as_str())))
        .collect();
    let object = |fqn: &str| {
        named
            .get(fqn)
            .copied()
            .ok_or_else(|| format!("schema model: {fqn} is not an object the store describes"))
    };

    let mut tables = Vec::new();
    let mut objects = Vec::new();
    let mut refs = Vec::new();
    for e in entities {
        if e.kind != "table" {
            objects.push(EntityNode {
                schema: e.db_schema.clone(),
                name: e.name.clone(),
                kind: e.kind.clone(),
                note: e.note.clone(),
                note_md: e.note_md.clone(),
                module: None,
            });
            continue;
        }
        tables.push(TableNode {
            schema: e.db_schema.clone(),
            name: e.name.clone(),
            kind: e.kind.clone(),
            note: e.note.clone(),
            note_md: e.note_md.clone(),
            columns: e
                .columns
                .iter()
                .map(|c| Column {
                    name: c.name.clone(),
                    ty: c.sql_type.clone(),
                    pk: c.primary_key,
                    nn: c.not_null,
                    en: c.enum_typed,
                    def: c.default_expr.clone(),
                    fk: c.foreign_key,
                    uq: c.unique_key,
                    identity: None,
                    generated: None,
                    note: c.note.clone(),
                })
                .collect(),
            indexes: e
                .indexes
                .iter()
                .map(|i| Index {
                    def: i.definition.clone(),
                    unique: i.unique,
                    name: i.name.clone(),
                    predicate: None,
                })
                .collect(),
            checks: Vec::new(),
            module: None,
        });
        for r in &e.refs {
            let (to_schema, to_table) = object(&r.to)?;
            refs.push(Ref {
                from: RefEnd {
                    s: e.db_schema.clone(),
                    t: e.name.clone(),
                    c: r.from_column.clone(),
                },
                to: RefEnd {
                    s: to_schema.to_string(),
                    t: to_table.to_string(),
                    c: r.to_column.clone(),
                },
                action: r.on_delete.clone(),
            });
        }
    }

    let mut edges = Vec::with_capacity(deps.len());
    for d in deps {
        let (from_schema, from_name) = object(&d.from)?;
        let kind = match d.kind.as_str() {
            "reads" | "writes" | "calls" => d.kind.clone(),
            "type_use" => "member".to_string(),
            other => {
                return Err(format!(
                    "schema model: {} has a {other:?} dependency, which dbd has no kind for",
                    d.from
                ));
            }
        };
        let (to, unresolved) = match (&d.to, &d.target_name) {
            (Some(fqn), _) => {
                let (s, n) = object(fqn)?;
                (NodeRef { s: s.to_string(), n: n.to_string() }, false)
            }
            // dbd's own reading of a written name: `schema.name`, or a bare
            // name with no schema stated.
            (None, Some(written)) => {
                let (s, n) = written.rsplit_once('.').unwrap_or(("", written.as_str()));
                (NodeRef { s: s.to_string(), n: n.to_string() }, true)
            }
            (None, None) => {
                return Err(format!("schema model: a dependency of {} names no target", d.from));
            }
        };
        edges.push(DepEdge {
            from: NodeRef { s: from_schema.to_string(), n: from_name.to_string() },
            to,
            kind,
            unresolved,
        });
    }

    Ok(SchemaPayload { version: MODEL_VERSION, tables, refs, entities: objects, deps: edges })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pg_store::StoredSchemaRef;
    use crate::indexer::facts::{SchemaColumn, SchemaIndex};

    fn entity(schema: &str, name: &str, kind: &str) -> StoredSchemaEntity {
        StoredSchemaEntity {
            fqn: format!("sql·p·{schema}·{name}·item"),
            name: name.to_string(),
            kind: kind.to_string(),
            db_schema: schema.to_string(),
            source: "dbd".to_string(),
            naming: "stated".to_string(),
            note: None,
            note_md: None,
            columns: vec![],
            indexes: vec![],
            refs: vec![],
        }
    }

    fn column(name: &str, pk: bool) -> SchemaColumn {
        SchemaColumn {
            name: name.into(),
            sql_type: "uuid".into(),
            primary_key: pk,
            not_null: true,
            unique_key: false,
            foreign_key: !pk,
            enum_typed: false,
            default_expr: Some("gen_random_uuid()".into()),
            note: Some(format!("the {name}")),
        }
    }

    fn corpus() -> Vec<StoredSchemaEntity> {
        let mut customers = entity("app", "customers", "table");
        customers.columns = vec![column("id", true)];
        let mut orders = entity("app", "orders", "table");
        orders.note_md = Some("**orders**".into());
        orders.columns = vec![column("id", true), column("customer_id", false)];
        orders.indexes = vec![SchemaIndex {
            name: Some("orders_customer_idx".into()),
            definition: "(customer_id)".into(),
            unique: false,
        }];
        orders.refs = vec![StoredSchemaRef {
            from_column: "customer_id".into(),
            to: customers.fqn.clone(),
            to_column: "id".into(),
            on_delete: Some("cascade".into()),
        }];
        let totals = entity("report", "order_totals", "view");
        vec![customers, orders, totals]
    }

    /// Every column field reaches dbd's spelling — `ty`, `pk`, `nn`, `def` —
    /// and a table keeps its columns in order; a view is an entity, not a
    /// table with no columns.
    ///
    /// Mutations that must break this: swap `nn`/`uq`; file a view as a table;
    /// drop `note_md`.
    #[test]
    fn tables_carry_dbds_column_spelling_and_views_are_entities() {
        let payload = assemble(&corpus(), &[]).unwrap();
        assert_eq!(payload.version, 2);
        let names: Vec<&str> = payload.tables.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["customers", "orders"]);
        let orders = &payload.tables[1];
        assert_eq!((orders.schema.as_str(), orders.kind.as_str()), ("app", "table"));
        assert_eq!(orders.note_md.as_deref(), Some("**orders**"));
        let fk = &orders.columns[1];
        assert_eq!(
            (fk.name.as_str(), fk.ty.as_str(), fk.pk, fk.nn, fk.fk, fk.uq, fk.en),
            ("customer_id", "uuid", false, true, true, false, false)
        );
        assert_eq!(fk.def.as_deref(), Some("gen_random_uuid()"));
        assert_eq!(fk.note.as_deref(), Some("the customer_id"));
        assert_eq!(orders.indexes.len(), 1);
        assert_eq!(orders.indexes[0].def, "(customer_id)");
        assert_eq!(payload.entities.len(), 1);
        assert_eq!(
            (payload.entities[0].schema.as_str(), payload.entities[0].kind.as_str()),
            ("report", "view")
        );
    }

    /// A key is drawn between `(schema, table, column)` ends, the target's read
    /// off the object its fqn names.
    ///
    /// Mutation that must break this: take the target's schema from the source.
    #[test]
    fn a_key_names_both_ends_by_schema_table_and_column() {
        let mut entities = corpus();
        // The referenced table in ANOTHER schema, so a source-schema shortcut fails.
        entities[0].db_schema = "crm".into();
        let payload = assemble(&entities, &[]).unwrap();
        assert_eq!(payload.refs.len(), 1);
        let r = &payload.refs[0];
        assert_eq!(
            (r.from.s.as_str(), r.from.t.as_str(), r.from.c.as_str()),
            ("app", "orders", "customer_id")
        );
        assert_eq!((r.to.s.as_str(), r.to.t.as_str(), r.to.c.as_str()), ("crm", "customers", "id"));
        assert_eq!(r.action.as_deref(), Some("cascade"));
    }

    /// A placed dependency names both objects; an unplaced one keeps the name
    /// it was written with and says it is unresolved — dimmed, never dropped.
    /// `type_use` is dbd's `member`.
    ///
    /// Mutations that must break this: drop unresolved deps; map type_use to
    /// reads.
    #[test]
    fn deps_name_both_ends_and_an_unplaced_one_is_marked_not_dropped() {
        let entities = corpus();
        let (orders, totals) = (entities[1].fqn.clone(), entities[2].fqn.clone());
        let deps = vec![
            StoredDep {
                from: totals.clone(),
                to: Some(orders),
                target_name: None,
                kind: "reads".into(),
            },
            StoredDep {
                from: totals.clone(),
                to: None,
                target_name: Some("audit.events".into()),
                kind: "type_use".into(),
            },
        ];
        let payload = assemble(&entities, &deps).unwrap();
        let shape: Vec<(String, String, String, String, String, bool)> = payload
            .deps
            .iter()
            .map(|d| {
                (
                    d.from.s.clone(),
                    d.from.n.clone(),
                    d.to.s.clone(),
                    d.to.n.clone(),
                    d.kind.clone(),
                    d.unresolved,
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                (
                    "report".into(),
                    "order_totals".into(),
                    "app".into(),
                    "orders".into(),
                    "reads".into(),
                    false
                ),
                (
                    "report".into(),
                    "order_totals".into(),
                    "audit".into(),
                    "events".into(),
                    "member".into(),
                    true
                ),
            ]
        );
    }

    /// Drift is an error, never a guessed row.
    #[test]
    fn an_unknown_fqn_or_kind_is_an_error() {
        let entities = corpus();
        let stray = StoredDep {
            from: "sql·p·x·gone·item".into(),
            to: None,
            target_name: Some("t".into()),
            kind: "reads".into(),
        };
        assert!(assemble(&entities, &[stray]).is_err());
        let odd = StoredDep {
            from: entities[2].fqn.clone(),
            to: Some(entities[1].fqn.clone()),
            target_name: None,
            kind: "constructs".into(),
        };
        assert!(assemble(&entities, &[odd]).is_err());
        let mut dangling = corpus();
        dangling[1].refs[0].to = "sql·p·app·nowhere·item".into();
        assert!(assemble(&dangling, &[]).is_err());
    }
}
