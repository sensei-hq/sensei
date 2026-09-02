use super::*;

/// True only for a unique violation on `nodes_unique_identity` — the structural
/// identity index `(folder_id, file_path, kind, name, parent_id, line_start)`.
///
/// Matched by constraint NAME rather than by the generic 23505 code so the
/// adopt-by-identity fallback can never silently absorb a different conflict
/// (notably `nodes_unique_fqn`, which the ON CONFLICT clause already handles).
fn is_identity_conflict(e: &sqlx_core::error::Error) -> bool {
    matches!(
        e,
        sqlx_core::error::Error::Database(db) if db.constraint() == Some("nodes_unique_identity")
    )
}

/// Which side of a `calls` relation a coverage count is about — incoming edges
/// (who calls this) or outgoing ones (what this calls). A closed enum rather
/// than a string so [`PgStore::call_coverage`] picks its filter column at
/// compile time and no caller input ever reaches the SQL text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    Incoming,
    Outgoing,
}

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// BM25-style keyword ranking: matches nodes by name/signature/docstring.
    pub async fn rank_bm25(
        &self,
        folder_id: &uuid::Uuid,
        query: &str,
    ) -> Result<Vec<(String, f64)>, String> {
        let rows: Vec<(String, f64)> =
            sqlx_core::query_as::query_as("SELECT file_path, score FROM sensei.rank_bm25($1, $2)")
                .bind(folder_id)
                .bind(query)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── Graph (typed wrappers) ─────────────────────────────────────────

    pub async fn merge_function(
        &self,
        folder_id: &uuid::Uuid,
        name: &str,
        file_path: &str,
        signature: Option<&str>,
        line_start: Option<i32>,
        line_end: Option<i32>,
        parent_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(
            folder_id, "function", name, file_path, parent_id, signature, line_start, line_end,
        )
        .await
    }

    pub async fn merge_file(
        &self,
        folder_id: &uuid::Uuid,
        name: &str,
        file_path: &str,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, "file", name, file_path, None, None, None, None).await
    }

    pub async fn merge_type(
        &self,
        folder_id: &uuid::Uuid,
        name: &str,
        file_path: &str,
        kind: &str,
        line_start: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, kind, name, file_path, None, None, line_start, None).await
    }

    pub async fn merge_doc(
        &self,
        folder_id: &uuid::Uuid,
        name: &str,
        file_path: &str,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, "doc", name, file_path, None, None, None, None).await
    }

    pub async fn project_exists(&self, folder_id: &uuid::Uuid) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE folder_id = $1)",
        )
        .bind(folder_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn search_functions(
        &self,
        folder_id: &uuid::Uuid,
        query: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.search_functions_scoped(std::slice::from_ref(folder_id), query).await
    }

    pub async fn search_types(
        &self,
        folder_id: &uuid::Uuid,
        query: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.search_types_scoped(std::slice::from_ref(folder_id), query).await
    }

    pub async fn count_nodes_by_kind(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<std::collections::HashMap<String, i64>, String> {
        self.count_nodes_by_kind_scoped(std::slice::from_ref(folder_id)).await
    }

    pub async fn delete_node(&self, node_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE id = $1")
            .bind(node_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_nodes_by_file(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2")
            .bind(folder_id)
            .bind(file_path)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Distinct file paths (repo-relative) that a folder has indexed nodes for.
    /// Excludes `module` nodes — those record an ABSOLUTE directory path (not a
    /// file) and are re-derived structurally, so mixing them into a rel-path
    /// comparison would be wrong. Used by the reconcile's `prune_vanished` safety
    /// net to find nodes whose file no longer exists on disk.
    pub async fn list_indexed_files(&self, folder_id: &uuid::Uuid) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT file_path FROM sensei.nodes
              WHERE folder_id = $1 AND kind::text <> 'module' AND file_path <> ''",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    pub async fn clear_all_nodes(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        self.delete_nodes_by_folder(folder_id).await
    }

    // ── Repo (folders with kind='git'/'subtree') ──────────────────────

    /// Merge into a node's `props` jsonb (D5b): used to stamp a `section` node's
    /// `level` and real `line_start` (the identity key carries a NULL line so
    /// section identity is line-independent — 0.4). Idempotent (`props || $2`).
    pub async fn set_node_props(
        &self,
        node_id: &uuid::Uuid,
        props: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.nodes SET props = props || $2, modified_at = now() WHERE id = $1",
        )
        .bind(node_id)
        .bind(props)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Upsert a node (default `is_exported = false`). Thin wrapper over
    /// [`Self::upsert_node_ex`] for the many callers that don't carry visibility
    /// (file/section/rationale/module nodes, tests).
    pub async fn upsert_node(
        &self,
        folder_id: &uuid::Uuid,
        kind: &str,
        name: &str,
        file_path: &str,
        parent_id: Option<&uuid::Uuid>,
        signature: Option<&str>,
        line_start: Option<i32>,
        line_end: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node_ex(
            folder_id, kind, name, file_path, parent_id, signature, line_start, line_end, false,
        )
        .await
    }

    /// Upsert a node carrying `is_exported` (the code-symbol path passes the
    /// parser's `pub`/`export` visibility). `is_exported` is written on INSERT and
    /// refreshed on the D3 upsert-then-prune conflict, so a symbol that flips
    /// pub↔private is kept current.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_node_ex(
        &self,
        folder_id: &uuid::Uuid,
        kind: &str,
        name: &str,
        file_path: &str,
        parent_id: Option<&uuid::Uuid>,
        signature: Option<&str>,
        line_start: Option<i32>,
        line_end: Option<i32>,
        is_exported: bool,
    ) -> Result<uuid::Uuid, String> {
        // ON CONFLICT targets nodes_unique_identity (folder_id, file_path, kind, name,
        // parent_id, line_start NULLS NOT DISTINCT). DO UPDATE keeps the row STABLE on
        // re-scans — same UUID whether just inserted or pre-existing (D3 upsert-then-
        // prune) — preserving community_id and degree. It refreshes signature/line_end,
        // and re-nulls `embedding` ONLY when the signature changed: `embed_text` is a
        // function of (kind, name, signature, file_path), and on a same-identity
        // conflict the first three-of-four are fixed by the key, so `signature` is the
        // only embed input that can change — nulling on that (and preserving it
        // otherwise) keeps embeddings fresh without a separate content_hash column.
        // `language` is derived from the file extension at write time (the single
        // shared mapping). Populating it on THIS legacy path too — every non-Rust +
        // file/section/rationale node flows through here for the whole FQN
        // transition — is what gives the same-language bare-name fallback (plan 0.8)
        // something to filter on. COALESCE on conflict backfills pre-existing rows.
        let language = crate::languages::language_for_path(file_path);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes(folder_id, kind, name, file_path, parent_id, signature, line_start, line_end, is_exported, language)
             VALUES($1, $2::sensei.node_kind, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (folder_id, file_path, kind, name, parent_id, line_start) WHERE file_path IS NOT NULL DO UPDATE
               SET signature   = EXCLUDED.signature,
                   line_end    = EXCLUDED.line_end,
                   is_exported = EXCLUDED.is_exported,
                   language    = COALESCE(EXCLUDED.language, nodes.language),
                   embedding   = CASE WHEN nodes.signature IS DISTINCT FROM EXCLUDED.signature
                                      THEN NULL ELSE nodes.embedding END,
                   modified_at = now()
             RETURNING id"
        ).bind(folder_id).bind(kind).bind(name).bind(file_path)
            .bind(parent_id).bind(signature).bind(line_start).bind(line_end).bind(is_exported).bind(language)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Get-or-create a node by its fully-qualified name (SCIP/LSIF moniker model).
    /// A REFERENCE (`def = None`) creates — or returns — an unresolved STUB
    /// (`resolved=false`, NULL `file_path`). A DEFINITION (`def = Some`) creates or
    /// ENRICHES the same `(folder_id, fqn)` node in place: flips `resolved=true` and
    /// fills `file_path`/`signature`/`line_start`/`line_end`/`is_exported`/`parent_id`.
    ///
    /// Monotone + idempotent: a reference NEVER downgrades an already-resolved node
    /// (`resolved = OLD OR NEW`; def-only columns are kept unless the incoming row is
    /// itself a definition), and re-enrichment re-nulls the embedding only when the
    /// signature changed — the same freshness rule as `upsert_node_ex`. Arbiter is
    /// the partial `nodes_unique_fqn` index, so this coexists with the line-based
    /// `nodes_unique_identity`.
    pub async fn upsert_node_by_fqn(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        kind: &str,
        name: &str,
        language: Option<&str>,
        def: Option<FqnDef<'_>>,
    ) -> Result<uuid::Uuid, String> {
        let resolved = def.is_some();
        let file_path = def.as_ref().map(|d| d.file_path);
        let signature = def.as_ref().and_then(|d| d.signature);
        let line_start = def.as_ref().and_then(|d| d.line_start);
        let line_end = def.as_ref().and_then(|d| d.line_end);
        let is_exported = def.as_ref().is_some_and(|d| d.is_exported);
        let parent_id = def.as_ref().and_then(|d| d.parent_id);

        let inserted: Result<(uuid::Uuid,), sqlx_core::error::Error> = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes
                 (folder_id, fqn, kind, name, language, resolved,
                  file_path, signature, line_start, line_end, is_exported, parent_id)
             VALUES($1, $2, $3::sensei.node_kind, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             ON CONFLICT (folder_id, fqn) WHERE fqn IS NOT NULL DO UPDATE
               SET resolved    = nodes.resolved OR EXCLUDED.resolved,
                   kind        = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.kind ELSE nodes.kind END,
                   file_path   = COALESCE(EXCLUDED.file_path, nodes.file_path),
                   signature   = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.signature ELSE nodes.signature END,
                   line_start  = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.line_start ELSE nodes.line_start END,
                   line_end    = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.line_end ELSE nodes.line_end END,
                   is_exported = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.is_exported ELSE nodes.is_exported END,
                   parent_id   = COALESCE(EXCLUDED.parent_id, nodes.parent_id),
                   language    = COALESCE(EXCLUDED.language, nodes.language),
                   embedding   = CASE WHEN EXCLUDED.resolved
                                       AND nodes.signature IS DISTINCT FROM EXCLUDED.signature
                                      THEN NULL ELSE nodes.embedding END,
                   modified_at = now()
             RETURNING id"
        )
        .bind(folder_id).bind(fqn).bind(kind).bind(name).bind(language).bind(resolved)
        .bind(file_path).bind(signature).bind(line_start).bind(line_end).bind(is_exported).bind(parent_id)
        .fetch_one(&self.pool).await;

        let row: (uuid::Uuid,) = match inserted {
            Ok(r) => r,
            Err(e) if is_identity_conflict(&e) => {
                // The row already exists under a DIFFERENT fqn: `ON CONFLICT
                // (folder_id, fqn)` above can't see it, so the insert fell through
                // to a raw INSERT and hit `nodes_unique_identity`
                // (folder_id, file_path, kind, name, parent_id, line_start).
                //
                // This happens whenever a node's fqn SHAPE changes for a file that
                // was already indexed — e.g. the module container's fqn language
                // segment is derived from the parse output, so it flips when a parse
                // stops yielding top-level defs. Without this branch the write fails
                // forever: process_file returns Err, fail_folder withholds
                // scan_state, the reconcile re-drives the folder every tick, and the
                // folder never leaves `failed`.
                //
                // Adopt the existing row by re-pointing its fqn at the new value.
                // Keyed on the identity columns so we update exactly the row that
                // blocked us — NULLS NOT DISTINCT mirrors the index semantics.
                self.adopt_node_by_identity(
                    folder_id,
                    fqn,
                    kind,
                    name,
                    language,
                    resolved,
                    file_path,
                    signature,
                    line_start,
                    line_end,
                    is_exported,
                    parent_id,
                )
                .await?
            }
            Err(e) => return Err(e.to_string()),
        };
        Ok(row.0)
    }

    /// Re-point an existing node's `fqn` when an fqn-keyed upsert collided with
    /// `nodes_unique_identity`. Matches on the identity columns using
    /// `IS NOT DISTINCT FROM` so NULL `parent_id`/`line_start` compare equal, the
    /// same way the index's `NULLS NOT DISTINCT` does.
    #[allow(clippy::too_many_arguments)]
    async fn adopt_node_by_identity(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        kind: &str,
        name: &str,
        language: Option<&str>,
        resolved: bool,
        file_path: Option<&str>,
        signature: Option<&str>,
        line_start: Option<i32>,
        line_end: Option<i32>,
        is_exported: bool,
        parent_id: Option<&uuid::Uuid>,
    ) -> Result<(uuid::Uuid,), String> {
        sqlx_core::query_as::query_as(
            "UPDATE sensei.nodes SET
                 fqn         = $2,
                 resolved    = resolved OR $6,
                 kind        = CASE WHEN $6 THEN $3::sensei.node_kind ELSE kind END,
                 signature   = CASE WHEN $6 THEN $8 ELSE signature END,
                 line_end    = CASE WHEN $6 THEN $10 ELSE line_end END,
                 is_exported = CASE WHEN $6 THEN $11 ELSE is_exported END,
                 language    = COALESCE($5, language),
                 embedding   = CASE WHEN $6 AND signature IS DISTINCT FROM $8
                                    THEN NULL ELSE embedding END,
                 modified_at = now()
               WHERE folder_id = $1
                 AND file_path  IS NOT DISTINCT FROM $7
                 AND kind       = $3::sensei.node_kind
                 AND name       = $4
                 AND parent_id  IS NOT DISTINCT FROM $12
                 AND line_start IS NOT DISTINCT FROM $9
             RETURNING id",
        )
        .bind(folder_id)
        .bind(fqn)
        .bind(kind)
        .bind(name)
        .bind(language)
        .bind(resolved)
        .bind(file_path)
        .bind(signature)
        .bind(line_start)
        .bind(line_end)
        .bind(is_exported)
        .bind(parent_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("adopt node by identity ({name}): {e}"))
    }

    /// Get-or-create a first-class `lib_symbol` node for an EXTERNAL reference (a
    /// dependency's symbol), grouped under a per-package `lib_package` container so
    /// the graph shows "what we depend on and how much" (blueprint Fix 1, case 2).
    /// Both are `resolved=true` (the external symbol IS its own definition) with
    /// NULL `file_path` (no local file); the symbol's `parent_id` is its container.
    /// Owned by the referencing repo-root `folder_id` so they cascade with it.
    /// Stable ids across repeated references (arbiter = `nodes_unique_fqn`).
    /// Distinct import targets with their edge and resolved counts.
    ///
    /// DISTINCT on purpose: 136,484 import edges reduce to 15,533 distinct targets,
    /// so the caller classifies 15k strings instead of 136k rows. The
    /// classification itself stays in Rust
    /// ([`crate::languages::import_target::classify_import`]) — one owner. A SQL
    /// copy of the rule is how the scan exclusion resolver came to gate the watcher
    /// while pruning nothing.
    ///
    /// Propagates a read failure: an empty breakdown would report a codebase with
    /// no dependencies, which no codebase has.
    pub async fn import_target_counts(&self) -> Result<Vec<(String, i64, i64)>, String> {
        // `COALESCE` guards the DECODE, not a case in the data: `target_name` is
        // nullable in the schema but MEASURED non-null on all 136,484 import edges
        // (and no edge anywhere has neither target set — 0 of 715,985). The tuple
        // decodes to `String`, so a NULL that ever appeared would be a 500 rather
        // than one odd row.
        //
        // Grouped by TARGET, not by class: the classification lives in Rust
        // (`classify_import`) and putting it here too would be a second copy of the
        // rule. 136,484 edges reduce to 15,533 distinct targets, so the caller
        // classifies 15k strings instead of 136k rows.
        sqlx_core::query_as::query_as(
            "SELECT COALESCE(target_name, '') AS target,
                    count(*) AS edges,
                    count(*) FILTER (WHERE target_id IS NOT NULL) AS resolved
               FROM sensei.edges
              WHERE kind = 'imports'
              GROUP BY 1",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("import_target_counts: {e}"))
    }

    pub async fn upsert_lib_node_by_fqn(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        name: &str,
        package: &str,
    ) -> Result<uuid::Uuid, String> {
        // One `lib_package` container per dependency (fqn = `lib·<package>`).
        let pkg_fqn = format!("lib{}{}", crate::languages::fqn::SEP, package);
        let container: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes
                 (folder_id, fqn, kind, name, resolved, props)
             VALUES($1, $2, 'lib_package'::sensei.node_kind, $3, true,
                    jsonb_build_object('package', $3::text))
             ON CONFLICT (folder_id, fqn) WHERE fqn IS NOT NULL DO UPDATE
               SET resolved = true, modified_at = now()
             RETURNING id",
        )
        .bind(folder_id)
        .bind(&pkg_fqn)
        .bind(package)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        // The symbol, parented under its package container.
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes
                 (folder_id, fqn, kind, name, resolved, parent_id, props)
             VALUES($1, $2, 'lib_symbol'::sensei.node_kind, $3, true, $4,
                    jsonb_build_object('package', $5::text))
             ON CONFLICT (folder_id, fqn) WHERE fqn IS NOT NULL DO UPDATE
               SET resolved    = true,
                   parent_id   = COALESCE(EXCLUDED.parent_id, nodes.parent_id),
                   props       = nodes.props || jsonb_build_object('package', $5::text),
                   modified_at = now()
             RETURNING id",
        )
        .bind(folder_id)
        .bind(fqn)
        .bind(name)
        .bind(container.0)
        .bind(package)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// External dependencies referenced by a repo — one row per `lib_package` with
    /// how many of its symbols the repo actually uses (`{package, symbol_count}`).
    /// The graph-visible "what we depend on and how much".
    pub async fn list_dependencies(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT p.name, count(s.id)
               FROM sensei.nodes p
               LEFT JOIN sensei.nodes s
                 ON s.folder_id = p.folder_id
                AND s.parent_id = p.id
                AND s.kind = 'lib_symbol'::sensei.node_kind
              WHERE p.folder_id = $1 AND p.kind = 'lib_package'::sensei.node_kind
              GROUP BY p.name
              ORDER BY count(s.id) DESC, p.name",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(package, symbol_count)| {
            serde_json::json!({ "package": package, "symbol_count": symbol_count })
        }).collect())
    }

    pub async fn get_nodes_by_folder(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.get_nodes_scoped(std::slice::from_ref(folder_id)).await
    }

    /// Nodes in a folder that still need an embedding, restricted to the kinds
    /// worth embedding (code symbols + files + doc sections). Returns
    /// `(id, kind, name, signature, file_path)` — the fields needed to build the
    /// embedding text. Used by the `EmbedNodes` task.
    pub async fn nodes_without_embeddings(
        &self,
        folder_id: &uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<String>, String)>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind::text, name, signature, file_path
                   FROM sensei.nodes
                  WHERE folder_id = $1
                    AND embedding IS NULL
                    AND file_path IS NOT NULL
                    AND kind IN ('file','function','method','class','interface',
                                 'type','const','enum','enum_variant','section',
                                 'struct','component','hook','doc','extension')
                  ORDER BY file_path, line_start
                  LIMIT $2",
            )
            .bind(folder_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Store a node's vector embedding. The slice is rendered to pgvector's text
    /// form (`[v1,v2,...]`) and cast to `vector`, so no pgvector crate is needed.
    pub async fn set_node_embedding(
        &self,
        node_id: &uuid::Uuid,
        embedding: &[f32],
    ) -> Result<(), String> {
        let buf = vector_literal(embedding);
        sqlx_core::query::query("UPDATE sensei.nodes SET embedding = $1::vector WHERE id = $2")
            .bind(buf)
            .bind(node_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Semantic nearest-neighbour search over node embeddings, scoped to the
    /// given folders + node kinds. Reuses the same pgvector cosine-distance
    /// operator (`<=>`) as `find_duplicates`, ordering by ascending distance so
    /// the most semantically similar nodes come first. The query embedding is
    /// rendered with `vector_literal` and cast to `vector`, matching how
    /// `set_node_embedding` stores node vectors. Bounded by `limit` so it never
    /// materially slows the common query path. Returns
    /// `(id, name, file_path, signature, line_start)` — the fields the query
    /// handler projects into function/type hits for fusion with lexical results.
    pub async fn semantic_search_nodes(
        &self,
        folder_ids: &[uuid::Uuid],
        query_embedding: &[f32],
        kinds: &[&str],
        limit: i64,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<String>, Option<i32>)>, String> {
        if folder_ids.is_empty() || query_embedding.is_empty() || kinds.is_empty() {
            return Ok(Vec::new());
        }
        let vec_literal = vector_literal(query_embedding);
        let kind_strs: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<i32>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, file_path, signature, line_start
                   FROM sensei.nodes
                  WHERE folder_id = ANY($1::uuid[])
                    AND kind::text = ANY($3::text[])
                    AND embedding IS NOT NULL
                  ORDER BY embedding <=> $2::vector
                  LIMIT $4",
            )
            .bind(folder_ids)
            .bind(vec_literal)
            .bind(kind_strs)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Resolve nodes to their on-disk locations for snippet extraction, keyed by
    /// id. Returns `(id, abs_path, file_path, line_start, line_end, kind, name,
    /// signature)` — the repo's `abs_path` joined with the node `file_path` is the
    /// file to read, and the line range bounds the snippet. Missing line info
    /// falls back to line 1 (a one-line snippet). Used by `context_pack`.
    #[allow(clippy::type_complexity)]
    pub async fn node_locations(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<Vec<(uuid::Uuid, String, String, i32, i32, String, String, Option<String>)>, String>
    {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx_core::query_as::query_as(
            "SELECT n.id, f.abs_path, n.file_path,
                    COALESCE(n.line_start, 1),
                    COALESCE(n.line_end, n.line_start, 1),
                    n.kind::text, n.name, n.signature
               FROM sensei.nodes n
               JOIN sensei.folders f ON f.id = n.folder_id
              WHERE n.id = ANY($1::uuid[])
                AND n.file_path IS NOT NULL",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Find near-duplicate function/method pairs within a folder by cosine
    /// similarity on their code embeddings (HNSW `<=>` cosine distance). Each
    /// pair is returned once (`a.id < b.id`) at or above `min_similarity`,
    /// strongest first. Trivial functions (< 4 lines) are skipped — they bound
    /// the O(n²) self-join and avoid false positives from boilerplate. On-demand
    /// review query, not a hot path.
    pub async fn find_duplicates(
        &self,
        folder_id: &uuid::Uuid,
        min_similarity: f64,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let max_distance = 1.0 - min_similarity;
        let rows: Vec<(String, String, Option<i32>, String, String, Option<i32>, f64)> =
            sqlx_core::query_as::query_as(
                "SELECT a.name, a.file_path, a.line_start,
                        b.name, b.file_path, b.line_start,
                        1 - (a.embedding <=> b.embedding) AS similarity
                   FROM sensei.nodes a
                   JOIN sensei.nodes b
                     ON b.folder_id = a.folder_id
                    AND a.id < b.id
                    AND b.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND b.embedding IS NOT NULL
                    AND (b.line_end - b.line_start) >= 3
                  WHERE a.folder_id = $1
                    AND a.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND a.embedding IS NOT NULL
                    AND (a.line_end - a.line_start) >= 3
                    AND (a.embedding <=> b.embedding) <= $2
                  ORDER BY similarity DESC
                  LIMIT $3",
            )
            .bind(folder_id)
            .bind(max_distance)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(na, fa, la, nb, fb, lb, sim)| {
                serde_json::json!({
                    "a": { "name": na, "file": fa, "line": la },
                    "b": { "name": nb, "file": fb, "line": lb },
                    "similarity": (sim * 10000.0).round() / 10000.0,
                })
            })
            .collect())
    }

    /// Multi-folder variant of `find_duplicates` (#54). Runs the same
    /// cosine-similarity self-join but scopes the pair search to every
    /// folder belonging to a project — so a duplicate function defined in
    /// `crates/foo/src/x.rs` and `crates/bar/src/y.rs` (both inside the
    /// same project) surfaces even though they don't share a folder_id.
    ///
    /// Pairs are restricted to `a.id < b.id` so each dyad appears once. It does
    /// NOT require `a.folder_id != b.folder_id`: in a monorepo the indexer rolls
    /// every function node up to the single repo-root folder, so a cross-folder-only
    /// filter made this always return `count:0` (masking every real duplicate).
    /// The handler uses either this OR `find_duplicates` per call (never both), so
    /// there is no double-count to guard against.
    pub async fn find_duplicates_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
        min_similarity: f64,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() {
            return Ok(Vec::new());
        }
        let max_distance = 1.0 - min_similarity;
        let rows: Vec<(String, String, Option<i32>, String, String, Option<i32>, f64)> =
            sqlx_core::query_as::query_as(
                "SELECT a.name, a.file_path, a.line_start,
                        b.name, b.file_path, b.line_start,
                        1 - (a.embedding <=> b.embedding) AS similarity
                   FROM sensei.nodes a
                   JOIN sensei.nodes b
                     ON a.id < b.id
                    AND b.folder_id = ANY($1::uuid[])
                    AND b.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND b.embedding IS NOT NULL
                    AND (b.line_end - b.line_start) >= 3
                  WHERE a.folder_id = ANY($1::uuid[])
                    AND a.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND a.embedding IS NOT NULL
                    AND (a.line_end - a.line_start) >= 3
                    AND (a.embedding <=> b.embedding) <= $2
                  ORDER BY similarity DESC
                  LIMIT $3",
            )
            .bind(folder_ids)
            .bind(max_distance)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(na, fa, la, nb, fb, lb, sim)| {
                serde_json::json!({
                    "a": { "name": na, "file": fa, "line": la },
                    "b": { "name": nb, "file": fb, "line": lb },
                    "similarity": (sim * 10000.0).round() / 10000.0,
                })
            })
            .collect())
    }

    /// Abs paths of folders that still have embeddable nodes without an
    /// embedding. Used by the backfill endpoint to enqueue `EmbedNodes` for
    /// already-indexed folders (which a normal incremental scan won't revisit).
    pub async fn folders_with_pending_embeddings(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT f.abs_path
               FROM sensei.nodes n
               JOIN sensei.folders f ON f.id = n.folder_id
              WHERE n.embedding IS NULL
                AND n.file_path IS NOT NULL
                AND n.kind IN ('file','function','method','class','interface',
                               'type','const','enum','enum_variant','section',
                               'struct','component','hook','doc','extension')",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    pub async fn get_nodes_by_file(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<uuid::Uuid>, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, parent_id, line_start FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2 ORDER BY line_start"
        ).bind(folder_id).bind(file_path).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, pid, ls)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "parent_id": pid, "line_start": ls })
        }).collect())
    }

    pub async fn delete_nodes_by_folder(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1")
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn update_node_community(
        &self,
        node_id: &uuid::Uuid,
        community_id: i32,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.nodes SET community_id = $2, modified_at = now() WHERE id = $1",
        )
        .bind(node_id)
        .bind(community_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Edges ────────────────────────────────────────────────────────

    /// Insert (or upsert) an edge (D1). Edges carry an identity via two partial
    /// unique indexes, so a repeated identical insert returns the SAME row
    /// instead of duplicating. Branches on `target_id`: a resolved edge is keyed
    /// by its target node; an unresolved edge by `(target_name, target_file)`.
    /// `DO UPDATE SET modified_at = now()` (not `DO NOTHING`) so `RETURNING id`
    /// is always the surviving row's id.
    pub async fn insert_edge(
        &self,
        folder_id: &uuid::Uuid,
        source_id: &uuid::Uuid,
        target_id: Option<&uuid::Uuid>,
        target_name: Option<&str>,
        target_file: Option<&str>,
        kind: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = if let Some(tid) = target_id {
            sqlx_core::query_as::query_as(
                "INSERT INTO sensei.edges(folder_id, source_id, target_id, kind)
                 VALUES($1, $2, $3, $4::sensei.edge_kind)
                 ON CONFLICT (folder_id, source_id, target_id, kind) WHERE target_id IS NOT NULL
                   DO UPDATE SET modified_at = now()
                 RETURNING id",
            )
            .bind(folder_id)
            .bind(source_id)
            .bind(tid)
            .bind(kind)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx_core::query_as::query_as(
                "INSERT INTO sensei.edges(folder_id, source_id, target_name, target_file, kind)
                 VALUES($1, $2, $3, $4, $5::sensei.edge_kind)
                 ON CONFLICT (folder_id, source_id, target_name, target_file, kind) WHERE target_id IS NULL
                   DO UPDATE SET modified_at = now()
                 RETURNING id"
            ).bind(folder_id).bind(source_id).bind(target_name).bind(target_file).bind(kind)
                .fetch_one(&self.pool).await.map_err(|e| e.to_string())?
        };
        Ok(row.0)
    }

    pub async fn get_callers(
        &self,
        node_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT e.id, e.source_id, e.kind::text FROM sensei.edges e WHERE e.target_id = $1 AND e.kind = 'calls'::sensei.edge_kind"
        ).bind(node_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, kind)| {
            serde_json::json!({ "edge_id": id, "caller_id": src, "kind": kind })
        }).collect())
    }

    pub async fn get_callees(
        &self,
        node_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, Option<String>, String)> = sqlx_core::query_as::query_as(
            "SELECT e.id, e.target_id, e.target_name, e.kind::text FROM sensei.edges e WHERE e.source_id = $1 AND e.kind = 'calls'::sensei.edge_kind"
        ).bind(node_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, tgt, name, kind)| {
            serde_json::json!({ "edge_id": id, "callee_id": tgt, "callee_name": name, "kind": kind })
        }).collect())
    }

    /// Promote an unresolved edge to a resolved `target_id` (D1) — conflict-safe
    /// against `edges_unique_resolved`. If a resolved edge with the same
    /// `(folder_id, source_id, target_id, kind)` already exists, updating this
    /// row into it would violate the unique index; instead we MERGE — the UPDATE
    /// is guarded by a `NOT EXISTS`, and when it changes 0 rows (a dup exists, or
    /// the edge is already gone) we delete this now-redundant unresolved edge.
    ///
    /// The guard-then-delete is not one transaction, which is safe under the
    /// single-writer-per-folder invariant (W5/D6e): a folder's graph writes run as
    /// one barrier task at a time and the unique index is folder-scoped — so no
    /// concurrent resolve can race the `NOT EXISTS`.
    pub async fn resolve_edge(
        &self,
        edge_id: &uuid::Uuid,
        target_id: &uuid::Uuid,
    ) -> Result<(), String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.edges e
                SET target_id = $2, modified_at = now()
              WHERE e.id = $1
                AND NOT EXISTS (
                    SELECT 1 FROM sensei.edges d
                     WHERE d.folder_id = e.folder_id
                       AND d.source_id = e.source_id
                       AND d.target_id = $2
                       AND d.kind = e.kind
                       AND d.id <> e.id)",
        )
        .bind(edge_id)
        .bind(target_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if res.rows_affected() == 0 {
            // A resolved edge to the same target already exists (or this edge is
            // already gone): this unresolved edge is redundant — drop it so the
            // graph converges to the single resolved edge.
            sqlx_core::query::query("DELETE FROM sensei.edges WHERE id = $1")
                .bind(edge_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Replace a folder's entire edge set of one `kind` with `edges`, in ONE
    /// transaction (D2): DELETE every edge of that kind for the folder, then
    /// insert the current set. This makes a derived kind (e.g. `covers`) a pure
    /// function of the current tree — stale relations vanish instead of
    /// accumulating — and the single transaction means a crash can't leave the
    /// folder with a half-replaced (or empty) set: it either fully commits the
    /// new set or rolls back to the old one. Idempotent: re-running with the same
    /// set yields the same rows (the per-edge `ON CONFLICT` also absorbs a
    /// duplicate pair within the input set).
    pub async fn replace_edges_of_kind(
        &self,
        folder_id: &uuid::Uuid,
        kind: &str,
        edges: &[EdgeSpec],
    ) -> Result<(), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "DELETE FROM sensei.edges WHERE folder_id = $1 AND kind = $2::sensei.edge_kind",
        )
        .bind(folder_id)
        .bind(kind)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        for e in edges {
            if let Some(tid) = e.target_id {
                sqlx_core::query::query(
                    "INSERT INTO sensei.edges(folder_id, source_id, target_id, kind)
                     VALUES($1, $2, $3, $4::sensei.edge_kind)
                     ON CONFLICT (folder_id, source_id, target_id, kind) WHERE target_id IS NOT NULL
                       DO UPDATE SET modified_at = now()",
                )
                .bind(folder_id)
                .bind(e.source_id)
                .bind(tid)
                .bind(kind)
                .execute(&mut *tx)
                .await
                .map_err(|e2| e2.to_string())?;
            } else {
                sqlx_core::query::query(
                    "INSERT INTO sensei.edges(folder_id, source_id, target_name, target_file, kind)
                     VALUES($1, $2, $3, $4, $5::sensei.edge_kind)
                     ON CONFLICT (folder_id, source_id, target_name, target_file, kind) WHERE target_id IS NULL
                       DO UPDATE SET modified_at = now()"
                ).bind(folder_id).bind(e.source_id).bind(e.target_name.as_deref()).bind(e.target_file.as_deref()).bind(kind)
                    .execute(&mut *tx).await.map_err(|e2| e2.to_string())?;
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Collect unresolved reference stubs that nothing points at any more.
    ///
    /// A stub is a node with NO `file_path` and a kind other than
    /// `lib_symbol`/`lib_package` — the `unknown` locality in
    /// `sensei.graph_nodes`. It is minted by `upsert_node_by_fqn` when a reference
    /// resolves to a name no definition backs, and until now nothing could remove
    /// one: `prune_file_nodes` filters `file_path = $2`, which no stub can match.
    /// 84,446 accumulated. That absence is also why "stub count → 0" could not be
    /// driven by fixing the parsers alone — they stopped CREATING stubs, but the
    /// existing rows had no exit.
    ///
    /// TWO GUARDS, both load-bearing:
    ///
    /// * **Still referenced.** A stub with any edge is evidence that a reference
    ///   exists, even though its target is unknown. Dropping it would silently
    ///   lose the reference; it becomes collectable once the referencing file is
    ///   reindexed (`delete_edges_from_sources` drops the old edge, and the fixed
    ///   resolvers do not create a replacement). Measured 2026-09-01: 27,740 of
    ///   84,446 are already edge-free, the rest convert as their callers reindex.
    ///
    /// * **Has children.** `nodes.parent_id` cascades on delete, and live there are
    ///   42 stub parents carrying 574 REAL internal method nodes with file paths.
    ///   An unguarded delete would destroy all 574. The stub parent is wrong, but
    ///   it is load-bearing until its children are re-parented.
    ///
    /// Folder-scoped and idempotent — 84ms on this repo's largest folder (141,186
    /// nodes). Returns rows deleted.
    pub async fn prune_orphan_stubs(&self, folder_id: &uuid::Uuid) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.nodes n
              WHERE n.folder_id = $1
                AND n.file_path IS NULL
                AND n.kind NOT IN ('lib_symbol'::sensei.node_kind, 'lib_package'::sensei.node_kind)
                AND NOT EXISTS (
                    SELECT 1 FROM sensei.edges e
                     WHERE e.target_id = n.id OR e.source_id = n.id)
                AND NOT EXISTS (
                    SELECT 1 FROM sensei.nodes c WHERE c.parent_id = n.id)",
        )
        .bind(folder_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("prune_orphan_stubs: {e}"))?;

        // Clean up after ourselves. Deleting nodes here happens OUTSIDE the detect
        // transaction, so a community whose every member was an orphan stub is left
        // as a row describing nothing — measured 27,693 of them immediately after
        // the first GC pass, and `list_communities` / the Atlas communities/info
        // endpoint read them as phantom communities with a stale node_count. A
        // derived row with nothing left to describe is garbage by the same rule as
        // the stubs. `description` is not at risk: an emptied community has no
        // cluster left to caption.
        if res.rows_affected() > 0
            && let Err(e) = sqlx_core::query::query(
                "DELETE FROM inference.communities c
                  WHERE c.folder_id = $1
                    AND NOT EXISTS (
                        SELECT 1 FROM sensei.nodes n
                         WHERE n.folder_id = c.folder_id
                           AND n.community_id = c.community_id)",
            )
            .bind(folder_id)
            .execute(&self.pool)
            .await
        {
            // Non-fatal: the next detect replaces the folder's community set
            // anyway. Reclaiming garbage must not fail the caller.
            tracing::warn!(error = %e, "prune_orphan_stubs: emptied-community cleanup failed");
        }
        Ok(res.rows_affected())
    }

    /// Prune a file's nodes that vanished from the latest parse (D3 upsert-then-
    /// prune): every node for `(folder, file_path)` whose id is NOT in `kept_ids`.
    /// First unresolve inbound edges pointing at them (clear `target_id`, KEEP
    /// `target_name` as an honest unresolved residual — the caller re-emits a
    /// resolved FQN edge when it is next processed, and a full reindex heals it;
    /// Phase 7.1 retired the `resolve_edges` re-point pass), then delete the nodes
    /// (their out-edges cascade via the `source_id` FK). One transaction. Returns
    /// nodes pruned. An empty `kept_ids` prunes ALL of the file's nodes.
    pub async fn prune_file_nodes(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
        kept_ids: &[uuid::Uuid],
    ) -> Result<u64, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "UPDATE sensei.edges SET target_id = NULL, modified_at = now()
              WHERE folder_id = $1
                AND target_name IS NOT NULL
                AND target_id IN (
                    SELECT id FROM sensei.nodes
                     WHERE folder_id = $1 AND file_path = $2 AND id <> ALL($3))",
        )
        .bind(folder_id)
        .bind(file_path)
        .bind(kept_ids)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2 AND id <> ALL($3)",
        )
        .bind(folder_id)
        .bind(file_path)
        .bind(kept_ids)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Delete every out-edge sourced from `source_ids` in a folder (D3 per-file
    /// reconcile). A symbol that SURVIVES a re-index keeps its node id, so its
    /// stale out-edges (e.g. a call it no longer makes) aren't cascade-deleted —
    /// clear them so the caller can re-insert the current set (replace, not
    /// append). Returns rows deleted; an empty `source_ids` is a no-op.
    pub async fn delete_edges_from_sources(
        &self,
        folder_id: &uuid::Uuid,
        source_ids: &[uuid::Uuid],
    ) -> Result<u64, String> {
        if source_ids.is_empty() {
            return Ok(0);
        }
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.edges WHERE folder_id = $1 AND source_id = ANY($2)",
        )
        .bind(folder_id)
        .bind(source_ids)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Un-resolve edges that point INTO a file's nodes: clear `target_id` while
    /// keeping `target_name`. Called before re-indexing a changed file so the
    /// inbound cross-file edges survive (they'd otherwise be cascade-deleted when
    /// the target nodes are dropped). They become an honest unresolved residual,
    /// re-pointed when the calling file is next processed (FQN edges resolve at
    /// emit — Phase 7.1 retired the resolve_edges pass). Returns edges un-resolved.
    pub async fn unresolve_edges_to_file(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.edges SET target_id = NULL, modified_at = now()
              WHERE folder_id = $1
                AND target_id IN (SELECT id FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2)
                AND target_name IS NOT NULL"
        ).bind(folder_id).bind(file_path).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn get_edges_by_kind(
        &self,
        folder_id: &uuid::Uuid,
        kind: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.get_edges_scoped(std::slice::from_ref(folder_id), kind).await
    }

    // ── View-based graph queries ────────────────────────────────────

    /// Where a symbol is DEFINED within a scope — the lookup that separates
    /// "no such symbol" from "symbol with no callers". Returns one entry per
    /// definition site (a name can be defined in several folders of a
    /// monorepo), empty only when the name genuinely is not in the graph.
    ///
    /// Stubs are excluded (`file_path IS NOT NULL`): an unresolved reference
    /// stub carries the name but is not a definition, so counting it as one
    /// would report `found` for a symbol that was only ever mentioned.
    pub async fn symbol_definitions(
        &self,
        folder_ids: &[uuid::Uuid],
        name: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT kind::text, file_path, line_start
               FROM sensei.nodes
              WHERE folder_id = ANY($1) AND name = $2 AND file_path IS NOT NULL
              ORDER BY file_path, line_start LIMIT 20",
        )
        .bind(folder_ids)
        .bind(name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(kind, file, line)| {
                serde_json::json!({ "kind": kind, "file_path": file, "line_start": line })
            })
            .collect())
    }

    /// Resolution coverage as `(resolved, unresolved)` for one symbol's `calls`
    /// edges. This is the number that tells a caller how much to trust a
    /// caller/callee list: an unresolved count above zero means the graph knows
    /// a call happened but could not place both ends, so the list is incomplete
    /// and grep is still worth running.
    ///
    /// Counted with its own query rather than tallied from the returned list,
    /// because those lists are `LIMIT 100` — deriving coverage from a truncated
    /// list would under-report exactly when completeness matters most.
    pub async fn call_coverage(
        &self,
        folder_ids: &[uuid::Uuid],
        name: &str,
        direction: CallDirection,
    ) -> Result<(i64, i64), String> {
        // The filter column comes from a closed enum, never from caller input,
        // so this stays static SQL with the name passed as a bind parameter.
        let sql = match direction {
            CallDirection::Incoming => {
                "SELECT count(target_id), count(*) - count(target_id)
                   FROM sensei.call_graph
                  WHERE folder_id = ANY($1) AND target_symbol = $2 AND edge_kind = 'calls'"
            }
            CallDirection::Outgoing => {
                "SELECT count(target_id), count(*) - count(target_id)
                   FROM sensei.call_graph
                  WHERE folder_id = ANY($1) AND source_name = $2 AND edge_kind = 'calls'"
            }
        };
        let row: (i64, i64) = sqlx_core::query_as::query_as(sql)
            .bind(folder_ids)
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Find callers of a function by name via the call_graph view.
    /// `scope` is resolved via [`scope_folder_ids`]: a project name/UUID expands
    /// to all of that project's folders; a bare folder name falls back to just
    /// that folder.
    ///
    /// Filters `target_symbol`, NOT `target_name`. `target_name` is `tgt.name`
    /// off the view's LEFT JOIN and is therefore NULL for every unresolved
    /// edge, so the old filter silently dropped 117,201 of 335,756 `calls`
    /// edges and returned an empty list for 8,680 symbol names that had
    /// callers. See the `target_symbol` column comment.
    pub async fn get_callers_by_name(
        &self,
        scope: &str,
        target: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let folder_ids = self.scope_folder_ids(scope).await?;
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(String, String, String, Option<i32>, bool)> = sqlx_core::query_as::query_as(
            "SELECT source_name, source_kind::text, source_file, source_line, target_id IS NOT NULL
               FROM sensei.call_graph
              WHERE folder_id = ANY($1) AND target_symbol = $2 AND edge_kind = 'calls'
              ORDER BY source_file, source_line LIMIT 100",
        )
        .bind(&folder_ids[..])
        .bind(target)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, kind, file, line, resolved)| {
            serde_json::json!({ "name": name, "kind": kind, "file_path": file, "line_start": line, "resolved": resolved })
        }).collect())
    }

    /// Find callees of a function by name via the call_graph view.
    /// `scope` is resolved via [`scope_folder_ids`]: a project name/UUID expands
    /// to all of that project's folders; a bare folder name falls back to just
    /// that folder.
    pub async fn get_callees_by_name(
        &self,
        scope: &str,
        source: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let folder_ids = self.scope_folder_ids(scope).await?;
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        // `target_symbol` is the view's coalesce of the resolved and unresolved
        // name, so the display name is no longer stitched together here — one
        // owner for that rule.
        //
        // `locality` is READ FROM `sensei.graph_nodes`, never recomputed here. A
        // second SQL copy of that three-branch judgement is precisely what the
        // graph_nodes comment warns about, so internal/external come from the
        // owning view and the only thing this query decides is the edge-level
        // fact the owner cannot know: an unresolved edge has NO target node, so
        // there is no row to classify and it is `unknown`. That is what the
        // COALESCE means — "no target node", not a reimplementation of the rule.
        let rows: Vec<(String, Option<String>, Option<String>, Option<i32>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT cg.target_symbol, cg.target_kind::text, cg.target_file, cg.target_line,
                        coalesce(gn.locality, 'unknown')
                   FROM sensei.call_graph        cg
                   LEFT JOIN sensei.graph_nodes  gn ON gn.id = cg.target_id
                  WHERE cg.folder_id = ANY($1) AND cg.source_name = $2
                    AND cg.edge_kind = 'calls' AND cg.target_symbol IS NOT NULL
                  ORDER BY cg.target_file, cg.target_line LIMIT 100",
            )
            .bind(&folder_ids[..])
            .bind(source)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, kind, file, line, locality)| {
            serde_json::json!({ "name": name, "kind": kind, "file_path": file, "line_start": line, "locality": locality })
        }).collect())
    }

    /// Get files matching a tag via the file_tags view.
    pub async fn get_files_by_tag(
        &self,
        folder_name: &str,
        tag: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Vec<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, file_path, tags FROM sensei.file_tags
              WHERE folder = $1 AND $2 = ANY(tags)
              ORDER BY file_path LIMIT 200",
        )
        .bind(folder_name)
        .bind(tag)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(id, fp, tags)| serde_json::json!({ "id": id, "file_path": fp, "tags": tags }))
            .collect())
    }

    /// Get doc coverage with drift detection via the doc_coverage view.
    pub async fn get_doc_drift(&self, folder_name: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, String, String, bool)> = sqlx_core::query_as::query_as(
            "SELECT doc_name, doc_file, code_name, code_file, drifted
               FROM sensei.doc_coverage
              WHERE folder = $1
              ORDER BY drifted DESC, doc_file LIMIT 200",
        )
        .bind(folder_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(doc_name, doc_file, code_name, code_file, drifted)| {
            serde_json::json!({ "doc": doc_name, "docFile": doc_file, "code": code_name, "codeFile": code_file, "drifted": drifted })
        }).collect())
    }

    /// Count all edges across multiple folders (project-scoped variant).
    pub async fn count_edges_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM sensei.edges WHERE folder_id = ANY($1)",
        )
        .bind(folder_ids)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Count all edges for a folder.
    pub async fn count_edges(&self, folder_id: &uuid::Uuid) -> Result<i64, String> {
        self.count_edges_scoped(&[*folder_id]).await
    }

    /// Delete nodes whose file_path starts with a given prefix (for folder deletion).
    pub async fn delete_nodes_by_path_prefix(
        &self,
        folder_id: &uuid::Uuid,
        prefix: &str,
    ) -> Result<u64, String> {
        let result = sqlx_core::query::query(
            "DELETE FROM sensei.nodes WHERE folder_id = $1 AND file_path LIKE $2 || '%'",
        )
        .bind(folder_id)
        .bind(prefix)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(result.rows_affected())
    }

    pub async fn upsert_community(
        &self,
        folder_id: &uuid::Uuid,
        community_id: i32,
        label: &str,
        node_count: i32,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.communities(folder_id, community_id, label, node_count)
             VALUES($1, $2, $3, $4)
             ON CONFLICT(folder_id, community_id) DO UPDATE SET label = EXCLUDED.label, node_count = EXCLUDED.node_count, modified_at = now()
             RETURNING id"
        ).bind(folder_id).bind(community_id).bind(label).bind(node_count)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Set `is_test` for every node of a file (folder-scoped) to the file's
    /// test-ness (`languages::is_test_path`). `is_test` is a FILE-level property —
    /// all of a file's nodes (file/symbol/section/rationale/fqn-def) share it — so
    /// this runs once per file after emit rather than threading a param through
    /// every upsert. Guarded by `IS DISTINCT FROM` so a steady-state re-scan
    /// changes 0 rows (cheap) while a test↔prod rename flips them. `lib_symbol`/
    /// `lib_package` nodes (file_path NULL) are never matched (external deps aren't
    /// test). Returns rows changed.
    /// Correct the language stamp for one file's nodes.
    ///
    /// `upsert_node_ex` derives `language` from the file EXTENSION at write time,
    /// which is right for code and cannot work for `.txt`: `docs/llms/index.txt` is
    /// markdown (rokkit's corpus — headings, tables, fenced code) while
    /// `docs/License.txt` is prose, and only the CONTENT distinguishes them.
    ///
    /// A post-write correction rather than a new parameter on every node write,
    /// mirroring [`Self::set_nodes_is_test_for_file`]: the extension remains the
    /// default and the doc path, which has the content, overrides it. Threading a
    /// language through `upsert_node_ex` would touch every caller for one file type.
    ///
    /// Returns rows changed. The `IS DISTINCT FROM` guard makes a re-index a no-op
    /// rather than a write, so this does not churn `modified_at`.
    pub async fn set_nodes_language_for_file(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
        language: &str,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.nodes SET language = $3, modified_at = now()
              WHERE folder_id = $1 AND file_path = $2 AND language IS DISTINCT FROM $3",
        )
        .bind(folder_id)
        .bind(file_path)
        .bind(language)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("set_nodes_language_for_file: {e}"))?;
        Ok(res.rows_affected())
    }

    pub async fn set_nodes_is_test_for_file(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
        is_test: bool,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.nodes SET is_test = $3, modified_at = now()
              WHERE folder_id = $1 AND file_path = $2 AND is_test IS DISTINCT FROM $3",
        )
        .bind(folder_id)
        .bind(file_path)
        .bind(is_test)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Replace a folder's ENTIRE community assignment in one transaction (D4):
    /// delete its community rows, clear every node's `community_id`, then insert
    /// the new communities and set their members' `community_id`. This makes
    /// `inference.communities` + `nodes.community_id` a pure function of the
    /// current graph — no stale community rows, no stranded/orphaned
    /// `community_id`s (invariant 5) — and atomic (a crash can't leave a
    /// half-assigned folder). An empty `communities` just clears the folder.
    pub async fn replace_communities_for_folder(
        &self,
        folder_id: &uuid::Uuid,
        communities: &[CommunityAssignment],
    ) -> Result<u64, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        // Drop only the communities that VANISHED. This used to delete every row for
        // the folder and re-insert with `description = NULL`, so each re-detect
        // discarded model-authored prose — and `enrich_community_descriptions` is
        // capped at 25 communities per folder, so it could not replace what the
        // refresh wiped. A steady-state re-scan therefore burned model calls
        // regenerating text it had just thrown away.
        let surviving: Vec<i32> = communities.iter().map(|c| c.community_id).collect();
        sqlx_core::query::query(
            "DELETE FROM inference.communities WHERE folder_id = $1 AND community_id <> ALL($2)",
        )
        .bind(folder_id)
        .bind(&surviving)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        // Assign members FIRST, each guarded by `IS DISTINCT FROM`, THEN NULL only
        // the leftovers (nodes that had a community but are in none of the new ones)
        // — instead of null-all-then-reset. `community_id` is deterministic for an
        // identical graph (indexer::community, invariant 2), so an unchanged
        // re-detect rewrites 0 node rows. The committed state is identical to the
        // old clear-all-then-reset — a pure function of the graph (invariant 5),
        // atomic in one tx — but a steady-state re-scan no longer rewrites every
        // community node twice, which (like the unguarded degree recompute) piled
        // dead tuples into sensei.nodes on every DetectCommunities pass. Returns the
        // node rows actually changed.
        let mut changed: u64 = 0;
        let mut all_members: Vec<uuid::Uuid> = Vec::new();
        for c in communities {
            // Authoritative write for the DERIVED columns. `description` is not
            // derived — it costs a model call — so it is written once by
            // `enrich_community_descriptions` (off-barrier) and preserved here.
            //
            // Preserved only when this is demonstrably the SAME cluster, evidenced by
            // an identical hub set. `community_id` is positional (rank+1), so on a
            // changed graph id 3 can be an entirely different cluster; keeping its old
            // prose would caption the wrong thing. Differing hubs → the description is
            // discarded and enrichment regenerates it. Fails closed on doubt rather
            // than mislabelling.
            sqlx_core::query::query(
                "INSERT INTO inference.communities(folder_id, community_id, label, node_count, god_node_ids, description, props)
                 VALUES($1, $2, $3, $4, $5, NULL, '{\"source\":\"null\"}'::jsonb)
                 ON CONFLICT (folder_id, community_id) DO UPDATE
                   SET label        = EXCLUDED.label,
                       node_count   = EXCLUDED.node_count,
                       god_node_ids = EXCLUDED.god_node_ids,
                       description  = CASE WHEN communities.god_node_ids = EXCLUDED.god_node_ids
                                           THEN communities.description END,
                       props        = CASE WHEN communities.god_node_ids = EXCLUDED.god_node_ids
                                           THEN communities.props
                                           ELSE '{\"source\":\"null\"}'::jsonb END,
                       computed_at  = now(),
                       modified_at  = now()"
            ).bind(folder_id).bind(c.community_id).bind(&c.label).bind(c.member_node_ids.len() as i32)
                .bind(&c.god_node_ids)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            if !c.member_node_ids.is_empty() {
                let res = sqlx_core::query::query(
                    "UPDATE sensei.nodes SET community_id = $2, modified_at = now()
                      WHERE folder_id = $1 AND id = ANY($3) AND community_id IS DISTINCT FROM $2",
                )
                .bind(folder_id)
                .bind(c.community_id)
                .bind(&c.member_node_ids)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                changed += res.rows_affected();
                all_members.extend_from_slice(&c.member_node_ids);
            }
        }
        // Clear any node still carrying a community_id but no longer a member of any
        // community (removed/renamed symbols, or an empty `communities` that clears
        // the whole folder). `id <> ALL('{}')` is TRUE for every row, so an empty
        // member set nulls all assigned nodes — matching the old clear-all.
        let cleared = sqlx_core::query::query(
            "UPDATE sensei.nodes SET community_id = NULL, modified_at = now()
              WHERE folder_id = $1 AND community_id IS NOT NULL AND id <> ALL($2)",
        )
        .bind(folder_id)
        .bind(&all_members)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        changed += cleared.rows_affected();
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(changed)
    }

    pub async fn list_communities(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, i32)> = sqlx_core::query_as::query_as(
            "SELECT id, label, node_count FROM inference.communities WHERE folder_id = $1 ORDER BY node_count DESC"
        ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, label, count)| {
            serde_json::json!({ "id": id, "label": label, "node_count": count })
        }).collect())
    }

    /// Communities across ALL folders of a project scope (one query). Communities
    /// are stored per-folder and the repo root usually owns them, so a caller must
    /// aggregate over every scope folder — a single-folder lookup (a leaf) misses
    /// them (the #G5a `get_communities` bug).
    pub async fn list_communities_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
    ) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(uuid::Uuid, String, i32)> = sqlx_core::query_as::query_as(
            "SELECT id, label, node_count FROM inference.communities WHERE folder_id = ANY($1) ORDER BY node_count DESC"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, label, count)| {
            serde_json::json!({ "id": id, "label": label, "node_count": count })
        }).collect())
    }

    /// Communities across a project scope with LIVE membership counts (7.3): the
    /// `node_count` is computed from the real `nodes.community_id` join, not the
    /// denormalized `communities.node_count` — so the overview reflects the
    /// current graph (a node whose community changed since the last detect is
    /// counted where it actually is now). Also carries `god_node_ids`. Ordered by
    /// live count desc. This is what turns the flat "scattered circles" overview
    /// into one sized by real per-community membership.
    pub async fn list_communities_live_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
    ) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(uuid::Uuid, Option<String>, i64, Vec<uuid::Uuid>)> =
            sqlx_core::query_as::query_as(
                "SELECT c.id, c.label, count(n.id) AS live_count, c.god_node_ids
               FROM inference.communities c
               LEFT JOIN sensei.nodes n
                 ON n.folder_id = c.folder_id AND n.community_id = c.community_id
              WHERE c.folder_id = ANY($1)
              GROUP BY c.id, c.label, c.god_node_ids
              ORDER BY live_count DESC, c.id",
            )
            .bind(folder_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, label, count, gods)| {
            serde_json::json!({ "id": id, "label": label.unwrap_or_default(), "node_count": count, "god_node_ids": gods })
        }).collect())
    }

    /// The folder's communities with their `god_node_ids`, largest first — the
    /// input to description enrichment (D4.5). Bounded by `limit` so a huge cold
    /// repo enriches only its most significant clusters per detect run.
    pub async fn list_communities_with_god_nodes(
        &self,
        folder_id: &uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<(i32, String, i32, Vec<uuid::Uuid>)>, String> {
        let rows: Vec<(i32, Option<String>, i32, Vec<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT community_id, label, node_count, god_node_ids
               FROM inference.communities
              WHERE folder_id = $1
              ORDER BY node_count DESC, community_id
              LIMIT $2",
        )
        .bind(folder_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(cid, label, n, gods)| (cid, label.unwrap_or_default(), n, gods))
            .collect())
    }

    /// `(id, name, kind)` for a set of node ids — builds community description
    /// facts from the god-node hubs. Empty input is a no-op.
    pub async fn get_node_name_kind(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<Vec<(uuid::Uuid, String, String)>, String> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(uuid::Uuid, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, name, kind::text FROM sensei.nodes WHERE id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Stamp a community's model-authored `description` + its provenance
    /// (`props.source`), replacing the honest-empty placeholder from the
    /// authoritative write (D4.5). Only called on a successful narration-cache
    /// generation — a failure leaves the honest-empty NULL/`'null'` as written.
    pub async fn set_community_description(
        &self,
        folder_id: &uuid::Uuid,
        community_id: i32,
        description: &str,
        source: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.communities
                SET description = $3,
                    props = props || jsonb_build_object('source', $4::text),
                    modified_at = now()
              WHERE folder_id = $1 AND community_id = $2",
        )
        .bind(folder_id)
        .bind(community_id)
        .bind(description)
        .bind(source)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Reasoning Traces (inference) ─────────────────────────────────

    /// Upsert the current code-symbol names into the global `symbol_names`
    /// registry (monotonic — never prunes). The doc-drift scan reads this history
    /// to tell a REMOVED symbol (real drift) from an identifier that was never a
    /// symbol (prose/config — not drift). Returns the number of names recorded.
    pub async fn record_symbol_names(&self) -> Result<u64, String> {
        let sql = format!(
            "INSERT INTO sensei.symbol_names (name)
             SELECT DISTINCT name FROM sensei.nodes
              WHERE kind IN ({kinds}) AND name <> ''
             ON CONFLICT (name) DO UPDATE SET last_seen = now()",
            kinds = Self::DRIFT_SYMBOL_KINDS
        );
        let res =
            sqlx_core::query::query(&sql).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn scan_project_doc_drift(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        use crate::analysis::doc_drift::{
            extract_identifier_mentions, extract_mention_from_detail, is_broken_drift,
        };

        // 1. Load every doc node in this project along with its folder id
        //    and the absolute path so we can read the file content off disk.
        //    `n.content` is intentionally not stored for doc nodes today —
        //    the file remains the source of truth — so this scan reads the
        //    on-disk content each pass. Capped at 500 docs per run so a
        //    heavy project doesn't stall the request.
        #[allow(clippy::type_complexity)]
        let doc_rows: Vec<(uuid::Uuid, uuid::Uuid, String, String)> =
            sqlx_core::query_as::query_as(
                "SELECT n.id, n.folder_id, f.abs_path, n.file_path
               FROM sensei.nodes n
               JOIN sensei.folders f ON f.id = n.folder_id
              WHERE f.project_id = $1
                AND n.kind = 'doc'
              LIMIT 500",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let scanned_docs = doc_rows.len();

        // 2. Load known code identifier names into a set once, so the
        //    per-mention lookup is a HashSet::contains (cheap) rather than a DB
        //    round-trip per mention. Two deliberate widenings vs the original
        //    (which over-fired ~all mentions as broken):
        //    - ALL code-symbol kinds, not just 7 — the old whitelist predated
        //      struct/enum/hook/component/extension, so real project symbols of
        //      those kinds were wrongly flagged.
        //    - GLOBAL, not per-project — a doc legitimately references its
        //      indexed dependencies' symbols (e.g. a rokkit component). Those
        //      resolve to a real node in another project, so their mention is
        //      not drift. (Cross-project name collisions can mask a removed
        //      same-named symbol — an accepted precision tradeoff to kill the
        //      dependency-reference false positives.)
        let code_names: Vec<(String,)> = sqlx_core::query_as::query_as(&format!(
            "SELECT DISTINCT name FROM sensei.nodes WHERE kind IN ({kinds})",
            kinds = Self::DRIFT_SYMBOL_KINDS
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let mut known: std::collections::HashSet<String> =
            code_names.into_iter().map(|(n,)| n).collect();

        // Also treat DB schema identifiers as known: docs legitimately reference
        // table / column / view names and enum labels (`project_id`, `created_at`,
        // `tool_usage_stats`, `assistant_family`), which are real identifiers, not
        // drift — but they are never indexed as code-symbol nodes. Own-schemas
        // only (skip pg_catalog / information_schema noise).
        let schema_names: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT table_name AS name FROM information_schema.tables
              WHERE table_schema IN ('sensei','inference','activity','governance','staging')
             UNION
             SELECT column_name FROM information_schema.columns
              WHERE table_schema IN ('sensei','inference','activity','governance','staging')
             UNION
             SELECT e.enumlabel
               FROM pg_enum e JOIN pg_type t ON t.oid = e.enumtypid
               JOIN pg_namespace ns ON ns.oid = t.typnamespace
              WHERE ns.nspname IN ('sensei','inference','activity','governance')",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        known.extend(schema_names.into_iter().map(|(n,)| n));

        // 2b. Refresh the symbol-name history (monotonic upsert of the current
        //     symbols) so a symbol removed since a prior scan stays "known to have
        //     existed", then load the full history. The drift gate flags a mention
        //     ONLY when it was a real symbol (in `ever_symbols`) and no longer
        //     resolves (`known`) — so identifiers that were never symbols (enum
        //     variants, serde camelCase fields, string-dispatched tool names) are
        //     not drift. This is what removes the ~408 false positives.
        if let Err(e) = self.record_symbol_names().await {
            tracing::warn!(error = %e, "scan_project_doc_drift: record_symbol_names failed — history not refreshed this pass");
        }
        let ever_rows: Vec<(String,)> =
            sqlx_core::query_as::query_as("SELECT name FROM sensei.symbol_names")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        let ever_symbols: std::collections::HashSet<String> =
            ever_rows.into_iter().map(|(n,)| n).collect();

        // 3. Fan the mentions out per doc, inserting `broken` drift rows for
        //    mentions that were a real symbol and no longer resolve. We check for
        //    an existing broken row via a subquery to avoid duplicates.
        let mut new_broken: i64 = 0;
        for (doc_id, folder_id, abs_path, file_path) in &doc_rows {
            // Read the doc content off disk. Unreadable files (deleted,
            // permission denied) silently skip — we never fail the whole
            // scan for one bad file.
            let full_path = std::path::Path::new(abs_path).join(file_path);
            let content = match std::fs::read_to_string(&full_path) {
                Ok(text) => text,
                Err(_) => continue,
            };
            let mentions = extract_identifier_mentions(&content);
            for mention in mentions {
                // Flag only names that WERE a real symbol and no longer resolve.
                if !is_broken_drift(&mention, &known, &ever_symbols) {
                    continue;
                }
                let detail = format!("Mentions `{mention}` which is not in the code.");
                // Skip if we already logged this same drift signal.
                let existing: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
                    "SELECT id FROM inference.drift_items
                      WHERE doc_node_id = $1 AND detail = $2 AND resolved_at IS NULL
                      LIMIT 1",
                )
                .bind(doc_id)
                .bind(&detail)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
                if existing.is_some() {
                    continue;
                }
                // The `doc_node_id -> code_node_id` invariant requires a code
                // node — the DDL enforces NOT NULL on the FK. Use the doc node
                // as a self-reference so the FK stays satisfied without a
                // dedicated "unresolved" sentinel. Callers rely on
                // `code_node_id` matching `doc_node_id` to mean "broken".
                sqlx_core::query::query(
                    "INSERT INTO inference.drift_items
                        (folder_id, doc_node_id, code_node_id, status, detail)
                     VALUES ($1, $2, $2, 'broken', $3)",
                )
                .bind(folder_id)
                .bind(doc_id)
                .bind(&detail)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
                new_broken += 1;
            }
        }

        // 4. Resolve any existing broken rows whose mention now RESOLVES —
        //    the doc got fixed or the code got added since the last scan.
        //    We re-parse each open row's detail to recover the mention name.
        let open_rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT di.id, di.detail
               FROM inference.drift_items di
               JOIN sensei.folders f ON f.id = di.folder_id
              WHERE f.project_id = $1
                AND di.status = 'broken'
                AND di.resolved_at IS NULL",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut resolved: i64 = 0;
        for (drift_id, detail) in open_rows {
            // Clear an open row once it's no longer drift: the code came back /
            // the doc was fixed (now in `known`) OR the mention was never a real
            // symbol (absent from history) — the false-positive backlog.
            if let Some(mention) = extract_mention_from_detail(&detail)
                && !is_broken_drift(&mention, &known, &ever_symbols)
            {
                sqlx_core::query::query(
                    "UPDATE inference.drift_items
                        SET status = 'current', resolved_at = now()
                      WHERE id = $1",
                )
                .bind(drift_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
                resolved += 1;
            }
        }

        Ok(serde_json::json!({
            "scannedDocs": scanned_docs,
            "newBroken":   new_broken,
            "resolved":    resolved,
        }))
    }

    // ── Service registry + per-project scoping (T2 Slice B) ──────────────

    /// Enforce one-node-one-owner: a `folder`-kind (structural) subfolder must
    /// not carry a node that the project's canonical ROOT owner
    /// (git/standalone/subtree) already holds. Deletes each such duplicate —
    /// TWIN-GUARDED by identical `kind` AND a path-suffix match on BOTH
    /// `file_path` and `name`: the root node's repo-relative value must end in
    /// the structural node's subfolder-relative value (`right(...)` suffix, no
    /// LIKE wildcard hazard). For code symbols the `name` suffix collapses to an
    /// exact match (symbol names have no `/`, so the `'/' ||` separator can't
    /// spuriously match); for `file`/`module` nodes (where `name` is itself a
    /// path) the suffix catches the differing subfolder prefix. A node held
    /// UNIQUELY under a structural folder is therefore never removed — only
    /// proven duplicates are. Self-heals the pre-fix double-index residue (#101:
    /// members promoted to second index owners on 2026-07-13) and, run every
    /// scan, prevents future accumulation. Scoped to `root_id`. Edges cascade
    /// with the deleted nodes. Returns rows pruned.
    pub async fn dedup_structural_folder_nodes(&self, root_id: &uuid::Uuid) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.nodes s
               USING sensei.folders sf
              WHERE s.folder_id = sf.id
                AND sf.kind IN ('folder'::sensei.folder_kind, 'workspace_member'::sensei.folder_kind)
                AND sf.root_id = $1
                AND EXISTS (
                  SELECT 1
                    FROM sensei.nodes g
                    JOIN sensei.folders gf ON gf.id = g.folder_id
                   WHERE gf.project_id = sf.project_id
                     AND gf.kind IN ('git'::sensei.folder_kind,
                                     'standalone'::sensei.folder_kind,
                                     'subtree'::sensei.folder_kind)
                     AND g.kind = s.kind
                     AND (g.name = s.name
                          OR right(g.name, char_length(s.name) + 1)
                             = ('/' || s.name))
                     AND (g.file_path = s.file_path
                          OR right(g.file_path, char_length(s.file_path) + 1)
                             = ('/' || s.file_path)))",
        )
        .bind(root_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("dedup_structural_folder_nodes: {e}"))?;
        Ok(res.rows_affected())
    }

    /// Populate framework-pattern tags on file nodes so `get_patterns` /
    /// `get_file_tags` return real files (they read `sensei.file_tags`, a view
    /// over `nodes.tags` for `kind='file'` — previously always empty).
    ///
    /// The signal is the classifier's own node kinds: a file is tagged with the
    /// framework kinds of the symbols it contains (`hook`, `component`). This
    /// reuses the existing per-node classification — no separate tagger — and
    /// recomputes the full set per file (so a file that loses its last hook is
    /// cleared), scoped to one watch root. Runs in the scan reconcile. Returns
    /// file nodes whose tags changed.
    pub async fn tag_file_nodes_by_framework_kind(
        &self,
        root_id: &uuid::Uuid,
    ) -> Result<u64, String> {
        // Tag each `file` node with the framework roles it plays, so `get_patterns`
        // / `get_file_tags` answer "which files are components / hooks / routes /
        // middleware". Two signals, merged into `tags` and recomputed each scan
        // (self-correcting — adds AND removes):
        //   • symbol-kind — the `hook`/`component` node-kinds the classifier emits
        //     for symbols the file contains.
        //   • file-role (path convention, per-framework) — a whole file that *is* a
        //     route or middleware. `route`/`middleware` aren't node kinds, they are
        //     file-level roles, so a path convention is the right per-adapter
        //     detector: SvelteKit `+page`/`+layout`/`+server`/`+error` + Next
        //     `page`/`route` → `route`; SvelteKit `hooks.{server,client}` + Next
        //     `middleware` → `middleware`.
        // A CTE computes the desired tag set once per file; only rows whose set
        // actually changes are written (idempotent, accurate `rows_affected`).
        let res = sqlx_core::query::query(
            r"WITH desired AS (
                SELECT fn.id,
                       COALESCE((
                         SELECT array_agg(DISTINCT tag ORDER BY tag) FROM (
                           SELECT s.kind::text AS tag
                             FROM sensei.nodes s
                            WHERE s.folder_id = fn.folder_id
                              AND s.file_path = fn.file_path
                              AND s.kind IN ('hook','component')
                           UNION ALL
                           SELECT 'route'
                            WHERE fn.file_path ~ '(^|/)\+(page|layout|server|error)\.'
                               OR fn.file_path ~ '(^|/)(page|route)\.(tsx?|jsx?)$'
                           UNION ALL
                           SELECT 'middleware'
                            WHERE fn.file_path ~ '(^|/)hooks\.(server|client)\.(tsx?|jsx?)$'
                               OR fn.file_path ~ '(^|/)hooks\.(tsx?|jsx?)$'
                               OR fn.file_path ~ '(^|/)middleware\.(tsx?|jsx?)$'
                         ) src
                       ), '{}') AS tags
                  FROM sensei.nodes fn
                  JOIN sensei.folders f ON f.id = fn.folder_id
                 WHERE f.root_id = $1
                   AND fn.kind = 'file'
              )
              UPDATE sensei.nodes n
                 SET tags = d.tags
                FROM desired d
               WHERE n.id = d.id
                 AND n.tags IS DISTINCT FROM d.tags",
        )
        .bind(root_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("tag_file_nodes_by_framework_kind: {e}"))?;
        Ok(res.rows_affected())
    }

    /// Search functions across multiple folders (project-scoped variant).
    pub async fn search_functions_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
        query: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, file_path, signature, line_start FROM sensei.nodes
             WHERE folder_id = ANY($1) AND kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
             AND file_path IS NOT NULL
             AND (name ILIKE '%' || $2 || '%' OR signature ILIKE '%' || $2 || '%')
             ORDER BY name LIMIT 50"
        ).bind(folder_ids).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, fp, sig, line)| {
            serde_json::json!({ "id": id, "name": name, "file_path": fp, "signature": sig, "line_start": line })
        }).collect())
    }

    /// Search types across multiple folders (project-scoped variant).
    pub async fn search_types_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
        query: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, file_path, line_start FROM sensei.nodes
             WHERE folder_id = ANY($1) AND kind IN ('class'::sensei.node_kind, 'struct'::sensei.node_kind, 'interface'::sensei.node_kind, 'enum'::sensei.node_kind, 'type'::sensei.node_kind)
             AND file_path IS NOT NULL
             AND name ILIKE '%' || $2 || '%'
             ORDER BY name LIMIT 50"
        ).bind(folder_ids).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, fp, line)| {
            serde_json::json!({ "id": id, "name": name, "file_path": fp, "line_start": line })
        }).collect())
    }

    /// Per-edge-kind resolution coverage for a scope: `(kind, resolved, total)`.
    ///
    /// This is how much the graph-navigation tools can actually be trusted, and
    /// until now nothing reported it — `get_project_summary` answered "10,614
    /// functions" while 43% of this project's `calls` edges were unresolved and
    /// `imports`/`extends`/`references` were at 0%, so a caller had no way to
    /// know a caller list might be partial or that a whole edge kind was empty.
    /// Surfacing it lets a reader decide to grep BEFORE trusting a lookup,
    /// instead of inferring it from a suspiciously short answer.
    pub async fn edge_resolution_by_kind(
        &self,
        folder_ids: &[uuid::Uuid],
    ) -> Result<Vec<(String, i64, i64)>, String> {
        let rows: Vec<(String, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT kind::text, count(target_id), count(*)
               FROM sensei.edges
              WHERE folder_id = ANY($1)
              GROUP BY kind
              ORDER BY count(*) DESC",
        )
        .bind(folder_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Count nodes by kind across multiple folders (project-scoped variant).
    pub async fn count_nodes_by_kind_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
    ) -> Result<std::collections::HashMap<String, i64>, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT kind::text, COUNT(*) FROM sensei.nodes WHERE folder_id = ANY($1) GROUP BY kind",
        )
        .bind(folder_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().collect())
    }

    /// Get all nodes across multiple folders (project-scoped variant).
    pub async fn get_nodes_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
    ) -> Result<Vec<serde_json::Value>, String> {
        // file_path is Option: reference stubs + lib_symbol nodes have none. The
        // whole-graph projection must decode them without erroring (they serialize
        // to a null file_path); NULLs sort last under ORDER BY file_path.
        // `fqn`/`resolved` are projected (7.2) so the Atlas can key symbols by
        // moniker and distinguish enriched defs from reference stubs. `fqn` is NULL
        // for pre-FQN/legacy rows; `resolved` is NOT NULL (defaults false).
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<uuid::Uuid>, Option<i32>, Option<i32>, Option<i32>, uuid::Uuid, Option<String>, Option<String>, bool, bool)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, file_path, parent_id, line_start, line_end, community_id, folder_id, language, fqn, resolved, is_test FROM sensei.nodes WHERE folder_id = ANY($1) ORDER BY file_path, line_start, parent_id, id"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, fp, pid, ls, le, community_id, folder_id, language, fqn, resolved, is_test)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "file_path": fp, "parent_id": pid, "line_start": ls, "line_end": le, "community_id": community_id, "folder_id": folder_id, "language": language, "fqn": fqn, "resolved": resolved, "is_test": is_test })
        }).collect())
    }

    /// Get edges by kind across multiple folders (project-scoped variant).
    pub async fn get_edges_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
        kind: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.get_edges_scoped_kinds(folder_ids, &[kind]).await
    }

    /// Get edges of ANY of `kinds` across multiple folders (7.1) — the graph
    /// layout set is `calls,imports,extends` (+`implements` once emitted), not the
    /// single `calls` the node view used to fetch. Each row carries its `kind` so
    /// the client can style/overlay per relationship type.
    pub async fn get_edges_scoped_kinds(
        &self,
        folder_ids: &[uuid::Uuid],
        kinds: &[&str],
    ) -> Result<Vec<serde_json::Value>, String> {
        let kinds_owned: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<uuid::Uuid>, Option<String>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, source_id, target_id, target_name, kind::text FROM sensei.edges
              WHERE folder_id = ANY($1) AND kind::text = ANY($2)",
            )
            .bind(folder_ids)
            .bind(&kinds_owned)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, tgt, name, kind)| {
            serde_json::json!({ "id": id, "source_id": src, "target_id": tgt, "target_name": name, "kind": kind })
        }).collect())
    }

    // ── Gateway fallback chains + role assignments ──────────────────────
    //
    // Reads and writes for the Model Assignments wizard step. The DDL
    // model puts an optional `role` on `gateway.fallback_chains` (unique
    // when set); a chain-with-a-role IS the role assignment. Utility
    // chains (consensus-*) keep role=null and stay invisible to the
    // wizard.
}
