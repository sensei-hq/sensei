use rusqlite::params;
use super::super::Store;

impl Store {
    /// Exclude a repo: delete from repos table, clear indexed data, add to exclusion list.
    pub fn exclude_repo(&self, repo_id: &str, path: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO excluded_paths(path, repo_id, reason) VALUES(?1, ?2, 'user_excluded')",
            params![path, repo_id],
        )?;
        self.delete_repo(repo_id)?;
        Ok(())
    }

    /// Check if a path is excluded.
    pub fn is_excluded(&self, path: &str) -> bool {
        self.conn.query_row(
            "SELECT COUNT(*) FROM excluded_paths WHERE ?1 LIKE path || '%'",
            params![path],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0
    }

    /// List all exclusions.
    pub fn list_exclusions(&self) -> rusqlite::Result<Vec<(String, Option<String>, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, repo_id, excluded_at FROM excluded_paths ORDER BY excluded_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, String>(2)?))
        })?;
        rows.collect()
    }

    /// Remove an exclusion — path becomes discoverable again on next scan.
    pub fn remove_exclusion(&self, path: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM excluded_paths WHERE path = ?1", params![path])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Repo;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    fn seed_repo(s: &Store, id: &str, path: &str) {
        s.upsert_repo(&Repo {
            repo_id: id.into(), name: id.into(), path: path.into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec![], libs: vec![], tags: vec![], status: "active".into(),
            project_id: None, role: "unknown".into(), label: None,
        }).unwrap();
    }

    #[test]
    fn exclude_removes_from_repos() {
        let s = test_store();
        seed_repo(&s, "foo", "/tmp/foo");
        seed_repo(&s, "bar", "/tmp/bar");
        s.exclude_repo("foo", "/tmp/foo").unwrap();
        assert_eq!(s.list_repos().unwrap().len(), 1);
        assert!(s.get_repo("foo").unwrap().is_none());
    }

    #[test]
    fn excluded_path_listed() {
        let s = test_store();
        seed_repo(&s, "foo", "/tmp/foo");
        s.exclude_repo("foo", "/tmp/foo").unwrap();
        let excl = s.list_exclusions().unwrap();
        assert_eq!(excl.len(), 1);
        assert_eq!(excl[0].0, "/tmp/foo");
    }

    #[test]
    fn is_excluded_checks_children() {
        let s = test_store();
        seed_repo(&s, "foo", "/tmp/foo");
        s.exclude_repo("foo", "/tmp/foo").unwrap();
        assert!(s.is_excluded("/tmp/foo"));
        assert!(s.is_excluded("/tmp/foo/src/main.rs"));
        assert!(!s.is_excluded("/tmp/bar"));
    }

    #[test]
    fn remove_exclusion() {
        let s = test_store();
        seed_repo(&s, "foo", "/tmp/foo");
        s.exclude_repo("foo", "/tmp/foo").unwrap();
        s.remove_exclusion("/tmp/foo").unwrap();
        assert!(!s.is_excluded("/tmp/foo"));
        assert_eq!(s.list_exclusions().unwrap().len(), 0);
    }

    #[test]
    fn is_excluded_empty_returns_false() {
        let s = test_store();
        assert!(!s.is_excluded("/any/path"));
    }
}
