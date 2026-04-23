use rusqlite::{params, Row};
use super::super::Store;

fn row_to_session(row: &Row) -> rusqlite::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "id": row.get::<_, String>(0)?,
        "project": row.get::<_, String>(1)?,
        "task": row.get::<_, String>(2)?,
        "startedAt": row.get::<_, String>(3)?,
        "completedAt": row.get::<_, Option<String>>(4)?,
        "outcome": row.get::<_, Option<String>>(5)?,
        "summary": row.get::<_, Option<String>>(6)?,
        "cost": row.get::<_, Option<f64>>(7)?,
        "tokensIn": row.get::<_, Option<i64>>(8)?,
        "tokensOut": row.get::<_, Option<i64>>(9)?,
        "ftr": match row.get::<_, Option<String>>(5)? {
            Some(ref o) if o == "completed" => Some(1.0),
            Some(ref o) if o == "partial" => Some(0.5),
            Some(ref o) if o == "blocked" => Some(0.0),
            _ => None::<f64>,
        },
    }))
}

impl Store {
    pub fn create_session(&self, id: &str, repo_id: &str, task: &str) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO sessions(id, repo_id, task, started_at) VALUES(?1,?2,?3,?4)",
            params![id, repo_id, task, now],
        )?;
        Ok(())
    }

    pub fn update_session(&self, id: &str, outcome: Option<&str>, summary: Option<&str>, cost: Option<f64>, tokens_in: Option<i64>, tokens_out: Option<i64>) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET completed_at=?2, outcome=COALESCE(?3,outcome), summary=COALESCE(?4,summary), cost=COALESCE(?5,cost), tokens_in=COALESCE(?6,tokens_in), tokens_out=COALESCE(?7,tokens_out) WHERE id=?1",
            params![id, now, outcome, summary, cost, tokens_in, tokens_out],
        )?;
        Ok(())
    }

    pub fn get_sessions(&self, repo_id: Option<&str>) -> rusqlite::Result<Vec<serde_json::Value>> {
        let query = if repo_id.is_some() {
            "SELECT id, repo_id, task, started_at, completed_at, outcome, summary, cost, tokens_in, tokens_out FROM sessions WHERE repo_id=?1 ORDER BY started_at DESC LIMIT 50"
        } else {
            "SELECT id, repo_id, task, started_at, completed_at, outcome, summary, cost, tokens_in, tokens_out FROM sessions ORDER BY started_at DESC LIMIT 50"
        };
        let mut stmt = self.conn.prepare(query)?;
        let rows = if let Some(rid) = repo_id {
            stmt.query_map(params![rid], row_to_session)?
        } else {
            stmt.query_map([], row_to_session)?
        };
        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    #[test]
    fn create_and_list_sessions() {
        let s = test_store();
        s.create_session("s1", "proj-a", "fix bug").unwrap();
        s.create_session("s2", "proj-a", "add feature").unwrap();
        let sessions = s.get_sessions(Some("proj-a")).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn update_session_outcome() {
        let s = test_store();
        s.create_session("s1", "proj", "task").unwrap();
        s.update_session("s1", Some("completed"), Some("done"), Some(0.5), Some(1000), Some(500)).unwrap();
        let sessions = s.get_sessions(Some("proj")).unwrap();
        assert_eq!(sessions[0]["outcome"], "completed");
        assert_eq!(sessions[0]["summary"], "done");
        assert_eq!(sessions[0]["cost"], 0.5);
        assert_eq!(sessions[0]["tokensIn"], 1000);
        assert_eq!(sessions[0]["tokensOut"], 500);
        assert_eq!(sessions[0]["ftr"], 1.0);
    }

    #[test]
    fn ftr_values_by_outcome() {
        let s = test_store();
        s.create_session("s1", "proj", "t1").unwrap();
        s.update_session("s1", Some("completed"), None, None, None, None).unwrap();
        s.create_session("s2", "proj", "t2").unwrap();
        s.update_session("s2", Some("partial"), None, None, None, None).unwrap();
        s.create_session("s3", "proj", "t3").unwrap();
        s.update_session("s3", Some("blocked"), None, None, None, None).unwrap();

        let sessions = s.get_sessions(Some("proj")).unwrap();
        let ftrs: Vec<f64> = sessions.iter().filter_map(|s| s["ftr"].as_f64()).collect();
        assert!(ftrs.contains(&1.0)); // completed
        assert!(ftrs.contains(&0.5)); // partial
        assert!(ftrs.contains(&0.0)); // blocked
    }

    #[test]
    fn get_sessions_all() {
        let s = test_store();
        s.create_session("s1", "proj-a", "t1").unwrap();
        s.create_session("s2", "proj-b", "t2").unwrap();
        let all = s.get_sessions(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn get_sessions_filtered() {
        let s = test_store();
        s.create_session("s1", "proj-a", "t1").unwrap();
        s.create_session("s2", "proj-b", "t2").unwrap();
        assert_eq!(s.get_sessions(Some("proj-a")).unwrap().len(), 1);
        assert_eq!(s.get_sessions(Some("proj-b")).unwrap().len(), 1);
        assert_eq!(s.get_sessions(Some("proj-c")).unwrap().len(), 0);
    }

    #[test]
    fn partial_update_preserves_fields() {
        let s = test_store();
        s.create_session("s1", "proj", "task").unwrap();
        s.update_session("s1", Some("completed"), Some("done"), Some(1.0), None, None).unwrap();
        // Update only summary, outcome should be preserved via COALESCE
        s.update_session("s1", None, Some("updated"), None, None, None).unwrap();
        let sessions = s.get_sessions(Some("proj")).unwrap();
        assert_eq!(sessions[0]["outcome"], "completed"); // preserved
        assert_eq!(sessions[0]["summary"], "updated"); // updated
    }
}
