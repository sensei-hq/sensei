//! Reads for the dōjō graph push (#225; docs/spec/dojo/graph-sync.md §2–3).
//!
//! A checkout is a `git` folder: its files, nodes and edges all hang off it, and
//! `files.file_path` is relative to it — so it is already the repository-relative
//! path the dōjō keys on, and every node goes out at `folder_path = ''`.
//!
//! What the slices carry is graph-sync.md §2: no embedding, no content. An edge
//! into ANOTHER folder (another repository's checkout) is sent without a target
//! key — that key would mean nothing in this repository's graph — keeping only
//! its name.

use std::collections::BTreeMap;

use super::PgStore;

/// One checkout of a repository: the folder its graph hangs off, the branch it
/// was indexed on, and where it lives (git runs there).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCheckout {
    pub folder_id: uuid::Uuid,
    pub branch: Option<String>,
    pub abs_path: String,
}

/// One node as the dōjō reads it (`n` = the node, `p` = its parent). Keys only:
/// the parent by fqn, never a local uuid; no embedding, no content. A schema
/// node carries its entity, columns, indexes and foreign keys (a key's target by
/// node key, by the rule edges follow); any other node carries `schema: null`.
const NODE_JSON: &str = "jsonb_build_object(\
    'folder_path', '', 'fqn', n.fqn, 'kind', n.kind::text, 'name', n.name, \
    'parent_fqn', p.fqn, 'line_start', n.line_start, 'line_end', n.line_end, \
    'is_exported', n.is_exported, 'is_test', n.is_test, 'signature', n.signature, \
    'description', n.description, 'docstring', n.docstring, 'language', n.language, \
    'community_id', n.community_id, 'tags', n.tags, 'props', n.props, \
    'schema', (select jsonb_build_object( \
        'entity', jsonb_build_object('kind', se.kind::text, 'db_schema', se.db_schema, \
            'source', se.source::text, 'naming', se.naming::text, 'note', se.note, 'note_md', se.note_md), \
        'columns', coalesce((select jsonb_agg(jsonb_build_object('ordinal', c.ordinal, 'name', c.name, \
            'sql_type', c.sql_type, 'primary_key', c.primary_key, 'not_null', c.not_null, \
            'unique_key', c.unique_key, 'foreign_key', c.foreign_key, 'enum_typed', c.enum_typed, \
            'default_expr', c.default_expr, 'note', c.note) order by c.ordinal, c.name) \
            from sensei.schema_columns c where c.node_id = n.id), '[]'::jsonb), \
        'indexes', coalesce((select jsonb_agg(jsonb_build_object('name', i.name, \
            'definition', i.definition, 'unique_index', i.unique_index) order by i.name) \
            from sensei.schema_indexes i where i.node_id = n.id), '[]'::jsonb), \
        'refs', coalesce((select jsonb_agg(jsonb_build_object('from_column', r.from_column, \
            'to_folder', case when tn.folder_id = $1 then '' end, \
            'to_fqn', case when tn.folder_id = $1 then tn.fqn end, \
            'to_column', r.to_column, 'on_delete', r.on_delete::text) order by r.id) \
            from sensei.schema_refs r left join sensei.nodes tn on tn.id = r.to_node_id \
            where r.from_node_id = n.id), '[]'::jsonb)) \
      from sensei.schema_entities se where se.node_id = n.id))";

/// One edge as the dōjō reads it (`e` = the edge, `s` its source, `t` its
/// target, `$1` the checkout). A target in another folder gets no key.
const EDGE_JSON: &str = "jsonb_build_object(\
    'source_folder', '', 'source_fqn', s.fqn, 'kind', e.kind::text, \
    'target_folder', case when t.folder_id = $1 then '' end, \
    'target_fqn', case when t.folder_id = $1 then t.fqn end, \
    'target_name', coalesce(e.target_name, t.name), 'confidence', e.confidence::text, \
    'resolved_via', e.resolved_via, 'unresolved_reason', e.unresolved_reason, 'props', e.props)";

