use rusqlite::params;
use crate::types::Repo;
use super::super::Store;

impl Store {
    pub fn list_repos(&self) -> rusqlite::Result<Vec<Repo>> {
        let mut stmt = self.conn.prepare(
            "SELECT repo_id, name, path, remote_url, indexed_at, last_error, duplicate_of, stack, libs, status, project_id, role, label FROM repos ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            let stack_json: String = row.get(7)?;
            let libs_json: String = row.get(8)?;
            Ok(Repo {
                repo_id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                remote_url: row.get(3)?,
                indexed_at: row.get(4)?,
                last_error: row.get(5)?,
                duplicate_of: row.get(6)?,
                stack: serde_json::from_str(&stack_json).unwrap_or_default(),
                libs: serde_json::from_str(&libs_json).unwrap_or_default(),
                tags: vec![],
                status: row.get(9)?,
                project_id: row.get(10)?,
                role: row.get::<_, String>(11).unwrap_or_else(|_| "unknown".into()),
                label: row.get(12)?,
            })
        })?;
        let mut repos: Vec<Repo> = rows.collect::<Result<_, _>>()?;

        // Batch load all repo tags in one query (avoid N+1)
        let mut tag_stmt = self.conn.prepare(
            "SELECT entity_id, tag FROM tags WHERE entity_type = 'repo' ORDER BY entity_id, tag"
        )?;
        let tag_rows = tag_stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut tag_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for row in tag_rows.flatten() {
            tag_map.entry(row.0).or_default().push(row.1);
        }
        for p in &mut repos {
            p.tags = tag_map.remove(&p.repo_id).unwrap_or_default();
        }
        Ok(repos)
    }

    pub fn upsert_repo(&self, p: &Repo) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO repos(repo_id, name, path, remote_url, indexed_at, last_error, duplicate_of, stack, libs, status, project_id, role, label)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(repo_id) DO UPDATE SET
               name=excluded.name, path=excluded.path, remote_url=excluded.remote_url,
               indexed_at=excluded.indexed_at, last_error=excluded.last_error,
               duplicate_of=excluded.duplicate_of, stack=excluded.stack,
               libs=excluded.libs, status=excluded.status,
               project_id=excluded.project_id, role=excluded.role, label=excluded.label",
            params![
                p.repo_id, p.name, p.path, p.remote_url, p.indexed_at, p.last_error,
                p.duplicate_of, serde_json::to_string(&p.stack).unwrap(),
                serde_json::to_string(&p.libs).unwrap(), p.status,
                p.project_id, p.role, p.label
            ],
        )?;
        Ok(())
    }

    /// Quick repo registration with just id, name, path.
    pub fn upsert_repo_basic(&self, repo_id: &str, name: &str, path: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO repos(repo_id, name, path, status, stack, libs)
             VALUES(?1, ?2, ?3, 'active', '[]', '[]')
             ON CONFLICT(repo_id) DO UPDATE SET name=excluded.name, path=excluded.path",
            rusqlite::params![repo_id, name, path],
        )?;
        Ok(())
    }

    pub fn get_repo(&self, repo_id: &str) -> rusqlite::Result<Option<Repo>> {
        let repos = self.list_repos()?;
        Ok(repos.into_iter().find(|p| p.repo_id == repo_id))
    }

    /// Store repo-level metadata (icon, external links, summary).
    pub fn set_repo_metadata(&self, repo_id: &str, metadata: &serde_json::Value) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE repos SET metadata = ?1 WHERE repo_id = ?2",
            params![serde_json::to_string(metadata).unwrap_or_default(), repo_id],
        )?;
        Ok(())
    }

    /// Assign a repo to a project with a role.
    pub fn set_repo_project(&self, repo_id: &str, project_id: &str, role: &str, label: Option<&str>) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE repos SET project_id = ?1, role = ?2, label = ?3 WHERE repo_id = ?4",
            params![project_id, role, label, repo_id],
        )?;
        Ok(())
    }

    /// Remove a repo from its project (make standalone).
    pub fn clear_repo_project(&self, repo_id: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE repos SET project_id = NULL, role = 'unknown', label = NULL WHERE repo_id = ?1",
            params![repo_id],
        )?;
        Ok(())
    }

    pub fn delete_repo(&self, repo_id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM repos WHERE repo_id = ?1", params![repo_id])?;
        self.conn.execute("DELETE FROM tags WHERE entity_type = 'repo' AND entity_id = ?1", params![repo_id])?;
        Ok(())
    }

    pub fn mark_indexed(&self, repo_id: &str, new_libs: &[String]) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Merge new libs with existing (don't overwrite with smaller set from partial re-index)
        let existing_libs: Vec<String> = self.conn.query_row(
            "SELECT libs FROM repos WHERE repo_id = ?1",
            params![repo_id],
            |row| {
                let json: String = row.get(0)?;
                Ok(serde_json::from_str::<Vec<String>>(&json).unwrap_or_default())
            },
        ).unwrap_or_default();

        let mut merged: std::collections::BTreeSet<String> = existing_libs.into_iter().collect();
        merged.extend(new_libs.iter().cloned());
        let merged_vec: Vec<String> = merged.into_iter().collect();

        self.conn.execute(
            "UPDATE repos SET indexed_at = ?1, last_error = NULL, libs = ?2 WHERE repo_id = ?3",
            params![now, serde_json::to_string(&merged_vec).unwrap(), repo_id],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn mark_indexed_timestamp(&self, repo_id: &str) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE repos SET indexed_at = ?1, last_error = NULL WHERE repo_id = ?2",
            params![now, repo_id],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn mark_index_failed(&self, repo_id: &str, error: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE repos SET last_error = ?1 WHERE repo_id = ?2",
            params![error, repo_id],
        )?;
        Ok(())
    }

    /// Get the file path for a repo (needed for state.yaml sync).
    pub fn get_repo_path(&self, repo_id_or_name: &str) -> rusqlite::Result<Option<String>> {
        self.conn.query_row(
            "SELECT path FROM repos WHERE name=?1 OR repo_id=?1",
            params![repo_id_or_name],
            |row| row.get(0),
        ).optional()
    }
}

