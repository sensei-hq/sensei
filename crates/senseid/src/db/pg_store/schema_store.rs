//! The schema store (#251): columns, indexes and column-level foreign keys of
//! the database objects the graph holds as nodes.
//!
//! The rules for what lands — which producer, which node, what replaces what —
//! live in `crate::indexer::persist::write_schema`; this file is the SQL.
//! Spec: `docs/spec/indexer/20-schema-entities.md`.

use super::*;
use crate::indexer::facts::{SchemaColumn, SchemaFact, SchemaIndex};

/// A foreign key whose referenced object has no `schema_entities` row in the
/// folder, so there is nothing for `schema_refs.to_node_id` to point at.
///
/// Returned rather than dropped: the line an ER diagram would draw is missing,
/// and a caller that cannot see that has lost a fact without knowing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnplacedRef {
    pub from: String,
    pub from_column: String,
    pub to: String,
}

/// One object's detail as the store holds it. Labels raw, as `indexer.rs`'s
/// reads are: decoding them is `persist`'s job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSchemaEntity {
    pub fqn: String,
    pub kind: String,
    pub db_schema: String,
    pub source: String,
    pub naming: String,
    pub note: Option<String>,
    pub note_md: Option<String>,
    pub columns: Vec<SchemaColumn>,
    pub indexes: Vec<SchemaIndex>,
    pub refs: Vec<StoredSchemaRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSchemaRef {
    pub from_column: String,
    pub to: String,
    pub to_column: String,
    pub on_delete: Option<String>,
}

type ColumnTuple =
    (uuid::Uuid, String, String, bool, bool, bool, bool, bool, Option<String>, Option<String>);

impl PgStore {
    /// Replace the detail of every object in `objects` — each paired with the
    /// node it describes — in ONE transaction.
    ///
    /// Replace, not merge: an object's columns, indexes and outgoing refs are
    /// deleted and rewritten, so a column dropped from the DDL disappears. Refs
    /// go last, after every object of the batch has its entity row, so a key
    /// between two objects of one batch never depends on their order.
    pub async fn replace_schema_detail(
        &self,
        folder_id: &uuid::Uuid,
        objects: &[(uuid::Uuid, &SchemaFact)],
    ) -> Result<Vec<UnplacedRef>, String> {
        let mut tx = self.pool.begin().await.map_err(|e| format!("schema store: begin: {e}"))?;

        for (node, fact) in objects {
            let at = fact.fqn.as_str();
            sqlx_core::query::query(
                "INSERT INTO sensei.schema_entities
                        (node_id, kind, db_schema, source, naming, note, note_md)
                 VALUES ($1, $2::sensei.schema_object_kind, $3, $4::sensei.schema_source,
                         $5::sensei.schema_naming, $6, $7)
                 ON CONFLICT (node_id) DO UPDATE
                    SET kind = EXCLUDED.kind, db_schema = EXCLUDED.db_schema,
                        source = EXCLUDED.source, naming = EXCLUDED.naming,
                        note = EXCLUDED.note, note_md = EXCLUDED.note_md,
                        modified_at = now()",
            )
            .bind(node)
            .bind(fact.kind.as_label())
            .bind(&fact.db_schema)
            .bind(fact.source.as_label())
            .bind(fact.naming.as_label())
            .bind(&fact.note)
            .bind(&fact.note_md)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("schema store: entity {at}: {e}"))?;

            for table in ["schema_columns", "schema_indexes", "schema_refs"] {
                let key = if table == "schema_refs" { "from_node_id" } else { "node_id" };
                sqlx_core::query::query(&format!("DELETE FROM sensei.{table} WHERE {key} = $1"))
                    .bind(node)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("schema store: clear {table} of {at}: {e}"))?;
            }

