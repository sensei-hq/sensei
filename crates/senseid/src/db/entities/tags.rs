use rusqlite::params;
use super::super::Store;

impl Store {
    #[allow(dead_code)] // used in tests; will be called from API layer
    pub fn get_tags(&self, entity_type: &str, entity_id: &str) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT tag FROM tags WHERE entity_type = ?1 AND entity_id = ?2 ORDER BY tag"
        )?;
        let rows = stmt.query_map(params![entity_type, entity_id], |row| row.get(0))?;
        rows.collect()
    }

    pub fn add_tag(&self, entity_type: &str, entity_id: &str, tag: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tags(entity_type, entity_id, tag) VALUES(?1, ?2, ?3)",
            params![entity_type, entity_id, tag],
        )?;
        Ok(())
    }

    pub fn remove_tag(&self, entity_type: &str, entity_id: &str, tag: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM tags WHERE entity_type = ?1 AND entity_id = ?2 AND tag = ?3",
            params![entity_type, entity_id, tag],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Repo;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    fn seed_repo(s: &Store) {
        s.upsert_repo(&Repo {
            repo_id: "foo".into(), name: "foo".into(), path: "/tmp/foo".into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec![], libs: vec![], tags: vec![], status: "active".into(),
            project_id: None, role: "unknown".into(), label: None,
        }).unwrap();
    }

    #[test]
    fn add_and_get_tags() {
        let s = test_store();
        seed_repo(&s);
        s.add_tag("repo", "foo", "typescript").unwrap();
        s.add_tag("repo", "foo", "backend").unwrap();
        let tags = s.get_tags("repo", "foo").unwrap();
        assert_eq!(tags, vec!["backend", "typescript"]); // sorted
    }

    #[test]
    fn duplicate_tag_ignored() {
        let s = test_store();
        seed_repo(&s);
        s.add_tag("repo", "foo", "ts").unwrap();
        s.add_tag("repo", "foo", "ts").unwrap();
        assert_eq!(s.get_tags("repo", "foo").unwrap().len(), 1);
    }

    #[test]
    fn remove_tag() {
        let s = test_store();
        seed_repo(&s);
        s.add_tag("repo", "foo", "a").unwrap();
        s.add_tag("repo", "foo", "b").unwrap();
        s.remove_tag("repo", "foo", "a").unwrap();
        assert_eq!(s.get_tags("repo", "foo").unwrap(), vec!["b"]);
    }

    #[test]
    fn tags_isolate_by_entity() {
        let s = test_store();
        s.add_tag("repo", "r1", "shared").unwrap();
        s.add_tag("project", "p1", "shared").unwrap();
        assert_eq!(s.get_tags("repo", "r1").unwrap().len(), 1);
        assert_eq!(s.get_tags("project", "p1").unwrap().len(), 1);
        assert_eq!(s.get_tags("repo", "p1").unwrap().len(), 0);
    }

    #[test]
    fn get_tags_empty() {
        let s = test_store();
        assert_eq!(s.get_tags("repo", "nonexistent").unwrap().len(), 0);
    }

    #[test]
    fn remove_nonexistent_tag_is_noop() {
        let s = test_store();
        s.remove_tag("repo", "foo", "nope").unwrap(); // no error
    }
}
