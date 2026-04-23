use rusqlite::Connection;
use std::path::Path;

/// Central SQLite store — single database for all sensei state.
pub struct Store {
    pub(crate) conn: Connection,
}

impl Store {
    /// Get a reference to the underlying connection (for direct queries).
    pub fn conn_ref(&self) -> &Connection { &self.conn }

    /// Open (or create) the database at the given path.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
        let store = Self { conn };
        store.init_schema()?;
        store.migrate()?;
        Ok(store)
    }

    /// Open an in-memory database (for tests).
    #[allow(dead_code)]
    pub fn open_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        store.migrate()?;
        Ok(store)
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS repos(
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
                project_id TEXT,
                role TEXT NOT NULL DEFAULT 'unknown',
                label TEXT,
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS projects(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                client TEXT,
                category TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
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

            CREATE TABLE IF NOT EXISTS config(
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS scanned_roots(
                path TEXT PRIMARY KEY,
                created_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS excluded_paths(
                path TEXT PRIMARY KEY,
                repo_id TEXT,
                reason TEXT,
                excluded_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS detected_patterns(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                pattern_type TEXT NOT NULL,
                instance_count INTEGER NOT NULL DEFAULT 0,
                instances TEXT NOT NULL DEFAULT '[]',
                project TEXT NOT NULL,
                detected_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_patterns_project ON detected_patterns(project);

            CREATE TABLE IF NOT EXISTS events(
                id TEXT PRIMARY KEY,
                project TEXT NOT NULL,
                session_id TEXT,
                event_type TEXT NOT NULL,
                data TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_events_project ON events(project, created_at);
            CREATE INDEX IF NOT EXISTS idx_events_type ON events(project, event_type);
            CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);

            CREATE TABLE IF NOT EXISTS workflow_state(
                project TEXT PRIMARY KEY,
                active_phase TEXT,
                active_plan TEXT,
                active_task TEXT,
                active_issue INTEGER,
                last_checkpoint TEXT,
                rules_hash TEXT,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "
        )
    }

    /// Add columns that don't exist yet. Safe to run on every open.
    fn migrate(&self) -> rusqlite::Result<()> {
        let has_old_projects: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='projects' AND sql LIKE '%repo_id TEXT PRIMARY KEY%'",
            [], |row| row.get(0),
        )?;
        let has_repos_table: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='repos'",
            [], |row| row.get(0),
        )?;

        if has_old_projects && !has_repos_table {
            self.conn.execute_batch("
                ALTER TABLE projects RENAME TO repos;
                ALTER TABLE repos ADD COLUMN project_id TEXT;
                ALTER TABLE repos ADD COLUMN role TEXT NOT NULL DEFAULT 'unknown';
                ALTER TABLE repos ADD COLUMN label TEXT;
                ALTER TABLE repos ADD COLUMN metadata TEXT DEFAULT '{}';
            ")?;

            let has_solution_repos: bool = self.conn.query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='solution_repos'",
                [], |row| row.get(0),
            )?;
            if has_solution_repos {
                self.conn.execute_batch("
                    UPDATE repos SET
                        project_id = (SELECT sr.solution_id FROM solution_repos sr WHERE sr.repo_id = repos.repo_id LIMIT 1),
                        role = COALESCE((SELECT sr.role FROM solution_repos sr WHERE sr.repo_id = repos.repo_id LIMIT 1), 'unknown'),
                        label = (SELECT sr.label FROM solution_repos sr WHERE sr.repo_id = repos.repo_id LIMIT 1);
                    DROP TABLE IF EXISTS solution_repos;
                ")?;
            }

            let has_old_solutions: bool = self.conn.query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='solutions'",
                [], |row| row.get(0),
            )?;
            let has_new_projects: bool = self.conn.query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='projects'",
                [], |row| row.get(0),
            )?;
            if has_old_solutions && !has_new_projects {
                self.conn.execute_batch("ALTER TABLE solutions RENAME TO projects;")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_opens_in_memory() {
        let store = Store::open_memory().unwrap();
        let projects = store.list_projects().unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn conn_ref_accessible() {
        let store = Store::open_memory().unwrap();
        let _conn = store.conn_ref();
    }
}
