//! Indexer v2's row-level access to `sensei.nodes` and `sensei.edges`
//! (`docs/plans/indexer-v2-rust.md`, step 7).
//!
//! Everything here is v2's and has no caller on the shipped path. The policy —
//! which fact becomes which row — lives in `crate::indexer::persist`; this file
//! is only the SQL that layer cannot write for itself.
//!
//! Deliberately thin. The node MERGE is not here: `upsert_node_by_fqn` already
//! owns it, including the `nodes_unique_identity` recovery that took two
//! production incidents to get right, and a second copy of that statement is
//! how the two would drift.

use super::{FqnDef, PgStore};

/// One `sensei.nodes` row, as the columns hold it.
///
/// Raw on purpose: `crate::indexer::persist` owns the decoding, because turning
/// a `kind` label back into a [`crate::indexer::facts::SymbolKind`] is v2
/// grammar and this file is not where v2 grammar lives (R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeColumns {
    pub fqn: String,
    pub kind: String,
    pub name: String,
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub is_exported: bool,
    pub docstring: Option<String>,
    pub props: serde_json::Value,
}

/// One `sensei.edges` row, with both endpoints as the identities they carry
/// rather than as the row ids they are stored under.
///
/// `target_fqn` is `Some` exactly when the edge has a `target_id`; `target_name`
/// is `Some` exactly when it does not. That is the table's own shape — the two
/// partial unique indexes key a resolved edge by its target node and an
/// unresolved one by the name — and it is what makes an unresolved reference a
/// row rather than an absence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeColumns {
    pub source_fqn: String,
    pub kind: String,
    pub target_fqn: Option<String>,
    pub target_name: Option<String>,
    pub props: serde_json::Value,
}

/// The `sensei.nodes` projection [`PgStore::v2_definition_nodes`] selects, in
/// column order, and the `sensei.edges` one [`PgStore::v2_edges`] selects.
///
/// Named rather than written inline: sqlx hands back a positional tuple, and a
/// ten-element tuple spelled out at the binding site is exactly the shape this
/// step exists to keep out of the code. The alias is where the column order is
/// stated once; the destructure below is what names each element.
type NodeProjection = (
    Option<String>,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    bool,
    Option<String>,
    serde_json::Value,
);

type EdgeProjection = (Option<String>, String, Option<String>, Option<String>, serde_json::Value);

