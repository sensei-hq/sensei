use super::GraphDb;
use rusqlite::params;
use crate::types::HierarchyNode;

impl GraphDb {
    // ── Node CRUD ────────────────────────────────────────────────────

    /// Insert or replace a node in the hierarchy.
    pub fn merge_node(&self, node: &HierarchyNode) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO hierarchy_nodes(
                id, name, kind, level, parent_id, file, line, project,
                sig, body, docstring, complexity, tags, doc_type, doc_category
            ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                node.id, node.name, node.kind.as_str(), node.level, node.parent_id,
                node.file, node.line, node.project,
                node.sig.as_deref().map(|s| &s[..s.len().min(500)]),
                node.body.as_deref().map(|s| &s[..s.len().min(10000)]),
                node.docstring.as_deref().map(|s| &s[..s.len().min(2000)]),
                node.complexity, node.tags, node.doc_type, node.doc_category,
            ],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a node and all edges involving it.
    pub fn delete_node(&self, id: &str) -> Result<(), String> {
        self.conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", params![id]).ok();
        self.conn.execute("DELETE FROM hierarchy_nodes WHERE id = ?1", params![id]).ok();
        Ok(())
    }

    /// Delete all nodes with a given file path (and their edges).
    pub fn delete_by_file(&self, file_path: &str, project: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE file = ?1 AND project = ?2)
             OR to_id IN (SELECT id FROM hierarchy_nodes WHERE file = ?1 AND project = ?2)",
            params![file_path, project],
        ).ok();
        self.conn.execute("DELETE FROM hierarchy_nodes WHERE file = ?1 AND project = ?2", params![file_path, project]).ok();
        Ok(())
    }

    /// Clear all hierarchy nodes (packages, modules, grouping nodes, doc containment) for a project.
    /// Preserves function/type/file/doc leaf nodes.
    pub fn clear_hierarchy(&self, project: &str) -> Result<(), String> {
        let hierarchy_kinds = "('repo','package','module')";
        self.conn.execute(
            &format!("DELETE FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1 AND kind IN {}) OR to_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1 AND kind IN {})", hierarchy_kinds, hierarchy_kinds),
            params![project],
        ).ok();
        // Also delete containment and HAS_METHOD edges
        self.conn.execute(
            "DELETE FROM edges WHERE edge_type IN ('CONTAINS_PKG','CONTAINS_MOD','CONTAINS_FN','CONTAINS_FILE','CONTAINS_TYPE','CONTAINS_DOC','HAS_METHOD')
             AND (from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1) OR to_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1))",
            params![project],
        ).ok();
        self.conn.execute(
            &format!("DELETE FROM hierarchy_nodes WHERE project = ?1 AND kind IN {}", hierarchy_kinds),
            params![project],
        ).ok();
        Ok(())
    }

    pub fn clear_all(&self) -> Result<(), String> {
        self.conn.execute_batch("DELETE FROM hierarchy_nodes; DELETE FROM edges; DELETE FROM unresolved_refs;")
            .map_err(|e| e.to_string())
    }

    /// Get all nodes for a project.
    pub fn get_nodes(&self, project: &str) -> Result<Vec<crate::types::GraphNode>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, COALESCE(file,''), line, complexity, doc_type, level, parent_id, tags FROM hierarchy_nodes WHERE project = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(crate::types::GraphNode {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                file: row.get(3)?,
                line: row.get(4)?,
                complexity: row.get(5).ok(),
                doc_type: row.get::<_, Option<String>>(6)?,
                level: row.get::<_, Option<String>>(7)?,
                parent_id: row.get::<_, Option<String>>(8)?,
                tags: row.get::<_, Option<String>>(9)?,
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Count nodes by kind for a project. Returns a map of kind -> count.
    pub fn count_by_kind(&self, project: &str) -> Result<std::collections::HashMap<String, u32>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT kind, COUNT(*) FROM hierarchy_nodes WHERE project = ?1 GROUP BY kind"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<std::collections::HashMap<_, _>, _>>().map_err(|e| e.to_string())
    }

    /// Search nodes by name pattern and optional kind filter.
    pub fn search_nodes(&self, query: &str, project: &str, kinds: &[&str]) -> Result<Vec<crate::types::FunctionDetail>, String> {
        let pattern = format!("%{}%", query);
        let kind_clause = if kinds.is_empty() {
            String::new()
        } else {
            let ks: Vec<String> = kinds.iter().map(|k| format!("'{}'", k)).collect();
            format!(" AND kind IN ({})", ks.join(","))
        };
        let sql = format!(
            "SELECT id, name, COALESCE(file,''), line, sig, docstring, COALESCE(complexity,1), COALESCE(tags,''), kind
             FROM hierarchy_nodes WHERE project = ?1 AND name LIKE ?2{} LIMIT 50", kind_clause
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project, pattern], |row| {
            Ok(crate::types::FunctionDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                file: row.get(2)?,
                line: row.get(3)?,
                signature: row.get::<_, Option<String>>(4)?,
                docstring: row.get::<_, Option<String>>(5)?,
                complexity: row.get(6)?,
                tags: row.get::<_, String>(7).unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Find a node by exact name and kind.
    #[cfg(test)]
    pub fn find_node_by_name(&self, name: &str, project: &str, kind: &str) -> Result<Option<String>, String> {
        use rusqlite::OptionalExtension;
        self.conn.query_row(
            "SELECT id FROM hierarchy_nodes WHERE name = ?1 AND project = ?2 AND kind = ?3 LIMIT 1",
            params![name, project, kind],
            |row| row.get(0),
        ).optional().map_err(|e| e.to_string())
    }

    /// Tag a node (sets the tags field).
    #[cfg(test)]
    pub fn tag_node(&self, node_id: &str, tags: &str) -> Result<(), String> {
        self.conn.execute(
            "UPDATE hierarchy_nodes SET tags = ?2 WHERE id = ?1",
            params![node_id, tags],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get nodes by tag pattern and kind.
    pub fn nodes_by_tag(&self, tag: &str, project: &str, kind: &str) -> Result<Vec<(String, String, String)>, String> {
        let pattern = format!("%{}%", tag);
        let mut stmt = self.conn.prepare(
            "SELECT id, COALESCE(file,''), COALESCE(tags,'') FROM hierarchy_nodes WHERE project = ?1 AND kind = ?2 AND tags LIKE ?3 LIMIT 100"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project, kind, pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::super::GraphDb;
    use crate::types::{NodeKind, HierarchyNode};

    #[test]
    fn merge_and_count_nodes() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:hello:1", "hello", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:world:5", "world", "a.py", 5, "proj")).unwrap();
        let counts = db.count_by_kind("proj").unwrap();
        assert_eq!(counts.get("function"), Some(&2));
    }

    #[test]
    fn delete_by_file() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:foo:1", "foo", "/tmp/a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:bar:5", "bar", "/tmp/a.py", 5, "proj")).unwrap();
        db.merge_node(&make_fn("fn:b:baz:1", "baz", "/tmp/b.py", 1, "proj")).unwrap();
        let c = db.count_by_kind("proj").unwrap();
        assert_eq!(c.get("function"), Some(&3));
        db.delete_by_file("/tmp/a.py", "proj").unwrap();
        let c2 = db.count_by_kind("proj").unwrap();
        assert_eq!(c2.get("function"), Some(&1));
    }

    #[test]
    fn get_nodes_and_edges() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:5", "bar", "a.py", 5, "proj")).unwrap();
        db.merge_edge("fn:a:1", "fn:a:5", "CALLS").unwrap();
        let nodes = db.get_nodes("proj").unwrap();
        assert_eq!(nodes.len(), 2);
        let edges = db.get_edges("proj").unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, "CALLS");
    }

    #[test]
    fn search_nodes_by_name() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:hello:1", "hello", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:help:5", "help", "a.py", 5, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:world:10", "world", "a.py", 10, "proj")).unwrap();
        let results = db.search_nodes("hel", "proj", &["function"]).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn find_node_by_name() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:hello:1", "hello", "a.py", 1, "proj")).unwrap();
        let found = db.find_node_by_name("hello", "proj", "function").unwrap();
        assert_eq!(found, Some("fn:a:hello:1".to_string()));
        let missing = db.find_node_by_name("missing", "proj", "function").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn tag_and_find_by_tag() {
        let db = GraphDb::open_memory().unwrap();
        let mut f = HierarchyNode::group("file:a.ts".into(), "a".into(), NodeKind::File, "proj".into());
        f.file = Some("a.ts".into());
        db.merge_node(&f).unwrap();
        db.tag_node("file:a.ts", "react,hooks").unwrap();
        let files = db.nodes_by_tag("react", "proj", "file").unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn packages_and_modules() {
        let db = GraphDb::open_memory().unwrap();
        let mut pkg = HierarchyNode::group("pkg:proj:ui".into(), "ui".into(), NodeKind::Package, "proj".into());
        pkg.level = Some("npm-workspace".into());
        db.merge_node(&pkg).unwrap();
        let mut module = HierarchyNode::group("mod:proj:ui/src".into(), "ui/src".into(), NodeKind::Module, "proj".into());
        module.parent_id = Some("pkg:proj:ui".into());
        db.merge_node(&module).unwrap();
        let counts = db.count_by_kind("proj").unwrap();
        assert_eq!(counts.get("package"), Some(&1));
        assert_eq!(counts.get("module"), Some(&1));
    }

    #[test]
    fn clear_all_works() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_edge("fn:a:1", "fn:a:1", "CALLS").unwrap();
        db.clear_all().unwrap();
        let counts = db.count_by_kind("proj").unwrap();
        assert!(counts.is_empty());
    }
}
