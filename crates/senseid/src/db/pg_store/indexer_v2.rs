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

/// One EXTERNAL the graph names but never opens (R5), as the columns hold it.
///
/// Its own row type and not [`NodeColumns`], because a lib node has none of the
/// columns that make a definition one — no file, no lines, no docstring — and
/// widening `NodeColumns` with fields that are null for every lib row would put
/// the two shapes' emptiness on the same footing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibColumns {
    pub fqn: String,
    pub kind: String,
    pub name: String,
    /// `props.package`, as the row holds it. `None` means the row carries no
    /// such prop — the read states that rather than substituting a package
    /// derived from the fqn, which would make a missing write unreadable.
    pub package: Option<String>,
    /// The `lib_package` container's fqn, `None` for the container itself.
    pub parent_fqn: Option<String>,
}

/// What releasing one file's claim on an identity left behind (R10.4).
///
/// `still_claimed_by` is the whole point: a node another file still declares
/// must not be demoted, and a caller handed only "released" cannot tell the two
/// cases apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Released {
    pub node_id: uuid::Uuid,
    pub still_claimed_by: Vec<String>,
}

/// What taking a file's occurrences off an edge did.
///
/// Three outcomes and not a boolean, because "the row is still there" and "the
/// row was never there" are different facts and a caller that counts them
/// together is counting a miss as a keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dropped {
    /// Other files still contribute to this edge, so the row stands.
    Kept,
    /// Nothing was left on it, so the row is gone. The ONLY delete reconcile
    /// performs (D8).
    RowDeleted,
    /// No row under that id.
    NoSuchEdge,
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