use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store { Store::open_memory().unwrap() }

    fn make_repo(id: &str, path: &str) -> Repo {
        Repo {
            repo_id: id.into(), name: id.into(), path: path.into(),
            remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
            stack: vec!["typescript".into()], libs: vec![], tags: vec![],
            status: "active".into(), project_id: None, role: "unknown".into(), label: None,
        }
    }

    #[test]
    fn create_and_list_repos() {
        let s = test_store();
        s.upsert_repo(&make_repo("foo", "/tmp/foo")).unwrap();
        s.upsert_repo(&make_repo("bar", "/tmp/bar")).unwrap();
        let repos = s.list_repos().unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].repo_id, "bar"); // sorted by name
    }

    #[test]
    fn upsert_updates_existing() {
        let s = test_store();
        s.upsert_repo(&make_repo("foo", "/tmp/foo")).unwrap();
        let mut p2 = make_repo("foo", "/tmp/foo");
        p2.indexed_at = Some("2026-04-12".into());
        s.upsert_repo(&p2).unwrap();
        let repos = s.list_repos().unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].indexed_at.as_deref(), Some("2026-04-12"));
    }

    #[test]
    fn delete_repo_removes_tags() {
        let s = test_store();
        s.upsert_repo(&make_repo("foo", "/tmp/foo")).unwrap();
        s.add_tag("repo", "foo", "test").unwrap();
        s.delete_repo("foo").unwrap();
        assert_eq!(s.list_repos().unwrap().len(), 0);
        assert_eq!(s.get_tags("repo", "foo").unwrap().len(), 0);
    }

    #[test]
    fn mark_indexed_merges_libs() {
        let s = test_store();
        s.upsert_repo(&make_repo("foo", "/tmp/foo")).unwrap();
        s.mark_indexed("foo", &["zod".into(), "hono".into()]).unwrap();
        let p = s.get_repo("foo").unwrap().unwrap();
        assert!(p.indexed_at.is_some());
        assert_eq!(p.libs, vec!["hono", "zod"]); // sorted (BTreeSet merge)
        assert!(p.last_error.is_none());
    }

    #[test]
    fn mark_index_failed_sets_error() {
        let s = test_store();
        s.upsert_repo(&make_repo("foo", "/tmp/foo")).unwrap();
        s.mark_index_failed("foo", "connection error").unwrap();
        let p = s.get_repo("foo").unwrap().unwrap();
        assert_eq!(p.last_error.as_deref(), Some("connection error"));
    }

    #[test]
    fn upsert_repo_basic_creates_minimal() {
        let s = test_store();
        s.upsert_repo_basic("r1", "my-repo", "/tmp/r1").unwrap();
        let r = s.get_repo("r1").unwrap().unwrap();
        assert_eq!(r.name, "my-repo");
        assert_eq!(r.path, "/tmp/r1");
        assert_eq!(r.status, "active");
    }

    #[test]
    fn set_repo_metadata() {
        let s = test_store();
        s.upsert_repo(&make_repo("foo", "/tmp/foo")).unwrap();
        let meta = serde_json::json!({"icon": "🔧"});
        s.set_repo_metadata("foo", &meta).unwrap();
        // Metadata is stored but not returned by list_repos (it uses the SELECT without metadata)
    }

    #[test]
    fn set_and_clear_repo_project() {
        let s = test_store();
        s.upsert_repo(&make_repo("api", "/tmp/api")).unwrap();
        s.set_repo_project("api", "proj-1", "backend", Some("API")).unwrap();
        let r = s.get_repo("api").unwrap().unwrap();
        assert_eq!(r.project_id.as_deref(), Some("proj-1"));
        assert_eq!(r.role, "backend");
        assert_eq!(r.label.as_deref(), Some("API"));

        s.clear_repo_project("api").unwrap();
        let r = s.get_repo("api").unwrap().unwrap();
        assert!(r.project_id.is_none());
        assert_eq!(r.role, "unknown");
    }

    #[test]
    fn get_repo_path_by_name_or_id() {
        let s = test_store();
        s.upsert_repo_basic("test-repo", "test-project", "/home/user/project").unwrap();
        assert_eq!(s.get_repo_path("test-project").unwrap(), Some("/home/user/project".to_string()));
        assert_eq!(s.get_repo_path("test-repo").unwrap(), Some("/home/user/project".to_string()));
        assert_eq!(s.get_repo_path("nonexistent").unwrap(), None);
    }

    #[test]
    fn mark_indexed_timestamp_clears_error() {
        let s = test_store();
        s.upsert_repo(&make_repo("foo", "/tmp/foo")).unwrap();
        s.mark_index_failed("foo", "err").unwrap();
        s.mark_indexed_timestamp("foo").unwrap();
        let r = s.get_repo("foo").unwrap().unwrap();
        assert!(r.indexed_at.is_some());
        assert!(r.last_error.is_none());
    }
}
