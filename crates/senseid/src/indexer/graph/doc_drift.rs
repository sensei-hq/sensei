use super::{GraphDb, DocDrift};
use rusqlite::params;

impl GraphDb {
    // ── Doc drift ────────────────────────────────────────────────────

    #[cfg(test)]
    pub fn find_drifted_docs(
        &self, changed_file_ids: &[String], changed_fn_ids: &[String],
    ) -> Result<Vec<DocDrift>, String> {
        let mut drifts = Vec::new();
        for file_id in changed_file_ids {
            let mut stmt = self.conn.prepare(
                "SELECT n.id, COALESCE(n.file,''), e.edge_type FROM hierarchy_nodes n
                 JOIN edges e ON e.from_id = n.id
                 WHERE e.to_id = ?1 AND e.edge_type = 'COVERS'"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![file_id], |row| {
                Ok(DocDrift {
                    doc_id: row.get(0)?, doc_path: row.get(1)?,
                    edge_type: row.get(2)?, changed_target: file_id.clone(),
                })
            }).map_err(|e| e.to_string())?;
            for r in rows { if let Ok(d) = r { drifts.push(d); } }
        }
        for fn_id in changed_fn_ids {
            let mut stmt = self.conn.prepare(
                "SELECT n.id, COALESCE(n.file,''), e.edge_type FROM hierarchy_nodes n
                 JOIN edges e ON e.from_id = n.id
                 WHERE e.to_id = ?1 AND e.edge_type = 'MENTIONS_FN'"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![fn_id], |row| {
                Ok(DocDrift {
                    doc_id: row.get(0)?, doc_path: row.get(1)?,
                    edge_type: row.get(2)?, changed_target: fn_id.clone(),
                })
            }).map_err(|e| e.to_string())?;
            for r in rows { if let Ok(d) = r { drifts.push(d); } }
        }
        Ok(drifts)
    }

    #[cfg(test)]
    pub fn record_doc_drift(&self, drifts: &[DocDrift], project: &str) -> Result<(), String> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS doc_drift(
                doc_id TEXT NOT NULL, doc_path TEXT, edge_type TEXT,
                changed_target TEXT, detected_at TEXT, project TEXT
            )"
        ).map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        for drift in drifts {
            self.conn.execute(
                "INSERT OR REPLACE INTO doc_drift(doc_id, doc_path, edge_type, changed_target, detected_at, project) VALUES(?1,?2,?3,?4,?5,?6)",
                params![drift.doc_id, drift.doc_path, drift.edge_type, drift.changed_target, now, project],
            ).ok();
        }
        Ok(())
    }

    pub fn get_doc_drift(&self, project: &str) -> Result<Vec<DocDrift>, String> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS doc_drift(
                doc_id TEXT NOT NULL, doc_path TEXT, edge_type TEXT,
                changed_target TEXT, detected_at TEXT, project TEXT
            )"
        ).ok();
        let mut stmt = self.conn.prepare(
            "SELECT doc_id, doc_path, edge_type, changed_target FROM doc_drift WHERE project = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(DocDrift {
                doc_id: row.get(0)?, doc_path: row.get(1)?,
                edge_type: row.get(2)?, changed_target: row.get(3)?,
            })
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
    fn doc_drift_workflow() {
        let db = GraphDb::open_memory().unwrap();
        let mut f = HierarchyNode::group("file:a.py".into(), "a".into(), NodeKind::File, "proj".into());
        f.file = Some("a.py".into());
        db.merge_node(&f).unwrap();
        db.merge_node(&make_fn("fn:a:foo:1", "foo", "a.py", 1, "proj")).unwrap();
        let doc = HierarchyNode::doc("doc:readme".into(), "README".into(), NodeKind::Doc, "README.md".into(), Some("usage".into()), None, "proj".into());
        db.merge_node(&doc).unwrap();
        db.merge_edge("doc:readme", "file:a.py", "COVERS").unwrap();
        db.merge_edge("doc:readme", "fn:a:foo:1", "MENTIONS_FN").unwrap();
        let drifts = db.find_drifted_docs(&["file:a.py".into()], &["fn:a:foo:1".into()]).unwrap();
        assert_eq!(drifts.len(), 2);
        db.record_doc_drift(&drifts, "proj").unwrap();
        let stored = db.get_doc_drift("proj").unwrap();
        assert_eq!(stored.len(), 2);
    }
}