/// The [`PgStore::v2_lib_nodes`] projection: fqn, kind, name, `props.package`,
/// and the container's fqn. Named for the reason [`NodeProjection`] is.
type LibProjection = (Option<String>, String, String, Option<String>, Option<String>);

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
    ///
    /// `parent_id` is a parameter and not a field of [`NodeColumns`] for the
    /// reason `folder_id` is: it is a row id the CALLER resolved, not something
    /// the walk read. It is also not optional in spirit —
    /// `nodes_unique_identity` is `(folder_id, file_path, kind, name, parent_id,
    /// line_start)` with NULLS NOT DISTINCT, so leaving it null collapses two
    /// same-named members of two types written on one line into one row, and
    /// the collapse happens BELOW the fqn where nothing counts it. `None` means
    /// the declaration is a member of nothing (a free item), which is a fact and
    /// not a gap.
    pub async fn upsert_v2_symbol(
        &self,
        folder_id: &uuid::Uuid,
        columns: &NodeColumns,
        parent_id: Option<&uuid::Uuid>,
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

        // A re-scan of a file nobody edited must not churn the graph. Without
        // this, every pass rewrites every node and stamps `modified_at = now()`
        // on all of them, and anything downstream that keys off freshness — an
        // embedding refresh, a "what changed" query — reads a whole repository
        // as touched every tick.
        //
        // A read before the write and not a narrower UPDATE, because the row is
        // written by TWO statements and the first of them is
        // `upsert_node_by_fqn`, which the shipped indexer also calls and whose
        // behaviour v2 may not change. Skipping is the only lever this side of
        // that boundary. It costs one SELECT on a first write and saves two
        // writes on every unchanged re-write, which is the direction a daemon
        // that re-scans runs in.
        if let Some(id) = self.v2_symbol_unchanged(folder_id, columns, parent_id).await? {
            return Ok(id);
        }

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
                    parent_id,
                }),
            )
            .await?;

        // `||` MERGES, the `set_node_props` idiom: props another writer owns
        // must survive a writer that knows less about them.
        //
        // `claims` is the exception and takes `jsonb_set`, the
        // [`Self::merge_v2_edge_occurrences`] idiom, for the same reason: `||`
        // at the top level would REPLACE the whole `claims` object with this
        // file's key and release every other file's claim on the same identity
        // (D9, R10.4). Two files can declare one fqn — measured, 2 in this
        // repository — and the claim set is what tells "F was the only declarer"
        // from "F was one of two".
        sqlx_core::query::query(
            "UPDATE sensei.nodes
                SET docstring = $2,
                    props = jsonb_set(
                        props || $3,
                        '{claims}',
                        coalesce(props -> 'claims', '{}'::jsonb)
                            || jsonb_build_object($4::text, true)),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(docstring.as_deref())
        .bind(props)
        .bind(file_path)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert_v2_symbol detail ({fqn}): {e}"))?;

        Ok(id)
    }

    /// The id of the row [`Self::upsert_v2_symbol`] would write, WHEN writing it
    /// would change nothing.
    ///
    /// Every column and prop that write sets is compared, and the comparison is
    /// the write's own semantics rather than a resemblance:
    ///
    /// - `signature IS NULL` because the write always sets it from a `None`.
    /// - `parent_id` only when one is supplied — `upsert_node_by_fqn` merges it
    ///   with `COALESCE`, so a `None` leaves whatever is there and changes
    ///   nothing.
    /// - the props key by key through `jsonb_each`, and NOT with `@>`.
    ///   Containment is the trap here: `params` is an ARRAY, and `[a, b] @> [b]`
    ///   is true, so a declaration that LOST a parameter would look unchanged and
    ///   the loss would never be written. Per-key `IS DISTINCT FROM` is exact,
    ///   and it stays out of v2's grammar (R7) — it compares whatever keys the
    ///   caller brought.
    /// - `props.claims` must already name this file, because claiming is part of
    ///   what the write does (D9).
    async fn v2_symbol_unchanged(
        &self,
        folder_id: &uuid::Uuid,
        columns: &NodeColumns,
        parent_id: Option<&uuid::Uuid>,
    ) -> Result<Option<uuid::Uuid>, String> {
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
        // Compared, not defaulted: a definition with no file is not one, and the
        // caller refuses it a line later. Nothing here may decide that a row
        // whose `file_path` is unknown matches the one on disk.
        let Some(file_path) = file_path.as_deref() else {
            return Ok(None);
        };

        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes
              WHERE folder_id = $1 AND fqn = $2
                AND resolved
                AND kind = $3::sensei.node_kind
                AND name = $4
                AND language IS NOT DISTINCT FROM $5
                AND file_path IS NOT DISTINCT FROM $6
                AND signature IS NULL
                AND line_start IS NOT DISTINCT FROM $7
                AND line_end IS NOT DISTINCT FROM $8
                AND is_exported = $9
                AND docstring IS NOT DISTINCT FROM $10
                AND ($11::uuid IS NULL OR parent_id IS NOT DISTINCT FROM $11)
                AND props -> 'claims' ? $12
                AND NOT EXISTS (
                        SELECT 1 FROM jsonb_each($13::jsonb) AS want(key, value)
                         WHERE props -> want.key IS DISTINCT FROM want.value)",
        )
        .bind(folder_id)
        .bind(fqn)
        .bind(kind)
        .bind(name)
        .bind(language.as_deref())
        .bind(file_path)
        .bind(line_start)
        .bind(line_end)
        .bind(is_exported)
        .bind(docstring.as_deref())
        .bind(parent_id)
        .bind(file_path)
        .bind(props)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("v2_symbol_unchanged ({fqn}): {e}"))?;
        Ok(row.map(|(id,)| id))
    }

    /// The id of the node `fqn` names, WHEN a reference to it would change
    /// nothing.
    ///
    /// The companion of [`Self::v2_symbol_unchanged`] for the other write a file
    /// makes: `upsert_node_by_fqn` with no definition, which is how a use site
    /// reaches a target no file has declared yet. On a row that already exists
    /// that statement changes exactly two things — it backfills `language`
    /// through `COALESCE(EXCLUDED.language, nodes.language)`, and it stamps
    /// `modified_at`. Everything else is written `nodes.<col>` back onto itself.
    /// So an equal `language` is the whole of "this would be a no-op", and
    /// skipping on it is not an optimisation with a behavioural tail.
    ///
    /// Without it a re-scan of an unchanged file still churns one row per file:
    /// the file's OWN module node, which the file references at every file-scope
    /// use site and which some other file declares.
    pub async fn v2_node_unchanged_by_reference(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        language: Option<&str>,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes
              WHERE folder_id = $1 AND fqn = $2 AND language IS NOT DISTINCT FROM $3",
        )
        .bind(folder_id)
        .bind(fqn)
        .bind(language)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("v2_node_unchanged_by_reference ({fqn}): {e}"))?;
        Ok(row.map(|(id,)| id))
    }

    /// Every identity `file_path` currently CLAIMS to declare (D9).
    ///
    /// The claim set and not `nodes.file_path`: that column holds one path, and
    /// a node two files declare has two claimants. It is also not the column
    /// that says which file's edges hang off a node — measured, 356 of this
    /// repository's 372 files have their own module node declared by a DIFFERENT
    /// file — so keying removal on it deletes rows nobody re-indexed (R10.1).
    pub async fn v2_claims_of_file(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
    ) -> Result<std::collections::BTreeSet<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT fqn FROM sensei.nodes
              WHERE folder_id = $1 AND fqn IS NOT NULL AND props -> 'claims' ? $2
              ORDER BY fqn",
        )
        .bind(folder_id)
        .bind(file_path)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("v2_claims_of_file ({file_path}): {e}"))?;
        Ok(rows.into_iter().map(|(fqn,)| fqn).collect())
    }

    /// Take `file_path`'s claim off one identity and say who is left (R10.4).
    ///
    /// `jsonb_set` and `-`, never a whole-props write: `claims` sits beside
    /// props another writer owns, and the point of the operation is to remove
    /// ONE key.
    ///
    /// `Ok(None)` when the folder holds no node under that fqn. That is a
    /// genuine absence — the caller asked to release a claim on something that
    /// is not there — and never a released-nothing reported as a release.
    pub async fn release_v2_claim(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        file_path: &str,
    ) -> Result<Option<Released>, String> {
        let row: Option<(uuid::Uuid, serde_json::Value)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.nodes
                SET props = jsonb_set(
                        props,
                        '{claims}',
                        coalesce(props -> 'claims', '{}'::jsonb) - $3),
                    modified_at = now()
              WHERE folder_id = $1 AND fqn = $2
          RETURNING id, coalesce(props -> 'claims', '{}'::jsonb)",
        )
        .bind(folder_id)
        .bind(fqn)
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("release_v2_claim ({fqn} / {file_path}): {e}"))?;

        row.map(|(node_id, remaining)| {
            // `RETURNING` reads the row as the UPDATE left it, so this is what
            // is STILL claimed. An object is the only shape `claims` is written
            // in; anything else means another writer put something there and
            // reconcile must not guess what it meant.
            let held_by = remaining.as_object().ok_or_else(|| {
                format!("release_v2_claim ({fqn}): props.claims is not an object: {remaining}")
            })?;
            Ok(Released { node_id, still_claimed_by: held_by.keys().cloned().collect() })
        })
        .transpose()
    }

    /// Turn a definition back into the stub shape, because no file claims it
    /// any more (R10.2, D8).
    ///
    /// Every definition-only column AND prop goes, not just `resolved`. A row
    /// that says `resolved = false` while still carrying a `line_start` answers
    /// "where is X defined" with a line in a file that no longer defines it,
    /// which is a fabricated reading (R4).
    ///
    /// `-` on the props and not a whole-props write, so `claims` — now empty but
    /// still the record of what the node is keyed by — and anything another
    /// writer owns survive.
    ///
    /// The row is NOT deleted. A node deletion cascades through `source_id`,
    /// `target_id` and `parent_id` with no count, and the input that would
    /// trigger it cannot be trusted: a damaged parse is undetectable (R10.3).
    /// Collecting a stub nothing points at belongs to
    /// [`Self::prune_orphan_stubs_scoped`], which already owns that predicate.
    pub async fn demote_v2_symbol(&self, node_id: &uuid::Uuid, kind: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.nodes
                SET resolved = false,
                    kind = $2::sensei.node_kind,
                    file_path = NULL,
                    line_start = NULL,
                    line_end = NULL,
                    signature = NULL,
                    docstring = NULL,
                    is_exported = false,
                    props = props - 'symbol_kind' - 'visibility' - 'declared_type'
                                  - 'params' - 'span_columns',
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(node_id)
        .bind(kind)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("demote_v2_symbol: {e}"))?;
        Ok(())
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

    /// Every external node of a folder — the rows an edge to a dependency mints.
    ///
    /// The read side of the four writes `crate::indexer::persist`'s
    /// `target_ref_of` reaches with a package: the container's fqn, the
    /// container's name, and `props.package` on both the container and the
    /// symbol. Until this existed nothing read any of them, so inverting the one
    /// line that derives the package left the whole suite green while every
    /// dependency in the graph was filed under the wrong name.
    pub async fn v2_lib_nodes(&self, folder_id: &uuid::Uuid) -> Result<Vec<LibColumns>, String> {
        let rows: Vec<LibProjection> = sqlx_core::query_as::query_as(
            "SELECT n.fqn, n.kind::text, n.name, n.props ->> 'package', p.fqn
                       FROM sensei.nodes n
                  LEFT JOIN sensei.nodes p ON p.id = n.parent_id
                      WHERE n.folder_id = $1
                        AND n.kind IN ('lib_package'::sensei.node_kind,
                                       'lib_symbol'::sensei.node_kind)
                   ORDER BY n.kind, n.fqn",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("v2_lib_nodes: {e}"))?;

        rows.into_iter()
            .map(|(fqn, kind, name, package, parent_fqn)| {
                // A lib node is minted by fqn and by nothing else, so one without
                // an fqn is a row this cannot have written.
                let fqn = fqn.ok_or_else(|| {
                    format!("v2_lib_nodes: a {kind} node named {name} carries no fqn")
                })?;
                Ok(LibColumns { fqn, kind, name, package, parent_fqn })
            })
            .collect()
    }

    /// What each v2 node is a MEMBER of, as `(node fqn, owner fqn)`.
    ///
    /// The read side of [`Self::upsert_v2_symbol`]'s `parent_id`. Without it
    /// that column is write-only: nothing in the round trip reads it, so
    /// `parent_id: None` sat there passing the whole suite while it silently
    /// merged rows the identity index could no longer tell apart.
    ///
    /// Both sides are FQNs and not ids, for the reason [`EdgeColumns`] gives:
    /// an id proves only that two rows agree, while an identity is what the
    /// caller can compare against the ownership relation the walk read.
    ///
    /// Every fqn-keyed node of the folder, not only definitions: a member whose
    /// owner has not been indexed yet is parented on the owner's STUB, and
    /// excluding stubs would report that as "no owner".
    pub async fn v2_containment(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<(String, Option<String>)>, String> {
        let rows: Vec<(Option<String>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT n.fqn, p.fqn
                   FROM sensei.nodes n
              LEFT JOIN sensei.nodes p ON p.id = n.parent_id
                  WHERE n.folder_id = $1 AND n.fqn IS NOT NULL
               ORDER BY n.fqn",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("v2_containment: {e}"))?;

        rows.into_iter()
            .map(|(fqn, parent)| {
                // `fqn IS NOT NULL` is in the predicate, so a null here means the
                // filter and the projection disagree — an error, never a row
                // invented to fill the hole.
                let fqn = fqn.ok_or_else(|| {
                    "v2_containment: a row matched `fqn IS NOT NULL` and carries no fqn".to_string()
                })?;
                Ok((fqn, parent))
            })
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

    /// Take ONE file's occurrences off an edge, and delete the row only when
    /// nothing is left on it (R10.4).
    ///
    /// Never `DELETE FROM sensei.edges WHERE source_id = ANY(…)`. That is the v1
    /// shape and it takes every other file's occurrences with it: an edge row is
    /// keyed `(folder, source, target, kind)` and two files can produce one —
    /// measured, 4 of this repository's 82,913.
    ///
    /// Two statements rather than one data-modifying CTE. A CTE's sub-statements
    /// cannot see each other's effects, so the DELETE would have to re-derive
    /// "is it empty now" from the row as it was BEFORE the UPDATE — the same
    /// value read twice under two different meanings, which is the shape this
    /// whole step exists to remove.
    pub async fn drop_v2_edge_occurrences(
        &self,
        edge_id: &uuid::Uuid,
        file_path: &str,
    ) -> Result<Dropped, String> {
        let row: Option<(bool,)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.edges
                SET props = jsonb_set(
                        props,
                        '{occurrences}',
                        coalesce(props -> 'occurrences', '{}'::jsonb) - $2),
                    modified_at = now()
              WHERE id = $1
          RETURNING coalesce(props -> 'occurrences', '{}'::jsonb) = '{}'::jsonb",
        )
        .bind(edge_id)
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("drop_v2_edge_occurrences ({file_path}): {e}"))?;

        let Some((emptied,)) = row else {
            return Ok(Dropped::NoSuchEdge);
        };
        if !emptied {
            return Ok(Dropped::Kept);
        }
        sqlx_core::query::query("DELETE FROM sensei.edges WHERE id = $1")
            .bind(edge_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("drop_v2_edge_occurrences delete ({file_path}): {e}"))?;
        Ok(Dropped::RowDeleted)
    }

    /// Every edge row `file_path` contributed an occurrence to (R10.1).
    ///
    /// `sources` is the identities the caller knows this file's edges can be
    /// sourced at: everything it claims, everything it USED to claim, and its
    /// own module identity. That last one is not optional — a file's own module
    /// node is minted by the `mod x;` in its PARENT, so it is in no claim set,
    /// and 3,290 of this repository's file-scope edges hang off exactly such a
    /// node.
    ///
    /// Two disjuncts, and the second is the correctness clause. A source this
    /// file emits at but does not declare and cannot name — measured, 26 of
    /// 119,248 edge sources, all split-`impl` anchors — is a node no file
    /// declares, so it is unresolved, and that is what the second reaches. A
    /// `gin` index on `props -> 'occurrences'` would collapse both into one
    /// clean test; it is DDL on a table the shipped indexer writes, so it is
    /// step 9's.
    ///
    /// The occurrence key is then required OUTRIGHT, so what comes back is
    /// exactly what this file contributed and never a row it merely sits near.
    /// [`Self::v2_edges_naming_file`] is the unnarrowed form the corpus test
    /// proves this against.
    pub async fn v2_edges_contributed_by(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
        sources: &[String],
    ) -> Result<Vec<uuid::Uuid>, String> {
        let rows: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT e.id
                   FROM sensei.edges e
                   JOIN sensei.nodes s ON s.id = e.source_id
                  WHERE e.folder_id = $1
                    AND e.props -> 'occurrences' ? $2
                    AND (s.fqn = ANY($3) OR s.resolved = false)
               ORDER BY e.id",
        )
        .bind(folder_id)
        .bind(file_path)
        .bind(sources)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("v2_edges_contributed_by ({file_path}): {e}"))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Every edge row whose occurrences NAME `file_path`, with no narrowing.
    ///
    /// The unit of edge attribution as R10.1 states it, and therefore the
    /// definition [`Self::v2_edges_contributed_by`] is measured against. Kept
    /// apart from it rather than used in its place because it can use no index
    /// at all until step 9 adds one, and a reconcile that scans every edge of a
    /// folder per file is a different cost curve.
    pub async fn v2_edges_naming_file(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
    ) -> Result<Vec<uuid::Uuid>, String> {
        let rows: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.edges
              WHERE folder_id = $1 AND props -> 'occurrences' ? $2
              ORDER BY id",
        )
        .bind(folder_id)
        .bind(file_path)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("v2_edges_naming_file ({file_path}): {e}"))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
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
