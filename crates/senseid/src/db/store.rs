use rusqlite::{Connection, params};
use std::path::Path;
use crate::types::{Project, Solution, SolutionRepo, IndexError};

/// Central SQLite store — single database for all sensei state.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (or create) the database at the given path.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Open an in-memory database (for tests).
    pub fn open_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects(
                repo_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                remote_url TEXT,
                indexed_at TEXT,
                last_error TEXT,
                duplicate_of TEXT,
                stack TEXT DEFAULT '[]',
                libs TEXT DEFAULT '[]',
                status TEXT DEFAULT 'active',
                last_commit_days INTEGER,
                commit_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS solutions(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                client TEXT,
                category TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS solution_repos(
                solution_id TEXT NOT NULL REFERENCES solutions(id) ON DELETE CASCADE,
                repo_id TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'unknown',
                label TEXT,
                PRIMARY KEY(solution_id, repo_id)
            );

            CREATE TABLE IF NOT EXISTS tags(
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY(entity_type, entity_id, tag)
            );

            CREATE TABLE IF NOT EXISTS index_jobs(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id TEXT NOT NULL UNIQUE,
                repo_path TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                attempts INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                error TEXT
            );

            CREATE TABLE IF NOT EXISTS index_errors(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                error TEXT NOT NULL,
                adapter TEXT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_errors_repo ON index_errors(repo_id);

            CREATE TABLE IF NOT EXISTS sessions(
                id TEXT PRIMARY KEY,
                repo_id TEXT NOT NULL,
                task TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                outcome TEXT,
                summary TEXT,
                cost REAL,
                tokens_in INTEGER,
                tokens_out INTEGER
            );

            CREATE TABLE IF NOT EXISTS lib_docs(
                id TEXT PRIMARY KEY,
                lib_name TEXT NOT NULL,
                title TEXT NOT NULL,
                url TEXT,
                local_path TEXT,
                summary TEXT NOT NULL DEFAULT '',
                content TEXT,
                source_type TEXT NOT NULL,
                component TEXT,
                indexed_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS lib_docs_lib ON lib_docs(lib_name);

            CREATE TABLE IF NOT EXISTS lib_meta(
                name TEXT PRIMARY KEY,
                source_type TEXT NOT NULL,
                base_url TEXT,
                used_by TEXT DEFAULT '[]',
                indexed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS project_configs(
                repo_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_type TEXT NOT NULL,
                parsed_data TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY(repo_id, file_path)
            );
            "
        )
    }

    // ── Projects ─────────────────────────────────────────────────────────────

    pub fn list_projects(&self) -> rusqlite::Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT repo_id, name, path, remote_url, indexed_at, last_error, duplicate_of, stack, libs, status FROM projects ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            let stack_json: String = row.get(7)?;
            let libs_json: String = row.get(8)?;
            Ok(Project {
                repo_id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                remote_url: row.get(3)?,
                indexed_at: row.get(4)?,
                last_error: row.get(5)?,
                duplicate_of: row.get(6)?,
                stack: serde_json::from_str(&stack_json).unwrap_or_default(),
                libs: serde_json::from_str(&libs_json).unwrap_or_default(),
                tags: vec![], // loaded separately
                status: row.get(9)?,
            })
        })?;
        let mut projects: Vec<Project> = rows.collect::<Result<_, _>>()?;
        // Load tags
        for p in &mut projects {
            p.tags = self.get_tags("project", &p.repo_id)?;
        }
        Ok(projects)
    }

    pub fn upsert_project(&self, p: &Project) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO projects(repo_id, name, path, remote_url, indexed_at, last_error, duplicate_of, stack, libs, status)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(repo_id) DO UPDATE SET
               name=excluded.name, path=excluded.path, remote_url=excluded.remote_url,
               indexed_at=excluded.indexed_at, last_error=excluded.last_error,
               duplicate_of=excluded.duplicate_of, stack=excluded.stack,
               libs=excluded.libs, status=excluded.status",
            params![
                p.repo_id, p.name, p.path, p.remote_url, p.indexed_at, p.last_error,
                p.duplicate_of, serde_json::to_string(&p.stack).unwrap(),
                serde_json::to_string(&p.libs).unwrap(), p.status
            ],
        )?;
        Ok(())
    }

    pub fn get_project(&self, repo_id: &str) -> rusqlite::Result<Option<Project>> {
        let projects = self.list_projects()?;
        Ok(projects.into_iter().find(|p| p.repo_id == repo_id))
    }

    pub fn delete_project(&self, repo_id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM projects WHERE repo_id = ?1", params![repo_id])?;
        self.conn.execute("DELETE FROM tags WHERE entity_type = 'project' AND entity_id = ?1", params![repo_id])?;
        Ok(())
    }

    pub fn mark_indexed(&self, repo_id: &str, new_libs: &[String]) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Merge new libs with existing (don't overwrite with smaller set from partial re-index)
        let existing_libs: Vec<String> = self.conn.query_row(
            "SELECT libs FROM projects WHERE repo_id = ?1",
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
            "UPDATE projects SET indexed_at = ?1, last_error = NULL, libs = ?2 WHERE repo_id = ?3",
            params![now, serde_json::to_string(&merged_vec).unwrap(), repo_id],
        )?;
        Ok(())
    }

    pub fn mark_indexed_timestamp(&self, repo_id: &str) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE projects SET indexed_at = ?1, last_error = NULL WHERE repo_id = ?2",
            params![now, repo_id],
        )?;
        Ok(())
    }

    pub fn mark_index_failed(&self, repo_id: &str, error: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE projects SET last_error = ?1 WHERE repo_id = ?2",
            params![error, repo_id],
        )?;
        Ok(())
    }

    // ── Tags ─────────────────────────────────────────────────────────────────

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

    // ── Solutions ────────────────────────────────────────────────────────────

    pub fn list_solutions(&self) -> rusqlite::Result<Vec<Solution>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, client, category, created_at, updated_at FROM solutions ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Solution {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                client: row.get(3)?,
                category: row.get(4)?,
                repos: vec![],
                tags: vec![],
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut solutions: Vec<Solution> = rows.collect::<Result<_, _>>()?;
        for s in &mut solutions {
            s.repos = self.get_solution_repos(&s.id)?;
            s.tags = self.get_tags("solution", &s.id)?;
        }
        Ok(solutions)
    }

    pub fn create_solution(&self, s: &Solution) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO solutions(id, name, description, client, category, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![s.id, s.name, s.description, s.client, s.category, now, now],
        )?;
        for r in &s.repos {
            self.conn.execute(
                "INSERT INTO solution_repos(solution_id, repo_id, role, label) VALUES(?1, ?2, ?3, ?4)",
                params![s.id, r.repo_id, r.role, r.label],
            )?;
        }
        Ok(())
    }

    pub fn delete_solution(&self, id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM solutions WHERE id = ?1", params![id])?;
        self.conn.execute("DELETE FROM tags WHERE entity_type = 'solution' AND entity_id = ?1", params![id])?;
        Ok(())
    }

    fn get_solution_repos(&self, solution_id: &str) -> rusqlite::Result<Vec<SolutionRepo>> {
        let mut stmt = self.conn.prepare(
            "SELECT repo_id, role, label FROM solution_repos WHERE solution_id = ?1"
        )?;
        let rows = stmt.query_map(params![solution_id], |row| {
            Ok(SolutionRepo {
                repo_id: row.get(0)?,
                role: row.get(1)?,
                label: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn add_repo_to_solution(&self, solution_id: &str, repo: &SolutionRepo) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO solution_repos(solution_id, repo_id, role, label) VALUES(?1, ?2, ?3, ?4)",
            params![solution_id, repo.repo_id, repo.role, repo.label],
        )?;
        Ok(())
    }

    pub fn remove_repo_from_solution(&self, solution_id: &str, repo_id: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM solution_repos WHERE solution_id = ?1 AND repo_id = ?2",
            params![solution_id, repo_id],
        )?;
        Ok(())
    }

    // ── Index Errors ─────────────────────────────────────────────────────────

    pub fn log_index_error(&self, err: &IndexError) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO index_errors(repo_id, file_path, error, adapter, timestamp) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![err.repo_id, err.file_path, err.error, err.adapter, err.timestamp],
        )?;
        Ok(())
    }

    pub fn get_index_errors(&self, repo_id: Option<&str>) -> rusqlite::Result<Vec<IndexError>> {
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<IndexError> {
            Ok(IndexError {
                repo_id: row.get(0)?,
                file_path: row.get(1)?,
                error: row.get(2)?,
                adapter: row.get(3)?,
                timestamp: row.get(4)?,
            })
        };

        match repo_id {
            Some(id) => {
                let mut stmt = self.conn.prepare(
                    "SELECT repo_id, file_path, error, adapter, timestamp FROM index_errors WHERE repo_id = ?1 ORDER BY timestamp DESC"
                )?;
                stmt.query_map(params![id], map_row)?.collect()
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT repo_id, file_path, error, adapter, timestamp FROM index_errors ORDER BY timestamp DESC LIMIT 200"
                )?;
                stmt.query_map([], map_row)?.collect()
            }
        }
    }

    pub fn clear_index_errors(&self, repo_id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM index_errors WHERE repo_id = ?1", params![repo_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Store {
        Store::open_memory().unwrap()
    }

    fn make_project(id: &str, path: &str) -> Project {
        Project {
            repo_id: id.into(),
            name: id.into(),
            path: path.into(),
            remote_url: None,
            indexed_at: None,
            last_error: None,
            duplicate_of: None,
            stack: vec!["typescript".into()],
            libs: vec![],
            tags: vec![],
            status: "active".into(),
        }
    }

    #[test]
    fn create_and_list_projects() {
        let s = test_store();
        s.upsert_project(&make_project("foo", "/tmp/foo")).unwrap();
        s.upsert_project(&make_project("bar", "/tmp/bar")).unwrap();
        let projects = s.list_projects().unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].repo_id, "bar"); // sorted by name
    }

    #[test]
    fn upsert_updates_existing() {
        let s = test_store();
        s.upsert_project(&make_project("foo", "/tmp/foo")).unwrap();
        let mut p2 = make_project("foo", "/tmp/foo");
        p2.indexed_at = Some("2026-04-12".into());
        s.upsert_project(&p2).unwrap();
        let projects = s.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].indexed_at.as_deref(), Some("2026-04-12"));
    }

    #[test]
    fn delete_project() {
        let s = test_store();
        s.upsert_project(&make_project("foo", "/tmp/foo")).unwrap();
        s.add_tag("project", "foo", "test").unwrap();
        s.delete_project("foo").unwrap();
        assert_eq!(s.list_projects().unwrap().len(), 0);
        assert_eq!(s.get_tags("project", "foo").unwrap().len(), 0);
    }

    #[test]
    fn mark_indexed() {
        let s = test_store();
        s.upsert_project(&make_project("foo", "/tmp/foo")).unwrap();
        s.mark_indexed("foo", &["zod".into(), "hono".into()]).unwrap();
        let p = s.get_project("foo").unwrap().unwrap();
        assert!(p.indexed_at.is_some());
        assert_eq!(p.libs, vec!["hono", "zod"]); // sorted (BTreeSet merge)
        assert!(p.last_error.is_none());
    }

    #[test]
    fn tags_crud() {
        let s = test_store();
        s.upsert_project(&make_project("foo", "/tmp/foo")).unwrap();
        s.add_tag("project", "foo", "typescript").unwrap();
        s.add_tag("project", "foo", "backend").unwrap();
        s.add_tag("project", "foo", "typescript").unwrap(); // duplicate — ignored
        let tags = s.get_tags("project", "foo").unwrap();
        assert_eq!(tags, vec!["backend", "typescript"]);
        s.remove_tag("project", "foo", "backend").unwrap();
        assert_eq!(s.get_tags("project", "foo").unwrap(), vec!["typescript"]);
    }

    #[test]
    fn solution_crud() {
        let s = test_store();
        s.upsert_project(&make_project("api", "/tmp/api")).unwrap();
        s.upsert_project(&make_project("ui", "/tmp/ui")).unwrap();

        let sol = Solution {
            id: "sol-1".into(),
            name: "Acme Platform".into(),
            description: Some("Main product".into()),
            client: Some("Acme Corp".into()),
            category: "active".into(),
            repos: vec![
                SolutionRepo { repo_id: "api".into(), role: "backend".into(), label: Some("API".into()) },
                SolutionRepo { repo_id: "ui".into(), role: "frontend".into(), label: None },
            ],
            tags: vec![],
            created_at: None,
            updated_at: None,
        };
        s.create_solution(&sol).unwrap();

        let solutions = s.list_solutions().unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].name, "Acme Platform");
        assert_eq!(solutions[0].client, Some("Acme Corp".into()));
        assert_eq!(solutions[0].repos.len(), 2);
        assert_eq!(solutions[0].repos[0].role, "backend");
    }

    #[test]
    fn solution_add_remove_repo() {
        let s = test_store();
        s.upsert_project(&make_project("api", "/tmp/api")).unwrap();
        let sol = Solution {
            id: "sol-1".into(), name: "Test".into(), description: None,
            client: None, category: "active".into(), repos: vec![],
            tags: vec![], created_at: None, updated_at: None,
        };
        s.create_solution(&sol).unwrap();
        s.add_repo_to_solution("sol-1", &SolutionRepo {
            repo_id: "api".into(), role: "backend".into(), label: None,
        }).unwrap();
        let solutions = s.list_solutions().unwrap();
        assert_eq!(solutions[0].repos.len(), 1);
        s.remove_repo_from_solution("sol-1", "api").unwrap();
        let solutions = s.list_solutions().unwrap();
        assert_eq!(solutions[0].repos.len(), 0);
    }

    #[test]
    fn solution_tags() {
        let s = test_store();
        let sol = Solution {
            id: "sol-1".into(), name: "Test".into(), description: None,
            client: None, category: "active".into(), repos: vec![],
            tags: vec![], created_at: None, updated_at: None,
        };
        s.create_solution(&sol).unwrap();
        s.add_tag("solution", "sol-1", "production").unwrap();
        let solutions = s.list_solutions().unwrap();
        assert_eq!(solutions[0].tags, vec!["production"]);
    }

    #[test]
    fn delete_solution_cascades() {
        let s = test_store();
        s.upsert_project(&make_project("api", "/tmp/api")).unwrap();
        let sol = Solution {
            id: "sol-1".into(), name: "Test".into(), description: None,
            client: None, category: "active".into(),
            repos: vec![SolutionRepo { repo_id: "api".into(), role: "backend".into(), label: None }],
            tags: vec![], created_at: None, updated_at: None,
        };
        s.create_solution(&sol).unwrap();
        s.add_tag("solution", "sol-1", "test").unwrap();
        s.delete_solution("sol-1").unwrap();
        assert_eq!(s.list_solutions().unwrap().len(), 0);
        assert_eq!(s.get_tags("solution", "sol-1").unwrap().len(), 0);
    }

    #[test]
    fn index_errors() {
        let s = test_store();
        s.log_index_error(&IndexError {
            repo_id: "foo".into(),
            file_path: "src/bad.ts".into(),
            error: "SyntaxError at line 5".into(),
            adapter: Some("typescript".into()),
            timestamp: "2026-04-12T12:00:00Z".into(),
        }).unwrap();
        s.log_index_error(&IndexError {
            repo_id: "foo".into(),
            file_path: "src/broken.py".into(),
            error: "IndentationError".into(),
            adapter: Some("python".into()),
            timestamp: "2026-04-12T12:01:00Z".into(),
        }).unwrap();
        s.log_index_error(&IndexError {
            repo_id: "bar".into(),
            file_path: "src/x.rs".into(),
            error: "Parse error".into(),
            adapter: Some("rust".into()),
            timestamp: "2026-04-12T12:02:00Z".into(),
        }).unwrap();

        // All errors
        let all = s.get_index_errors(None).unwrap();
        assert_eq!(all.len(), 3);

        // Errors for foo
        let foo_errs = s.get_index_errors(Some("foo")).unwrap();
        assert_eq!(foo_errs.len(), 2);

        // Clear foo errors
        s.clear_index_errors("foo").unwrap();
        assert_eq!(s.get_index_errors(Some("foo")).unwrap().len(), 0);
        assert_eq!(s.get_index_errors(Some("bar")).unwrap().len(), 1);
    }

    #[test]
    fn mark_index_failed() {
        let s = test_store();
        s.upsert_project(&make_project("foo", "/tmp/foo")).unwrap();
        s.mark_index_failed("foo", "Kuzu connection error").unwrap();
        let p = s.get_project("foo").unwrap().unwrap();
        assert_eq!(p.last_error.as_deref(), Some("Kuzu connection error"));
    }
}