/// The `nodes` and `edges` arrays of the slice whose nodes satisfy `which`
/// (a predicate over `n`, and over `s` for edges).
fn slice_arrays(which_node: &str, which_source: &str) -> String {
    format!(
        "'nodes', coalesce((select jsonb_agg({NODE_JSON} order by n.fqn) \
                            from sensei.nodes n left join sensei.nodes p on p.id = n.parent_id \
                           where n.folder_id = $1 and {which_node}), '[]'::jsonb), \
         'edges', coalesce((select jsonb_agg({EDGE_JSON} order by s.fqn, e.target_name) \
                            from sensei.edges e \
                            join sensei.nodes s on s.id = e.source_id \
                            left join sensei.nodes t on t.id = e.target_id \
                           where s.folder_id = $1 and {which_source}), '[]'::jsonb)"
    )
}

impl PgStore {
    /// Every checkout (a `git` folder) of the repository `repo_key`.
    pub async fn graph_checkouts(&self, repo_key: &str) -> Result<Vec<GraphCheckout>, String> {
        let rows: Vec<(uuid::Uuid, Option<String>, String)> = sqlx_core::query_as::query_as(
            "select f.id, f.branch, f.abs_path \
               from sensei.folders f \
               join sensei.repositories r on r.id = f.repository_id \
              where r.repo_key = $1 and f.kind = 'git' \
              order by f.abs_path",
        )
        .bind(repo_key)
        .fetch_all(self.pool())
        .await
        .map_err(|e| format!("graph_checkouts: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(folder_id, branch, abs_path)| GraphCheckout { folder_id, branch, abs_path })
            .collect())
    }

    /// path → content hash of every file of a checkout that holds graph: files
    /// the walk skipped carry none and are left out.
    pub async fn graph_manifest(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<BTreeMap<String, String>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "select file_path, content_hash from sensei.files \
              where folder_id = $1 and skip_reason is null and content_hash is not null",
        )
        .bind(folder_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| format!("graph_manifest: {e}"))?;
        Ok(rows.into_iter().collect())
    }

    /// The files of a checkout whose parse has not run on their current content:
    /// a content change resets `parsed_at`, so these carry a new hash over their
    /// old nodes until the parse lands (#225). A skipped file is decided, not
    /// pending.
    pub async fn graph_parse_pending(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<std::collections::HashSet<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "select file_path from sensei.files \
              where folder_id = $1 and skip_reason is null and parsed_at is null",
        )
        .bind(folder_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| format!("graph_parse_pending: {e}"))?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    /// The slice of each named file: its nodes, and the edges whose source is in
    /// it, as the dōjō's `file_graph_slices` reads them. A path with no `files`
    /// row is absent from the result.
    pub async fn graph_slices(
        &self,
        folder_id: &uuid::Uuid,
        paths: &[String],
    ) -> Result<Vec<(String, serde_json::Value)>, String> {
        let sql = format!(
            "select f.file_path, jsonb_build_object('path', f.file_path, 'content_hash', f.content_hash, {}) \
               from sensei.files f \
              where f.folder_id = $1 and f.file_path = any($2) \
              order by f.file_path",
            slice_arrays("n.file_id = f.id", "s.file_id = f.id"),
        );
        sqlx_core::query_as::query_as(&sql)
            .bind(folder_id)
            .bind(paths)
            .fetch_all(self.pool())
            .await
            .map_err(|e| format!("graph_slices: {e}"))
    }

