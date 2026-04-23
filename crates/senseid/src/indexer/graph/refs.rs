use super::GraphDb;
use rusqlite::params;

impl GraphDb {
    // ── Unresolved references (staging for resolve_edges) ──────────

    /// Store an unresolved reference (import, call, parent) for later resolution.
    pub fn add_unresolved_ref(&self, source_id: &str, ref_kind: &str, ref_target: &str, project: &str) -> Result<(), String> {
        self.conn.execute(
            "INSERT INTO unresolved_refs(source_id, ref_kind, ref_target, project) VALUES(?1,?2,?3,?4)",
            params![source_id, ref_kind, ref_target, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get all unresolved references for a project.
    pub fn get_unresolved_refs(&self, project: &str) -> Result<Vec<(String, String, String)>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT source_id, ref_kind, ref_target FROM unresolved_refs WHERE project = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Clear unresolved references for a project (after resolution).
    pub fn clear_unresolved_refs(&self, project: &str) -> Result<(), String> {
        self.conn.execute("DELETE FROM unresolved_refs WHERE project = ?1", params![project])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Clear unresolved references from a specific source node.
    pub fn clear_unresolved_refs_from(&self, source_id: &str, project: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM unresolved_refs WHERE source_id = ?1 AND project = ?2",
            params![source_id, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }
}
