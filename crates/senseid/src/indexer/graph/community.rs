use super::GraphDb;
use rusqlite::params;

impl GraphDb {
    // ── Community detection support ──────────────────────────────────

    pub fn store_communities(&self, communities: &std::collections::HashMap<String, u32>) -> Result<(), String> {
        for (node_id, comm) in communities {
            self.conn.execute("UPDATE hierarchy_nodes SET community = ?2 WHERE id = ?1", params![node_id, comm]).ok();
        }
        Ok(())
    }

    pub fn get_communities(&self, project: &str) -> Result<Vec<super::super::community::CommunityInfo>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT community, COUNT(*) as cnt, GROUP_CONCAT(name, ', ') as members
             FROM hierarchy_nodes
             WHERE project = ?1 AND community >= 0
             GROUP BY community
             ORDER BY cnt DESC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(super::super::community::CommunityInfo {
                id: row.get(0)?,
                size: row.get(1)?,
                sample_members: row.get::<_, String>(2)
                    .unwrap_or_default()
                    .split(", ")
                    .take(5)
                    .map(|s| s.to_string())
                    .collect(),
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::super::GraphDb;

    #[test]
    fn communities() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:2", "bar", "a.py", 5, "proj")).unwrap();
        let mut comms = std::collections::HashMap::new();
        comms.insert("fn:a:1".to_string(), 0u32);
        comms.insert("fn:a:2".to_string(), 0u32);
        db.store_communities(&comms).unwrap();
        let result = db.get_communities("proj").unwrap();
        assert!(!result.is_empty());
    }
}
