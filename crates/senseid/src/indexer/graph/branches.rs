use super::GraphDb;
use rusqlite::params;

impl GraphDb {
    // ── Branch graph operations ────────────────────────────────────

    /// Clone all nodes and edges from one project to another (for branch snapshots).
    /// Source: repo_id (current branch), Target: repo_id@branch_name
    pub fn clone_project_graph(&self, source_project: &str, target_project: &str) -> Result<u32, String> {
        // Clone nodes
        let count: u32 = self.conn.execute(
            "INSERT OR REPLACE INTO hierarchy_nodes(id, name, kind, level, parent_id, file, line, project, sig, body, docstring, complexity, tags, doc_type, doc_category)
             SELECT
                REPLACE(id, ?1, ?2), name, kind, level,
                CASE WHEN parent_id IS NOT NULL THEN REPLACE(parent_id, ?1, ?2) ELSE NULL END,
                file, line, ?2, sig, body, docstring, complexity, tags, doc_type, doc_category
             FROM hierarchy_nodes WHERE project = ?1",
            params![source_project, target_project],
        ).map_err(|e| e.to_string())? as u32;

        // Clone edges (only those between nodes in the source project)
        self.conn.execute(
            "INSERT OR REPLACE INTO edges(from_id, to_id, edge_type, weight)
             SELECT REPLACE(from_id, ?1, ?2), REPLACE(to_id, ?1, ?2), edge_type, weight
             FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)",
            params![source_project, target_project],
        ).ok();

        Ok(count)
    }

    /// Delete all nodes and edges for a project (branch cleanup).
    pub fn delete_project_graph(&self, project: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)
             OR to_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)",
            params![project],
        ).ok();
        self.conn.execute("DELETE FROM hierarchy_nodes WHERE project = ?1", params![project]).ok();
        self.conn.execute("DELETE FROM unresolved_refs WHERE project = ?1", params![project]).ok();
        Ok(())
    }

    /// Check if a project graph exists (has any nodes).
    pub fn project_exists(&self, project: &str) -> bool {
        self.conn.query_row(
            "SELECT COUNT(*) FROM hierarchy_nodes WHERE project = ?1 LIMIT 1",
            params![project], |row| row.get::<_, u32>(0),
        ).unwrap_or(0) > 0
    }
}