impl PgStore {
    /// Write one v2 declaration.
    ///
    /// Takes the row as ONE value and never as an argument list (R9): a
    /// positional insert is a hand-copied enumeration of the producer's fields,
    /// so adding a field to the producer leaves it compiling and silently
    /// carrying less. That is not a hypothetical — it is why
    /// `extract_return_type` ran on every function for months while no return
    /// type ever reached a column.
    ///
    /// Two statements, and the split is forced. [`Self::upsert_node_by_fqn`]
    /// owns identity and the merge contract, and it has a slot for neither
    /// `docstring` nor `props`; giving it one would change a function the
    /// shipped indexer calls on every symbol it writes. So the second statement
    /// carries what the first has nowhere to put. It writes both in ONE round
    /// trip rather than reusing `set_node_props` and adding a third: the caller
    /// runs this per symbol, and the live index holds 136,583 of them.
    pub async fn upsert_v2_symbol(
        &self,
        folder_id: &uuid::Uuid,
        columns: &NodeColumns,
    ) -> Result<uuid::Uuid, String> {
        let NodeColumns {
            fqn,
            kind,
            name,
            file_path,
            language,
            line_start,
            line_end,
            is_exported,
            docstring,
            props,
        } = columns;

        // A definition without a home file is not a definition — it is the stub
        // shape, which `v2_definition_nodes` deliberately does not read back.
        // Writing one here would put a row in the graph that reads as a
        // declaration and names no source.
        let file_path = file_path
            .as_deref()
            .ok_or_else(|| format!("upsert_v2_symbol: {fqn} has no file path"))?;

        let id = self
            .upsert_node_by_fqn(
                folder_id,
                fqn,
                kind,
                name,
                language.as_deref(),
                Some(FqnDef {
                    file_path,
                    signature: None,
                    line_start: *line_start,
                    line_end: *line_end,
                    is_exported: *is_exported,
                    parent_id: None,
                }),
            )
            .await?;

        // `||` MERGES, the `set_node_props` idiom: props another writer owns
        // must survive a writer that knows less about them.
        sqlx_core::query::query(
            "UPDATE sensei.nodes
                SET docstring = $2, props = props || $3, modified_at = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(docstring.as_deref())
        .bind(props)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert_v2_symbol detail ({fqn}): {e}"))?;

        Ok(id)
    }

    /// Every DEFINITION node of a folder — the rows a v2 symbol write produced.
    ///
    /// `file_path IS NOT NULL` is what separates them from the two node shapes
    /// that legitimately have no file: the reference stubs a call to a
    /// not-yet-indexed definition creates, and the `lib_symbol`/`lib_package`
    /// nodes an external reference mints (R5). Both are edges' business, not
    /// symbols'.
    pub async fn v2_definition_nodes(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<NodeColumns>, String> {
        let rows: Vec<NodeProjection> = sqlx_core::query_as::query_as(
            "SELECT fqn, kind::text, name, file_path, language, line_start, line_end,
                    is_exported, docstring, props
               FROM sensei.nodes
              WHERE folder_id = $1 AND file_path IS NOT NULL
              ORDER BY fqn",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("v2_definition_nodes: {e}"))?;

        rows.into_iter()
            .map(
                |(
                    fqn,
                    kind,
                    name,
                    file_path,
                    language,
                    line_start,
                    line_end,
                    is_exported,
                    docstring,
                    props,
                )| {
                    // A definition with no fqn is a LEGACY row, not a v2 one, and
                    // reading it as v2 would report a symbol nothing here wrote.
                    let fqn = fqn.ok_or_else(|| {
                        format!("v2_definition_nodes: {name} in {file_path:?} carries no fqn")
                    })?;
                    Ok(NodeColumns {
                        fqn,
                        kind,
                        name,
                        file_path,
                        language,
                        line_start,
                        line_end,
                        is_exported,
                        docstring,
                        props,
                    })
                },
            )
            .collect()
    }
}

impl PgStore {
    /// Record what ONE FILE contributes to one edge, keeping what other files
    /// contributed to the same edge.
    ///
    /// A second statement rather than props on the insert, for the reason
    /// [`Self::upsert_v2_symbol`] needs one: `insert_edge_with_props` merges
    /// with `props = edges.props || EXCLUDED.props`, and jsonb `||` REPLACES a
    /// key rather than appending to it. An edge is keyed `(folder, source,
    /// target, kind)` and two files can produce the same edge — `crates/mcp`'s
    /// `lib.rs` and `main.rs` reduce to one module path, so the imports of both
    /// hang off one file identity — and with a flat list the second write
    /// silently erased the first's occurrences.
    ///
    /// Keying the list BY FILE is what satisfies both halves at once, using the
    /// same `||` and no new merge policy: the object's own key replaces this
    /// file's list, so a re-scan drops spans that have moved, and every other
    /// file's list is untouched.
    ///
    /// `jsonb_set` and not a whole-props write, because `props` is shared —
    /// `props.relation` is written by the insert above and read by
    /// `prune_mislabelled_containment_extends`, and a writer that knows less
    /// about a key must not erase it.
    pub async fn merge_v2_edge_occurrences(
        &self,
        edge_id: &uuid::Uuid,
        file_path: &str,
        occurrences: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.edges
                SET props = jsonb_set(
                        props,
                        '{occurrences}',
                        coalesce(props -> 'occurrences', '{}'::jsonb)
                            || jsonb_build_object($2::text, $3::jsonb)),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(edge_id)
        .bind(file_path)
        .bind(occurrences)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("merge_v2_edge_occurrences ({file_path}): {e}"))?;
        Ok(())
    }

    /// Every v2 edge of a folder.
    ///
    /// Selected by `props -> 'occurrences'`, which only a v2 write puts there.
    /// A folder can hold both indexers' output during the cutover, and reading
    /// a v1 edge as a v2 one would report facts v2 never produced.
    pub async fn v2_edges(&self, folder_id: &uuid::Uuid) -> Result<Vec<EdgeColumns>, String> {
        let rows: Vec<EdgeProjection> = sqlx_core::query_as::query_as(
            "SELECT s.fqn, e.kind::text, t.fqn, e.target_name, e.props
                   FROM sensei.edges e
                   JOIN sensei.nodes s ON s.id = e.source_id
              LEFT JOIN sensei.nodes t ON t.id = e.target_id
                  WHERE e.folder_id = $1 AND e.props -> 'occurrences' IS NOT NULL
               ORDER BY s.fqn, e.kind, t.fqn, e.target_name",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("v2_edges: {e}"))?;

        rows.into_iter()
            .map(|(source_fqn, kind, target_fqn, target_name, props)| {
                // Only a v2 write puts `occurrences` in props, and every node a
                // v2 write touches is fqn-keyed. A source without one means the
                // filter above matched something this cannot read.
                let source_fqn = source_fqn.ok_or_else(|| {
                    format!("v2_edges: a {kind} edge has a source node with no fqn")
                })?;
                Ok(EdgeColumns { source_fqn, kind, target_fqn, target_name, props })
            })
            .collect()
    }
}
