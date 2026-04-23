use super::GraphDb;
use rusqlite::params;

impl GraphDb {
    // ── Edges ────────────────────────────────────────────────────────

    pub fn merge_edge(&self, from_id: &str, to_id: &str, edge_type: &str) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO edges(from_id, to_id, edge_type) VALUES(?1,?2,?3)",
            params![from_id, to_id, edge_type],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get all edges for a project (where from_id belongs to this project).
    pub fn get_edges(&self, project: &str) -> Result<Vec<crate::types::GraphEdge>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, e.to_id, e.edge_type FROM edges e
             WHERE e.from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(crate::types::GraphEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                edge_type: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Count edges for a project.
    pub fn count_edges(&self, project: &str) -> Result<u32, String> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)",
            params![project], |row| row.get(0),
        ).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::super::GraphDb;
    use crate::types::{NodeKind, HierarchyNode};

    #[test]
    fn merge_edge_and_query() {
        let db = GraphDb::open_memory().unwrap();
        let mut file = HierarchyNode::group("file:a.py".into(), "a".into(), NodeKind::File, "proj".into());
        file.file = Some("a.py".into());
        db.merge_node(&file).unwrap();
        db.merge_node(&make_fn("fn:a:foo:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_edge("file:a.py", "fn:a:foo:1", "EXPORTS_FN").unwrap();
        let edges = db.get_edges("proj").unwrap();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn containment_edges() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&HierarchyNode::group("pkg:proj:ui".into(), "ui".into(), NodeKind::Package, "proj".into())).unwrap();
        db.merge_node(&HierarchyNode::group("mod:proj:src".into(), "src".into(), NodeKind::Module, "proj".into())).unwrap();
        db.merge_node(&make_fn("fn:a:foo:1", "foo", "a.ts", 1, "proj")).unwrap();
        db.merge_node(&make_type("type:a:Foo:1", "Foo", "a.ts", 1, NodeKind::Class, "proj")).unwrap();
        db.merge_edge("pkg:proj:ui", "mod:proj:src", "CONTAINS_MOD").unwrap();
        db.merge_edge("mod:proj:src", "fn:a:foo:1", "CONTAINS_FN").unwrap();
        db.merge_edge("type:a:Foo:1", "fn:a:foo:1", "HAS_METHOD").unwrap();
        let edges = db.get_edges("proj").unwrap();
        let types: Vec<&str> = edges.iter().map(|e| e.edge_type.as_str()).collect();
        assert!(types.contains(&"CONTAINS_MOD"));
        assert!(types.contains(&"CONTAINS_FN"));
        assert!(types.contains(&"HAS_METHOD"));
    }

    #[test]
    fn count_edges_works() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_type("type:a:Foo:1", "Foo", "a.py", 1, NodeKind::Class, "proj")).unwrap();
        db.merge_node(&HierarchyNode::group("mod:proj:src".into(), "src".into(), NodeKind::Module, "proj".into())).unwrap();
        db.merge_edge("fn:a:1", "fn:a:1", "CALLS").unwrap();
        db.merge_edge("type:a:Foo:1", "fn:a:1", "HAS_METHOD").unwrap();
        db.merge_edge("mod:proj:src", "fn:a:1", "CONTAINS_FN").unwrap();
        assert_eq!(db.count_edges("proj").unwrap(), 3);
    }
}