            for (ordinal, c) in fact.columns.iter().enumerate() {
                let ordinal = i32::try_from(ordinal + 1).unwrap_or(i32::MAX);
                sqlx_core::query::query(
                    "INSERT INTO sensei.schema_columns
                            (node_id, ordinal, name, sql_type, primary_key, not_null,
                             unique_key, foreign_key, enum_typed, default_expr, note)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                )
                .bind(node)
                .bind(ordinal)
                .bind(&c.name)
                .bind(&c.sql_type)
                .bind(c.primary_key)
                .bind(c.not_null)
                .bind(c.unique_key)
                .bind(c.foreign_key)
                .bind(c.enum_typed)
                .bind(&c.default_expr)
                .bind(&c.note)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("schema store: column {at}.{}: {e}", c.name))?;
            }

            for index in &fact.indexes {
                sqlx_core::query::query(
                    "INSERT INTO sensei.schema_indexes (node_id, name, definition, unique_index)
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(node)
                .bind(&index.name)
                .bind(&index.definition)
                .bind(index.unique)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("schema store: index on {at}: {e}"))?;
            }
        }

        let mut unplaced = Vec::new();
        for (node, fact) in objects {
            for r in &fact.refs {
                // The target is placed HERE, against the store, and not looked up
                // beforehand: an object written earlier in this transaction is
                // visible to this statement and to nothing outside it.
                let placed: Option<(i64,)> = sqlx_core::query_as::query_as(
                    "INSERT INTO sensei.schema_refs
                            (from_node_id, from_column, to_node_id, to_column, on_delete)
                     SELECT $1, $2, se.node_id, $4, $5::sensei.fk_action
                       FROM sensei.nodes n
                       JOIN sensei.schema_entities se ON se.node_id = n.id
                      WHERE n.folder_id = $6 AND n.fqn = $3
                     RETURNING id",
                )
                .bind(node)
                .bind(&r.from_column)
                .bind(r.to.as_str())
                .bind(&r.to_column)
                .bind(r.on_delete.map(|a| a.as_label()))
                .bind(folder_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| {
                    format!("schema store: ref {}.{}: {e}", fact.fqn.as_str(), r.from_column)
                })?;
                if placed.is_none() {
                    unplaced.push(UnplacedRef {
                        from: fact.fqn.as_str().to_string(),
                        from_column: r.from_column.clone(),
                        to: r.to.as_str().to_string(),
                    });
                }
            }
        }

        tx.commit().await.map_err(|e| format!("schema store: commit: {e}"))?;
        Ok(unplaced)
    }

    /// Every object the store describes in `folder_id`, with its columns in
    /// declaration order and its indexes and refs in write order.
    pub async fn schema_detail(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<StoredSchemaEntity>, String> {
        type EntityTuple =
            (uuid::Uuid, String, String, String, String, String, Option<String>, Option<String>);
        let entities: Vec<EntityTuple> = sqlx_core::query_as::query_as(
            "SELECT se.node_id, n.fqn, se.kind::text, se.db_schema, se.source::text,
                    se.naming::text, se.note, se.note_md
               FROM sensei.schema_entities se
               JOIN sensei.nodes n ON n.id = se.node_id
              WHERE n.folder_id = $1
              ORDER BY n.fqn",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("schema store: read entities: {e}"))?;

        let columns: Vec<ColumnTuple> = sqlx_core::query_as::query_as(
            "SELECT c.node_id, c.name, c.sql_type, c.primary_key, c.not_null, c.unique_key,
                    c.foreign_key, c.enum_typed, c.default_expr, c.note
               FROM sensei.schema_columns c
               JOIN sensei.nodes n ON n.id = c.node_id
              WHERE n.folder_id = $1
              ORDER BY c.node_id, c.ordinal",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("schema store: read columns: {e}"))?;

        let indexes: Vec<(uuid::Uuid, Option<String>, String, bool)> =
            sqlx_core::query_as::query_as(
                "SELECT i.node_id, i.name, i.definition, i.unique_index
                   FROM sensei.schema_indexes i
                   JOIN sensei.nodes n ON n.id = i.node_id
                  WHERE n.folder_id = $1
                  ORDER BY i.id",
            )
            .bind(folder_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("schema store: read indexes: {e}"))?;

        let refs: Vec<(uuid::Uuid, String, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT r.from_node_id, r.from_column, t.fqn, r.to_column, r.on_delete::text
                   FROM sensei.schema_refs r
                   JOIN sensei.nodes n ON n.id = r.from_node_id
                   JOIN sensei.nodes t ON t.id = r.to_node_id
                  WHERE n.folder_id = $1
                  ORDER BY r.id",
            )
            .bind(folder_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("schema store: read refs: {e}"))?;

        Ok(entities
            .into_iter()
            .map(|(node, fqn, kind, db_schema, source, naming, note, note_md)| StoredSchemaEntity {
                fqn,
                kind,
                db_schema,
                source,
                naming,
                note,
                note_md,
                columns: columns
                    .iter()
                    .filter(|c| c.0 == node)
                    .map(|c| SchemaColumn {
                        name: c.1.clone(),
                        sql_type: c.2.clone(),
                        primary_key: c.3,
                        not_null: c.4,
                        unique_key: c.5,
                        foreign_key: c.6,
                        enum_typed: c.7,
                        default_expr: c.8.clone(),
                        note: c.9.clone(),
                    })
                    .collect(),
                indexes: indexes
                    .iter()
                    .filter(|i| i.0 == node)
                    .map(|i| SchemaIndex {
                        name: i.1.clone(),
                        definition: i.2.clone(),
                        unique: i.3,
                    })
                    .collect(),
                refs: refs
                    .iter()
                    .filter(|r| r.0 == node)
                    .map(|r| StoredSchemaRef {
                        from_column: r.1.clone(),
                        to: r.2.clone(),
                        to_column: r.3.clone(),
                        on_delete: r.4.clone(),
                    })
                    .collect(),
            })
            .collect())
    }
}
