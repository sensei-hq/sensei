use rusqlite::{params, Row};
use super::super::Store;

impl Store {
    pub fn insert_event(
        &self,
        id: &str,
        project: &str,
        session_id: Option<&str>,
        event_type: &str,
        data: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO events(id, project, session_id, event_type, data) VALUES(?1,?2,?3,?4,?5)",
            params![id, project, session_id, event_type, data],
        )?;
        Ok(())
    }

    pub fn list_events(
        &self,
        project: &str,
        event_type: Option<&str>,
        session_id: Option<&str>,
        limit: u32,
    ) -> rusqlite::Result<Vec<serde_json::Value>> {
        let mut sql = String::from(
            "SELECT id, project, session_id, event_type, data, created_at FROM events WHERE project=?1"
        );
        let mut param_idx = 2;
        if event_type.is_some() {
            sql.push_str(&format!(" AND event_type=?{}", param_idx));
            param_idx += 1;
        }
        if session_id.is_some() {
            sql.push_str(&format!(" AND session_id=?{}", param_idx));
        }
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {}", limit));

        let mut stmt = self.conn.prepare(&sql)?;

        let rows: Vec<serde_json::Value> = match (event_type, session_id) {
            (Some(et), Some(sid)) => {
                stmt.query_map(params![project, et, sid], Self::row_to_event)?
                    .filter_map(|r| r.ok()).collect()
            }
            (Some(et), None) => {
                stmt.query_map(params![project, et], Self::row_to_event)?
                    .filter_map(|r| r.ok()).collect()
            }
            (None, Some(sid)) => {
                stmt.query_map(params![project, sid], Self::row_to_event)?
                    .filter_map(|r| r.ok()).collect()
            }
            (None, None) => {
                stmt.query_map(params![project], Self::row_to_event)?
                    .filter_map(|r| r.ok()).collect()
            }
        };
        Ok(rows)
    }

    pub fn count_events(&self, project: &str, event_type: Option<&str>) -> rusqlite::Result<u64> {
        if let Some(et) = event_type {
            self.conn.query_row(
                "SELECT COUNT(*) FROM events WHERE project=?1 AND event_type=?2",
                params![project, et],
                |r| r.get(0),
            )
        } else {
            self.conn.query_row(
                "SELECT COUNT(*) FROM events WHERE project=?1",
                params![project],
                |r| r.get(0),
            )
        }
    }

    fn row_to_event(row: &Row) -> rusqlite::Result<serde_json::Value> {
        let data_str: String = row.get(4)?;
        let data: serde_json::Value = serde_json::from_str(&data_str)
            .unwrap_or(serde_json::json!(data_str));
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "project": row.get::<_, String>(1)?,
            "session_id": row.get::<_, Option<String>>(2)?,
            "event_type": row.get::<_, String>(3)?,
            "data": data,
            "created_at": row.get::<_, String>(5)?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    #[test]
    fn insert_and_list() {
        let s = test_store();
        s.insert_event("e1", "proj-a", Some("s1"), "phase_transition", r#"{"from":"ideate","to":"analyze"}"#).unwrap();
        s.insert_event("e2", "proj-a", Some("s1"), "tool_used", r#"{"tool":"search","is_mcp":true}"#).unwrap();
        s.insert_event("e3", "proj-a", Some("s1"), "turn", r#"{"classification":"new_request"}"#).unwrap();

        let events = s.list_events("proj-a", None, None, 50).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["event_type"], "turn"); // most recent first
    }

    #[test]
    fn filter_by_type() {
        let s = test_store();
        s.insert_event("e1", "proj", None, "phase_transition", "{}").unwrap();
        s.insert_event("e2", "proj", None, "tool_used", "{}").unwrap();
        s.insert_event("e3", "proj", None, "tool_used", "{}").unwrap();
        s.insert_event("e4", "proj", None, "turn", "{}").unwrap();

        let tool_events = s.list_events("proj", Some("tool_used"), None, 50).unwrap();
        assert_eq!(tool_events.len(), 2);
        assert!(tool_events.iter().all(|e| e["event_type"] == "tool_used"));
    }

    #[test]
    fn filter_by_session() {
        let s = test_store();
        s.insert_event("e1", "proj", Some("s1"), "turn", "{}").unwrap();
        s.insert_event("e2", "proj", Some("s2"), "turn", "{}").unwrap();
        s.insert_event("e3", "proj", Some("s1"), "turn", "{}").unwrap();

        let s1_events = s.list_events("proj", None, Some("s1"), 50).unwrap();
        assert_eq!(s1_events.len(), 2);
        assert!(s1_events.iter().all(|e| e["session_id"] == "s1"));
    }

    #[test]
    fn filter_by_type_and_session() {
        let s = test_store();
        s.insert_event("e1", "proj", Some("s1"), "turn", "{}").unwrap();
        s.insert_event("e2", "proj", Some("s1"), "tool_used", "{}").unwrap();
        s.insert_event("e3", "proj", Some("s2"), "turn", "{}").unwrap();

        let filtered = s.list_events("proj", Some("turn"), Some("s1"), 50).unwrap();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn limit_respected() {
        let s = test_store();
        for i in 0..10 {
            s.insert_event(&format!("e{}", i), "proj", None, "turn", "{}").unwrap();
        }
        assert_eq!(s.list_events("proj", None, None, 3).unwrap().len(), 3);
    }

    #[test]
    fn count_events() {
        let s = test_store();
        s.insert_event("e1", "proj", None, "turn", "{}").unwrap();
        s.insert_event("e2", "proj", None, "turn", "{}").unwrap();
        s.insert_event("e3", "proj", None, "tool_used", "{}").unwrap();

        assert_eq!(s.count_events("proj", None).unwrap(), 3);
        assert_eq!(s.count_events("proj", Some("turn")).unwrap(), 2);
        assert_eq!(s.count_events("proj", Some("tool_used")).unwrap(), 1);
        assert_eq!(s.count_events("proj", Some("nonexistent")).unwrap(), 0);
    }

    #[test]
    fn data_parsed_as_json() {
        let s = test_store();
        s.insert_event("e1", "proj", None, "locate", r#"{"tools":["search","get_callers"]}"#).unwrap();
        let events = s.list_events("proj", None, None, 1).unwrap();
        assert_eq!(events[0]["data"]["tools"][0], "search");
    }

    #[test]
    fn isolates_projects() {
        let s = test_store();
        s.insert_event("e1", "proj-x", None, "turn", "{}").unwrap();
        s.insert_event("e2", "proj-y", None, "turn", "{}").unwrap();
        assert_eq!(s.list_events("proj-x", None, None, 50).unwrap().len(), 1);
        assert_eq!(s.list_events("proj-y", None, None, 50).unwrap().len(), 1);
        assert_eq!(s.list_events("proj-z", None, None, 50).unwrap().len(), 0);
    }
}
