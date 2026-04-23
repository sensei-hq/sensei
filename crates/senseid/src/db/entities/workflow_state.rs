use rusqlite::{params, OptionalExtension};
use super::super::Store;

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_workflow_state(
        &self,
        project: &str,
        phase: Option<&str>,
        plan: Option<&str>,
        task: Option<&str>,
        issue: Option<i64>,
        checkpoint: Option<&str>,
        rules_hash: Option<&str>,
    ) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO workflow_state(project, active_phase, active_plan, active_task, active_issue, last_checkpoint, rules_hash, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(project) DO UPDATE SET
               active_phase=COALESCE(?2, active_phase),
               active_plan=COALESCE(?3, active_plan),
               active_task=COALESCE(?4, active_task),
               active_issue=COALESCE(?5, active_issue),
               last_checkpoint=COALESCE(?6, last_checkpoint),
               rules_hash=COALESCE(?7, rules_hash),
               updated_at=?8",
            params![project, phase, plan, task, issue, checkpoint, rules_hash, now],
        )?;
        Ok(())
    }

    pub fn get_workflow_state(&self, project: &str) -> rusqlite::Result<Option<serde_json::Value>> {
        self.conn.query_row(
            "SELECT active_phase, active_plan, active_task, active_issue, last_checkpoint, rules_hash, updated_at
             FROM workflow_state WHERE project=?1",
            params![project],
            |row| {
                Ok(serde_json::json!({
                    "project": project,
                    "active_phase": row.get::<_, Option<String>>(0)?,
                    "active_plan": row.get::<_, Option<String>>(1)?,
                    "active_task": row.get::<_, Option<String>>(2)?,
                    "active_issue": row.get::<_, Option<i64>>(3)?,
                    "last_checkpoint": row.get::<_, Option<String>>(4)?,
                    "rules_hash": row.get::<_, Option<String>>(5)?,
                    "updated_at": row.get::<_, String>(6)?,
                }))
            },
        ).optional()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    #[test]
    fn upsert_and_get() {
        let s = test_store();
        assert!(s.get_workflow_state("test").unwrap().is_none());

        s.upsert_workflow_state("test", Some("ideate"), None, None, None, None, None).unwrap();
        let state = s.get_workflow_state("test").unwrap().unwrap();
        assert_eq!(state["active_phase"], "ideate");
        assert!(state["active_task"].is_null());
    }

    #[test]
    fn partial_update_preserves_fields() {
        let s = test_store();
        s.upsert_workflow_state("test", Some("build"), Some("plan.md"), Some("task 1"), Some(42), Some("2026-04-17T12:00:00Z"), Some("hash123")).unwrap();
        // Update only phase
        s.upsert_workflow_state("test", Some("validate"), None, None, None, None, None).unwrap();

        let state = s.get_workflow_state("test").unwrap().unwrap();
        assert_eq!(state["active_phase"], "validate"); // updated
        assert_eq!(state["active_plan"], "plan.md"); // preserved
        assert_eq!(state["active_task"], "task 1"); // preserved
        assert_eq!(state["active_issue"], 42); // preserved
        assert_eq!(state["last_checkpoint"], "2026-04-17T12:00:00Z"); // preserved
        assert_eq!(state["rules_hash"], "hash123"); // preserved
    }

    #[test]
    fn full_update() {
        let s = test_store();
        s.upsert_workflow_state("test", Some("build"), Some("docs/wave1.md"), Some("feature #1"), Some(55), Some("2026-04-17T12:00:00Z"), Some("abc123")).unwrap();

        let state = s.get_workflow_state("test").unwrap().unwrap();
        assert_eq!(state["active_phase"], "build");
        assert_eq!(state["active_plan"], "docs/wave1.md");
        assert_eq!(state["active_task"], "feature #1");
        assert_eq!(state["active_issue"], 55);
        assert!(state["updated_at"].as_str().is_some());
    }

    #[test]
    fn multiple_projects_isolated() {
        let s = test_store();
        s.upsert_workflow_state("a", Some("ideate"), None, None, None, None, None).unwrap();
        s.upsert_workflow_state("b", Some("build"), None, None, Some(10), None, None).unwrap();

        let a = s.get_workflow_state("a").unwrap().unwrap();
        let b = s.get_workflow_state("b").unwrap().unwrap();
        assert_eq!(a["active_phase"], "ideate");
        assert_eq!(b["active_phase"], "build");
        assert_eq!(b["active_issue"], 10);
        assert!(a["active_issue"].is_null());
    }

    #[test]
    fn nonexistent_returns_none() {
        let s = test_store();
        assert!(s.get_workflow_state("does-not-exist").unwrap().is_none());
    }

    #[test]
    fn updated_at_changes() {
        let s = test_store();
        s.upsert_workflow_state("test", Some("ideate"), None, None, None, None, None).unwrap();
        let t1 = s.get_workflow_state("test").unwrap().unwrap()["updated_at"].as_str().unwrap().to_string();
        std::thread::sleep(std::time::Duration::from_millis(10));
        s.upsert_workflow_state("test", Some("analyze"), None, None, None, None, None).unwrap();
        let t2 = s.get_workflow_state("test").unwrap().unwrap()["updated_at"].as_str().unwrap().to_string();
        assert_ne!(t1, t2);
    }
}