    /// The checkout's unplaced slice: nodes with no file (packages, modules,
    /// external placeholders) and their edges, under a null path.
    pub async fn graph_unplaced_slice(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        let sql = format!(
            "select jsonb_build_object('path', null, {})",
            slice_arrays("n.file_id is null", "s.file_id is null"),
        );
        let (slice,): (serde_json::Value,) = sqlx_core::query_as::query_as(&sql)
            .bind(folder_id)
            .fetch_one(self.pool())
            .await
            .map_err(|e| format!("graph_unplaced_slice: {e}"))?;
        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_core::query::query;
    use sqlx_core::query_as::query_as;

    struct Fixture {
        pg: PgStore,
        repo_key: String,
        folder: uuid::Uuid,
        other: uuid::Uuid,
        repository: uuid::Uuid,
    }

    /// One repository with a checkout on `main` — src/a.ts holding class C with
    /// method f, function g; f calls g, f calls a node in ANOTHER repository's
    /// checkout, g calls something unresolved; skipped.bin with no graph; a
    /// package node with no file, calling g — and that other checkout.
    async fn fixture() -> Fixture {
        let pg = PgStore::connect_test().await.unwrap();
        crate::tasks::test_support::ensure_test_watch_root(&pg).await;
        let u = uuid::Uuid::new_v4();
        let repo_key = format!("test/graph-push-{u}");
        let (repository,): (uuid::Uuid,) =
            query_as("insert into sensei.repositories(repo_key, name) values($1, $1) returning id")
                .bind(&repo_key)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        let folder = uuid::Uuid::new_v4();
        let other = uuid::Uuid::new_v4();
        for (id, rid, name) in
            [(folder, Some(repository), format!("gp-{u}")), (other, None, format!("gp-other-{u}"))]
        {
            query(
                "insert into sensei.folders(id, root_id, kind, name, path, abs_path, repository_id, branch) \
                 values($1, '00000000-0000-0000-0000-000000000001', 'git', $2, $2, $3, $4, 'main')",
            )
            .bind(id)
            .bind(&name)
            .bind(format!("/_test/{name}"))
            .bind(rid)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        for (fid, path, hash, skip) in [
            (folder, "db/t.sql", "ht", None),
            (folder, "src/a.ts", "ha", None),
            (folder, "skipped.bin", "hs", Some("binary_content")),
            (other, "x.ts", "hx", None),
        ] {
            query(
                "insert into sensei.files(folder_id, file_path, mtime, content_hash, skip_reason) \
                 values($1, $2, 0, $3, $4::sensei.scan_skip_reason)",
            )
            .bind(fid)
            .bind(path)
            .bind(hash)
            .bind(skip)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        let mut ids = std::collections::HashMap::new();
        for (fid, file, fqn, kind, parent) in [
            (folder, Some("src/a.ts"), "a.C", "class", None),
            (folder, Some("src/a.ts"), "a.C.f", "method", Some("a.C")),
            (folder, Some("src/a.ts"), "a.g", "function", None),
            (folder, None, "pkg", "package", None),
            (folder, Some("db/t.sql"), "app.orders", "struct", None),
            (folder, Some("db/t.sql"), "app.customers", "struct", None),
            (other, Some("x.ts"), "x.h", "function", None),
        ] {
            let (id,): (uuid::Uuid,) = query_as(
                "insert into sensei.nodes(folder_id, file_id, kind, name, fqn, parent_id, line_start, line_end, tags, language) \
                 values($1, (select id from sensei.files where folder_id = $1 and file_path = $2), $3::sensei.node_kind, $4, $4, \
                        (select id from sensei.nodes where folder_id = $1 and fqn = $5), 1, 2, '{t}', 'typescript') returning id",
            )
            .bind(fid)
            .bind(file)
            .bind(kind)
            .bind(fqn)
            .bind(parent)
            .fetch_one(pg.pool())
            .await
            .unwrap_or_else(|e| panic!("node {fqn}: {e}"));
            ids.insert(fqn, id);
        }
        for (src, tgt, name) in [
            ("a.C.f", Some("a.g"), "g"),
            ("a.C.f", Some("x.h"), "h"),
            ("a.g", None, "unknown_fn"),
            ("pkg", Some("a.g"), "g"),
        ] {
            query(
                "insert into sensei.edges(folder_id, source_id, target_id, target_name, kind, props) \
                 values($1, $2, $3, $4, 'calls', '{\"occurrences\": {}}')",
            )
            .bind(folder)
            .bind(ids[src])
            .bind(tgt.map(|t| ids[t]))
            .bind(name)
            .execute(pg.pool())
            .await
            .unwrap_or_else(|e| panic!("edge {src}→{name}: {e}"));
        }
        // app.orders declares a table: two columns, an index, a key to customers.
        for sql in [
            "insert into sensei.schema_entities(node_id, kind, db_schema, source, naming) values($1, 'table', 'app', 'dbd', 'stated')",
            "insert into sensei.schema_columns(node_id, ordinal, name, sql_type, primary_key, not_null) values($1, 1, 'id', 'uuid', true, true)",
            "insert into sensei.schema_columns(node_id, ordinal, name, sql_type, foreign_key) values($1, 2, 'customer_id', 'uuid', true)",
            "insert into sensei.schema_indexes(node_id, name, definition, unique_index) values($1, 'orders_customer', 'create index', false)",
        ] {
            query(sql)
                .bind(ids["app.orders"])
                .execute(pg.pool())
                .await
                .unwrap_or_else(|e| panic!("{sql}: {e}"));
        }
        // A key's target must be an entity locally (schema_refs' foreign key).
        query("insert into sensei.schema_entities(node_id, kind, db_schema, source, naming) values($1, 'table', 'app', 'dbd', 'stated')")
            .bind(ids["app.customers"])
            .execute(pg.pool())
            .await
            .unwrap();
        query(
            "insert into sensei.schema_refs(from_node_id, from_column, to_node_id, to_column, on_delete) \
             values($1, 'customer_id', $2, 'id', 'cascade')",
        )
        .bind(ids["app.orders"])
        .bind(ids["app.customers"])
        .execute(pg.pool())
        .await
        .unwrap();
        Fixture { pg, repo_key, folder, other, repository }
    }

    async fn drop_fixture(f: Fixture) {
        for id in [f.folder, f.other] {
            query("delete from sensei.folders where id = $1")
                .bind(id)
                .execute(f.pg.pool())
                .await
                .unwrap();
        }
        query("delete from sensei.repositories where id = $1")
            .bind(f.repository)
            .execute(f.pg.pool())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_repository_lists_its_checkouts_with_branch_and_path() {
        let f = fixture().await;
        let cs = f.pg.graph_checkouts(&f.repo_key).await.unwrap();
        assert_eq!(cs.len(), 1, "only this repository's checkout: {cs:?}");
        assert_eq!(cs[0].folder_id, f.folder);
        assert_eq!(cs[0].branch.as_deref(), Some("main"));
        assert!(cs[0].abs_path.starts_with("/_test/gp-"));
        assert!(f.pg.graph_checkouts("test/no-such-repo").await.unwrap().is_empty());
        drop_fixture(f).await;
    }

    #[tokio::test]
    async fn the_manifest_lists_files_with_graph_and_leaves_skipped_ones_out() {
        let f = fixture().await;
        let m = f.pg.graph_manifest(&f.folder).await.unwrap();
        assert_eq!(
            m,
            BTreeMap::from([
                ("db/t.sql".to_string(), "ht".to_string()),
                ("src/a.ts".to_string(), "ha".to_string())
            ])
        );
        drop_fixture(f).await;
    }

    #[tokio::test]
    async fn a_slice_carries_the_files_nodes_and_its_outgoing_edges_keyed_for_the_dojo() {
        let f = fixture().await;
        let slices =
            f.pg.graph_slices(&f.folder, &["src/a.ts".into(), "missing.ts".into()]).await.unwrap();
        assert_eq!(slices.len(), 1, "a path with no files row is absent");
        let (path, s) = &slices[0];
        assert_eq!(path, "src/a.ts");
        assert_eq!(s["path"], "src/a.ts");
        assert_eq!(s["content_hash"], "ha");

        let nodes = s["nodes"].as_array().unwrap();
        let fqns: Vec<&str> = nodes.iter().map(|n| n["fqn"].as_str().unwrap()).collect();
        assert_eq!(
            fqns,
            ["a.C", "a.C.f", "a.g"],
            "the file's nodes only — not the package, not the other repo's"
        );
        let m = nodes.iter().find(|n| n["fqn"] == "a.C.f").unwrap();
        assert_eq!(m["folder_path"], "");
        assert_eq!(m["kind"], "method");
        assert_eq!(m["parent_fqn"], "a.C", "the parent travels as a key, not a local uuid");
        assert_eq!(m["tags"], serde_json::json!(["t"]));
        assert_eq!(m["language"], "typescript");
        assert!(m.get("embedding").is_none() && m.get("content").is_none(), "§2: neither travels");

        let edges = s["edges"].as_array().unwrap();
        assert_eq!(
            edges.len(),
            3,
            "f→g, f→other repo, g→unresolved; the package's edge is not this file's"
        );
        let to = |name: &str| edges.iter().find(|e| e["target_name"] == name).unwrap();
        assert_eq!(
            (to("g")["source_fqn"].as_str(), to("g")["target_fqn"].as_str()),
            (Some("a.C.f"), Some("a.g"))
        );
        assert_eq!(to("g")["target_folder"], "");
        assert_eq!(to("g")["kind"], "calls");
        // Another repository's node: no key that would mean nothing here.
        assert!(to("h")["target_fqn"].is_null() && to("h")["target_folder"].is_null());
        assert!(to("unknown_fn")["target_fqn"].is_null());
        drop_fixture(f).await;
    }

    #[tokio::test]
    async fn a_schema_node_carries_its_entity_columns_indexes_and_keys_and_others_carry_none() {
        let f = fixture().await;
        let slices =
            f.pg.graph_slices(&f.folder, &["db/t.sql".into(), "src/a.ts".into()]).await.unwrap();
        let node = |path: &str, fqn: &str| {
            let (_, s) = slices.iter().find(|(p, _)| p == path).unwrap();
            s["nodes"].as_array().unwrap().iter().find(|n| n["fqn"] == fqn).unwrap().clone()
        };
        let orders = node("db/t.sql", "app.orders");
        let sc = &orders["schema"];
        assert_eq!(sc["entity"]["kind"], "table");
        assert_eq!(sc["entity"]["db_schema"], "app");
        assert_eq!(sc["entity"]["source"], "dbd");
        let cols: Vec<&str> =
            sc["columns"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
        assert_eq!(cols, ["id", "customer_id"], "in ordinal order");
        assert_eq!(sc["columns"][0]["primary_key"], true);
        assert_eq!(sc["indexes"][0]["name"], "orders_customer");
        let fk = &sc["refs"][0];
        assert_eq!(
            (fk["from_column"].as_str(), fk["to_fqn"].as_str(), fk["to_column"].as_str()),
            (Some("customer_id"), Some("app.customers"), Some("id"))
        );
        assert_eq!(fk["to_folder"], "", "the referenced table as a key, in this checkout");
        assert_eq!(fk["on_delete"], "cascade");
        // An entity with no columns, indexes or keys carries empty lists, not nulls.
        let customers = node("db/t.sql", "app.customers");
        assert_eq!(customers["schema"]["entity"]["kind"], "table");
        assert_eq!(customers["schema"]["columns"], serde_json::json!([]));
        assert!(
            node("src/a.ts", "a.g")["schema"].is_null(),
            "a node that declares nothing carries null"
        );
        drop_fixture(f).await;
    }

    #[tokio::test]
    async fn the_unplaced_slice_is_the_nodes_without_a_file_and_their_edges() {
        let f = fixture().await;
        let s = f.pg.graph_unplaced_slice(&f.folder).await.unwrap();
        assert!(s["path"].is_null());
        let fqns: Vec<&str> =
            s["nodes"].as_array().unwrap().iter().map(|n| n["fqn"].as_str().unwrap()).collect();
        assert_eq!(fqns, ["pkg"]);
        let edges = s["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0]["source_fqn"], "pkg");
        assert_eq!(edges[0]["target_fqn"], "a.g");
        drop_fixture(f).await;
    }
}
