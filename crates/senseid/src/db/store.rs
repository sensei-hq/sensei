use rusqlite::{Connection, params, OptionalExtension, Row};
use std::path::Path;
use crate::types::{Project, Solution, SolutionRepo, IndexError};

/// Central SQLite store — single database for all sensei state.
pub struct Store {
    conn: Connection,
}

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
    /// Get a reference to the underlying connection (for direct queries).
    pub fn conn_ref(&self) -> &Connection { &self.conn }

    /// Open (or create) the database at the given path.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Open an in-memory database (for tests).
    #[allow(dead_code)]
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

    /// Quick project registration with just id, name, path.
    pub fn upsert_project_basic(&self, repo_id: &str, name: &str, path: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO projects(repo_id, name, path, status, stack, libs)
             VALUES(?1, ?2, ?3, 'active', '[]', '[]')
             ON CONFLICT(repo_id) DO UPDATE SET name=excluded.name, path=excluded.path",
            rusqlite::params![repo_id, name, path],
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

    /// Exclude a project: delete from projects, clear indexed data, add to exclusion list.
    pub fn exclude_project(&self, repo_id: &str, path: &str) -> rusqlite::Result<()> {
        // Add to exclusion list
        self.conn.execute(
            "INSERT OR REPLACE INTO excluded_paths(path, repo_id, reason) VALUES(?1, ?2, 'user_excluded')",
            params![path, repo_id],
        )?;
        // Remove from projects
        self.delete_project(repo_id)?;
        Ok(())
    }

    /// Check if a path is excluded.
    pub fn is_excluded(&self, path: &str) -> bool {
        // Check exact match or if path starts with any excluded path
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

    #[allow(dead_code)]
    pub fn mark_indexed_timestamp(&self, repo_id: &str) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE projects SET indexed_at = ?1, last_error = NULL WHERE repo_id = ?2",
            params![now, repo_id],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
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
        // Use OR IGNORE to prevent duplicate creation
        self.conn.execute(
            "INSERT OR IGNORE INTO solutions(id, name, description, client, category, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![s.id, s.name, s.description, s.client, s.category, now, now],
        )?;
        for r in &s.repos {
            self.conn.execute(
                "INSERT OR REPLACE INTO solution_repos(solution_id, repo_id, role, label) VALUES(?1, ?2, ?3, ?4)",
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
            "SELECT sr.repo_id, sr.role, sr.label, p.path
             FROM solution_repos sr
             LEFT JOIN projects p ON p.repo_id = sr.repo_id
             WHERE sr.solution_id = ?1"
        )?;
        let rows = stmt.query_map(params![solution_id], |row| {
            Ok(SolutionRepo {
                repo_id: row.get(0)?,
                role: row.get(1)?,
                label: row.get(2)?,
                path: row.get(3)?,
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

    // ── Config (key-value preferences) ─────────────────────────────────────

    pub fn get_config(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn.query_row(
            "SELECT value FROM config WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).optional()
    }

    pub fn set_config(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO config(key, value) VALUES(?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_config(&self, key: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM config WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn execute_raw(&self, sql: &str) -> rusqlite::Result<()> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn get_all_config(&self) -> rusqlite::Result<std::collections::HashMap<String, String>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM config")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for r in rows { if let Ok((k, v)) = r { map.insert(k, v); } }
        Ok(map)
    }

    // ── Sessions ─────────────────────────────────────────────────────────



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

    // ── Detected Patterns ──────────────────────────────────────────────────

    /// Detect patterns by naming convention from hierarchy_nodes in the graph.
    /// Scans for suffixes like Adapter, Factory, Observer, etc.
    /// Returns detected patterns and stores them in detected_patterns table.
    pub fn detect_patterns_from_graph(&self, graph: &rusqlite::Connection, project: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
        let suffixes = [
            ("Adapter", "adapter"), ("Factory", "factory"), ("Observer", "observer"),
            ("Builder", "builder"), ("Strategy", "strategy"), ("Handler", "handler"),
            ("Middleware", "middleware"), ("Provider", "provider"), ("Decorator", "decorator"),
            ("Worker", "worker"), ("Hook", "hook"), ("Plugin", "plugin"),
            ("Controller", "controller"), ("Service", "service"), ("Repository", "repository"),
        ];

        // Clear existing detections for this project
        self.conn.execute("DELETE FROM detected_patterns WHERE project=?1", params![project])?;

        let mut results = Vec::new();

        for (suffix, pattern_type) in &suffixes {
            let like_pattern = format!("%{}", suffix);
            let mut stmt = graph.prepare(
                "SELECT name, file, kind FROM hierarchy_nodes WHERE project=?1 AND name LIKE ?2 AND kind IN ('class','struct','interface','type','component')"
            ).map_err(|e| rusqlite::Error::QueryReturnedNoRows)?; // map graph error

            let instances: Vec<serde_json::Value> = stmt.query_map(
                rusqlite::params![project, like_pattern],
                |row| {
                    Ok(serde_json::json!({
                        "name": row.get::<_, String>(0)?,
                        "file": row.get::<_, Option<String>>(1)?,
                        "kind": row.get::<_, String>(2)?,
                    }))
                }
            ).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?
            .filter_map(|r| r.ok())
            .collect();

            if instances.len() >= 2 {
                let id = format!("pattern:{}:{}", project, pattern_type);
                let name = format!("{}-pattern", pattern_type);
                let instances_json = serde_json::to_string(&instances).unwrap_or_default();

                self.conn.execute(
                    "INSERT OR REPLACE INTO detected_patterns(id, name, pattern_type, instance_count, instances, project) VALUES(?1,?2,?3,?4,?5,?6)",
                    params![id, name, pattern_type, instances.len() as i64, instances_json, project],
                )?;

                results.push(serde_json::json!({
                    "name": name,
                    "pattern_type": pattern_type,
                    "instance_count": instances.len(),
                    "instances": instances,
                }));
            }
        }

        Ok(results)
    }

    pub fn list_detected_patterns(&self, project: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, pattern_type, instance_count, instances FROM detected_patterns WHERE project=?1"
        )?;
        let rows = stmt.query_map(params![project], |row| {
            let instances_str: String = row.get(3)?;
            let instances: serde_json::Value = serde_json::from_str(&instances_str)
                .unwrap_or(serde_json::json!([]));
            Ok(serde_json::json!({
                "name": row.get::<_, String>(0)?,
                "pattern_type": row.get::<_, String>(1)?,
                "instance_count": row.get::<_, i64>(2)?,
                "instances": instances,
            }))
        })?;
        rows.collect()
    }

    /// Check if a specific symbol belongs to a detected pattern.
    pub fn get_pattern_for(&self, project: &str, symbol_name: &str) -> rusqlite::Result<Option<serde_json::Value>> {
        let patterns = self.list_detected_patterns(project)?;
        for pattern in patterns {
            if let Some(instances) = pattern["instances"].as_array() {
                for inst in instances {
                    if inst["name"].as_str() == Some(symbol_name) {
                        return Ok(Some(serde_json::json!({
                            "pattern_name": pattern["name"],
                            "pattern_type": pattern["pattern_type"],
                            "instance_count": pattern["instance_count"],
                            "role": "instance",
                            "instances": pattern["instances"],
                        })));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Find genuine code duplicates — filters out noise (common names, pattern instances).
    ///
    /// Three categories of results:
    /// - "true_duplicate": identical non-trivial signature in different files, not part of a pattern
    /// - "potential_pattern": same name across files, possibly an undocumented pattern (should be tagged)
    /// - "suspicious": same non-trivial name in different dirs, worth investigating
    pub fn find_duplicates(&self, graph: &rusqlite::Connection, project: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
        // Names to exclude — too generic to be meaningful duplicates
        let noise_names: Vec<&str> = vec![
            "__init__", "new", "default", "main", "run", "start", "stop",
            "get", "set", "close", "open", "read", "write", "send", "init",
            "clone", "drop", "from", "into", "try_from", "try_into",
            "fmt", "display", "debug", "serialize", "deserialize",
            "to_string", "as_ref", "as_mut", "eq", "hash", "cmp",
            "setup", "teardown", "before", "after", "test", "it", "describe",
            "empty", "len", "is_empty", "push", "pop", "clear",
            "build", "create", "update", "delete", "handle", "process",
            "render", "load", "save", "parse", "validate",
        ];

        // Get detected patterns so we can exclude pattern instances
        let patterns = self.list_detected_patterns(project).unwrap_or_default();
        let pattern_instance_names: Vec<String> = patterns.iter()
            .flat_map(|p| {
                p["instances"].as_array().unwrap_or(&vec![]).iter()
                    .filter_map(|i| i["name"].as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut results = Vec::new();

        // 1. Identical non-trivial signatures in different files
        let sig_query = graph.prepare(
            "SELECT a.name, a.file, a.sig, b.name, b.file, b.sig
             FROM hierarchy_nodes a
             JOIN hierarchy_nodes b ON a.sig = b.sig AND a.id < b.id
             WHERE a.project = ?1 AND b.project = ?1
             AND a.kind IN ('function','method') AND b.kind IN ('function','method')
             AND a.sig IS NOT NULL AND length(a.sig) > 20
             AND a.file != b.file"
        );

        if let Ok(mut stmt) = sig_query {
            let sig_dups: Vec<serde_json::Value> = stmt.query_map(
                rusqlite::params![project],
                |row| {
                    let name_a: String = row.get(0)?;
                    let name_b: String = row.get(3)?;
                    Ok((name_a, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?,
                        name_b, row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?))
                }
            ).ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|(name_a, _, _, name_b, _, _)| {
                // Exclude noise names
                let a_lower = name_a.to_lowercase();
                let b_lower = name_b.to_lowercase();
                !noise_names.contains(&a_lower.as_str()) && !noise_names.contains(&b_lower.as_str())
                // Exclude known pattern instances
                && !pattern_instance_names.contains(name_a) && !pattern_instance_names.contains(name_b)
            })
            .map(|(name_a, file_a, sig_a, name_b, file_b, sig_b)| {
                serde_json::json!({
                    "category": "true_duplicate",
                    "description": format!("Identical signature in different files: {} and {}", name_a, name_b),
                    "a": {"name": name_a, "file": file_a, "sig": sig_a},
                    "b": {"name": name_b, "file": file_b, "sig": sig_b},
                })
            })
            .collect();
            results.extend(sig_dups);
        }

        // 2. Same non-trivial name in different directories (potential undocumented pattern or copy-paste)
        let name_query = graph.prepare(
            "SELECT name, GROUP_CONCAT(DISTINCT file, '|'), COUNT(DISTINCT file) as file_cnt
             FROM hierarchy_nodes
             WHERE project = ?1 AND kind IN ('function','method')
             AND name IS NOT NULL AND length(name) > 5
             GROUP BY name HAVING file_cnt >= 2"
        );

        if let Ok(mut stmt) = name_query {
            let name_dups: Vec<serde_json::Value> = stmt.query_map(
                rusqlite::params![project],
                |row| {
                    let name: String = row.get(0)?;
                    let files_str: String = row.get(1)?;
                    let count: i64 = row.get(2)?;
                    Ok((name, files_str, count))
                }
            ).ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|(name, _, _)| {
                let lower = name.to_lowercase();
                !noise_names.contains(&lower.as_str())
                && !pattern_instance_names.contains(name)
            })
            .map(|(name, files_str, count)| {
                let files: Vec<&str> = files_str.split('|').collect();
                // Check if files are in different directories — more suspicious
                let dirs: std::collections::HashSet<&str> = files.iter()
                    .filter_map(|f| f.rfind('/').map(|i| &f[..i]))
                    .collect();
                let category = if dirs.len() > 1 { "suspicious" } else { "potential_pattern" };
                serde_json::json!({
                    "category": category,
                    "description": format!("'{}' appears in {} files across {} directories", name, count, dirs.len()),
                    "name": name,
                    "files": files,
                    "file_count": count,
                    "directory_count": dirs.len(),
                })
            })
            .collect();
            results.extend(name_dups);
        }

        Ok(results)
    }

    /// Analyze project conventions — naming patterns, file structures, consistent patterns.
    pub fn get_project_conventions(&self, graph: &rusqlite::Connection, project: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
        let mut conventions = Vec::new();

        // 1. File naming conventions — what suffixes are consistent?
        let mut file_stmt = graph.prepare(
            "SELECT REPLACE(REPLACE(file, RTRIM(file, REPLACE(file, '/', '')), ''), RTRIM(REPLACE(file, RTRIM(file, REPLACE(file, '/', '')), ''), REPLACE(REPLACE(file, RTRIM(file, REPLACE(file, '/', '')), ''), '.', '')), '') as ext, COUNT(*) as cnt
             FROM hierarchy_nodes WHERE project=?1 AND kind='file' AND file IS NOT NULL
             GROUP BY ext HAVING cnt >= 3 ORDER BY cnt DESC LIMIT 10"
        ).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

        let file_exts: Vec<serde_json::Value> = file_stmt.query_map(
            rusqlite::params![project],
            |row| Ok(serde_json::json!({"extension": row.get::<_, String>(0)?, "count": row.get::<_, i64>(1)?}))
        ).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?
        .filter_map(|r| r.ok()).collect();

        if !file_exts.is_empty() {
            conventions.push(serde_json::json!({
                "convention": "file_types",
                "description": "Consistent file types in the project",
                "evidence": file_exts,
            }));
        }

        // 2. Module structure conventions — common directory patterns
        let mut dir_stmt = graph.prepare(
            "SELECT DISTINCT REPLACE(file, '/' || REPLACE(file, RTRIM(file, REPLACE(file, '/', '')), ''), '') as dir
             FROM hierarchy_nodes WHERE project=?1 AND kind='file' AND file IS NOT NULL
             LIMIT 20"
        ).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

        let dirs: Vec<String> = dir_stmt.query_map(
            rusqlite::params![project],
            |row| row.get(0)
        ).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?
        .filter_map(|r| r.ok()).collect();

        // Check for common patterns like src/, tests/, adapters/
        let common_dirs: Vec<&String> = dirs.iter().filter(|d| {
            let d = d.to_lowercase();
            d.contains("adapter") || d.contains("handler") || d.contains("worker")
                || d.contains("test") || d.contains("hook") || d.contains("middleware")
        }).collect();

        if !common_dirs.is_empty() {
            conventions.push(serde_json::json!({
                "convention": "directory_patterns",
                "description": "Directories suggesting structural patterns",
                "evidence": common_dirs,
            }));
        }

        // 3. Naming conventions — consistent prefixes/suffixes across functions
        let mut naming_stmt = graph.prepare(
            "SELECT
               CASE
                 WHEN name LIKE 'get_%' THEN 'get_*'
                 WHEN name LIKE 'set_%' THEN 'set_*'
                 WHEN name LIKE 'is_%' THEN 'is_*'
                 WHEN name LIKE 'has_%' THEN 'has_*'
                 WHEN name LIKE 'create_%' THEN 'create_*'
                 WHEN name LIKE 'update_%' THEN 'update_*'
                 WHEN name LIKE 'delete_%' THEN 'delete_*'
                 WHEN name LIKE 'handle_%' THEN 'handle_*'
                 WHEN name LIKE 'parse_%' THEN 'parse_*'
                 WHEN name LIKE 'test_%' THEN 'test_*'
                 ELSE NULL
               END as prefix,
               COUNT(*) as cnt
             FROM hierarchy_nodes WHERE project=?1 AND kind IN ('function','method')
             GROUP BY prefix HAVING prefix IS NOT NULL AND cnt >= 3
             ORDER BY cnt DESC"
        ).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

        let naming: Vec<serde_json::Value> = naming_stmt.query_map(
            rusqlite::params![project],
            |row| Ok(serde_json::json!({"prefix": row.get::<_, String>(0)?, "count": row.get::<_, i64>(1)?}))
        ).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?
        .filter_map(|r| r.ok()).collect();

        if !naming.is_empty() {
            conventions.push(serde_json::json!({
                "convention": "naming_patterns",
                "description": "Consistent function naming prefixes",
                "evidence": naming,
            }));
        }

        // 4. Detected design patterns (from detect_patterns)
        let patterns = self.list_detected_patterns(project)?;
        if !patterns.is_empty() {
            conventions.push(serde_json::json!({
                "convention": "design_patterns",
                "description": "Detected design patterns by naming convention",
                "evidence": patterns,
            }));
        }

        Ok(conventions)
    }

    pub fn match_pattern(&self, project: &str, description: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
        // Simple keyword matching against pattern types and instance names
        let desc_lower = description.to_lowercase();
        let patterns = self.list_detected_patterns(project)?;

        let mut matches: Vec<serde_json::Value> = patterns.into_iter().filter(|p| {
            let ptype = p["pattern_type"].as_str().unwrap_or("");
            let pname = p["name"].as_str().unwrap_or("");
            // Check if description mentions the pattern type or any instance name
            desc_lower.contains(ptype)
                || p["instances"].as_array().map_or(false, |insts| {
                    insts.iter().any(|i| {
                        let iname = i["name"].as_str().unwrap_or("").to_lowercase();
                        desc_lower.contains(&iname) || iname.contains(&desc_lower)
                    })
                })
                || desc_lower.contains(&pname.to_lowercase())
        }).collect();

        // If no keyword match, return all patterns as context
        if matches.is_empty() && !desc_lower.is_empty() {
            matches = self.list_detected_patterns(project)?;
            for m in &mut matches {
                m.as_object_mut().map(|o| o.insert("match_type".into(), serde_json::json!("context")));
            }
        }

        Ok(matches)
    }

    // ── Events ────────────────────────────────────────────────────────────

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

        // Build params dynamically
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
        let (sql, count) = if let Some(et) = event_type {
            let mut stmt = self.conn.prepare(
                "SELECT COUNT(*) FROM events WHERE project=?1 AND event_type=?2"
            )?;
            let c: u64 = stmt.query_row(params![project, et], |r| r.get(0))?;
            (String::new(), c)
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT COUNT(*) FROM events WHERE project=?1"
            )?;
            let c: u64 = stmt.query_row(params![project], |r| r.get(0))?;
            (String::new(), c)
        };
        let _ = sql;
        Ok(count)
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

    // ── Workflow State ──────────────────────────────────────────────────────

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

    /// Get the file path for a project (needed for state.yaml sync)
    pub fn get_project_path(&self, project: &str) -> rusqlite::Result<Option<String>> {
        self.conn.query_row(
            "SELECT path FROM projects WHERE name=?1 OR repo_id=?1",
            params![project],
            |row| row.get(0),
        ).optional()
    }

    // ── Library Docs & Meta ────────────────────────────────────────────────

    pub fn upsert_lib_doc(
        &self, id: &str, lib_name: &str, title: &str, url: Option<&str>,
        summary: &str, content: Option<&str>, source_type: &str,
        component: Option<&str>, indexed_at: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO lib_docs(id, lib_name, title, url, summary, content, source_type, component, indexed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, lib_name, title, url.unwrap_or(""), summary, content.unwrap_or(""), source_type, component.unwrap_or(""), indexed_at],
        )?;
        Ok(())
    }

    pub fn upsert_lib_meta(
        &self, name: &str, source_type: &str, base_url: Option<&str>,
        _version: Option<&str>, indexed_at: &str,
    ) -> rusqlite::Result<()> {
        // Merge: keep existing used_by, update other fields
        self.conn.execute(
            "INSERT INTO lib_meta(name, source_type, base_url, indexed_at) VALUES(?1,?2,?3,?4)
             ON CONFLICT(name) DO UPDATE SET source_type=?2, base_url=COALESCE(?3, base_url), indexed_at=?4",
            params![name, source_type, base_url.unwrap_or(""), indexed_at],
        )?;
        Ok(())
    }

    pub fn get_lib_doc_component(&self, lib_name: &str, component: &str) -> rusqlite::Result<Option<crate::indexer::lib_indexer::LibDoc>> {
        // Try exact match, then suffix match, then README/index alias
        let mut stmt = self.conn.prepare(
            "SELECT id, title, url, summary, content, source_type, component, indexed_at FROM lib_docs WHERE lib_name = ?1 AND (component = ?2 OR component LIKE '%' || ?2 OR ((?2 = 'index' OR ?2 = 'README') AND component IN ('index', 'README', 'index.txt'))) ORDER BY length(component) ASC LIMIT 1"
        )?;
        let mut rows = stmt.query_map(params![lib_name, component], |row| {
            Ok(crate::indexer::lib_indexer::LibDoc {
                id: row.get(0)?, title: row.get(1)?, url: row.get(2)?, summary: row.get(3)?,
                content: row.get::<_, Option<String>>(4)?, source_type: row.get(5)?,
                component: row.get(6)?, indexed_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(doc)) => Ok(Some(doc)),
            _ => Ok(None),
        }
    }

    pub fn get_lib_docs(&self, lib_name: &str) -> rusqlite::Result<Vec<crate::indexer::lib_indexer::LibDoc>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, url, summary, content, source_type, component, indexed_at FROM lib_docs WHERE lib_name = ?1"
        )?;
        let rows = stmt.query_map(params![lib_name], |row| {
            Ok(crate::indexer::lib_indexer::LibDoc {
                id: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                summary: row.get(3)?,
                content: row.get::<_, Option<String>>(4)?,
                source_type: row.get(5)?,
                component: row.get(6)?,
                indexed_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub fn search_lib_docs(&self, query: &str) -> rusqlite::Result<Vec<crate::indexer::lib_indexer::LibDoc>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, title, url, summary, content, source_type, component, indexed_at FROM lib_docs WHERE title LIKE ?1 OR summary LIKE ?1 OR content LIKE ?1 LIMIT 20"
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(crate::indexer::lib_indexer::LibDoc {
                id: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                summary: row.get(3)?,
                content: row.get::<_, Option<String>>(4)?,
                source_type: row.get(5)?,
                component: row.get(6)?,
                indexed_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    // ── Index Errors ─────────────────────────────────────────────────────────

    #[allow(dead_code)]
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

    // ── Metrics ─────────────────────────────────────────────────────────────

    /// Compute aggregate metrics for a project from sessions + events.
    pub fn compute_metrics(&self, project: &str) -> rusqlite::Result<serde_json::Value> {
        // Sessions → FTR
        let sessions = self.get_sessions(Some(project))?;
        let session_count = sessions.len() as u64;
        let ftr: Option<f64> = if sessions.is_empty() {
            None
        } else {
            let sum: f64 = sessions.iter()
                .filter_map(|s| s["ftr"].as_f64())
                .sum();
            let count = sessions.iter().filter(|s| s["ftr"].as_f64().is_some()).count();
            if count > 0 { Some(sum / count as f64) } else { None }
        };

        // Events → turn count, rework rate, tool adherence
        let turn_count = self.count_events(project, Some("turn"))?;
        let revision_count = self.count_events(project, Some("revision_requested"))?;
        let rework_rate = if turn_count > 0 {
            revision_count as f64 / turn_count as f64
        } else {
            0.0
        };

        // Tool adherence: MCP tools / total tools
        let tool_events = self.list_events(project, Some("tool_used"), None, 500)?;
        let total_tools = tool_events.len() as u64;
        let mcp_tools = tool_events.iter()
            .filter(|e| e["data"]["is_mcp"].as_bool() == Some(true))
            .count() as u64;
        let tool_adherence: Option<f64> = if total_tools > 0 {
            Some(mcp_tools as f64 / total_tools as f64)
        } else {
            None
        };

        Ok(serde_json::json!({
            "session_count": session_count,
            "ftr": ftr,
            "turn_count": turn_count,
            "rework_rate": rework_rate,
            "revision_count": revision_count,
            "tool_adherence": tool_adherence,
            "mcp_tool_count": mcp_tools,
            "total_tool_count": total_tools,
        }))
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
                SolutionRepo { path: None, repo_id: "api".into(), role: "backend".into(), label: Some("API".into()) },
                SolutionRepo { path: None, repo_id: "ui".into(), role: "frontend".into(), label: None },
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
            repo_id: "api".into(), role: "backend".into(), label: None, path: None,
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
            repos: vec![SolutionRepo { path: None, repo_id: "api".into(), role: "backend".into(), label: None }],
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
    fn exclude_and_list() {
        let s = test_store();
        s.upsert_project(&make_project("foo", "/tmp/foo")).unwrap();
        s.upsert_project(&make_project("bar", "/tmp/bar")).unwrap();

        // Exclude foo
        s.exclude_project("foo", "/tmp/foo").unwrap();

        // foo is gone from projects
        assert_eq!(s.list_projects().unwrap().len(), 1);
        assert!(s.get_project("foo").unwrap().is_none());

        // foo is in exclusions
        let excl = s.list_exclusions().unwrap();
        assert_eq!(excl.len(), 1);
        assert_eq!(excl[0].0, "/tmp/foo");

        // is_excluded checks
        assert!(s.is_excluded("/tmp/foo"));
        assert!(s.is_excluded("/tmp/foo/src/main.rs")); // child path also excluded
        assert!(!s.is_excluded("/tmp/bar"));

        // Remove exclusion
        s.remove_exclusion("/tmp/foo").unwrap();
        assert!(!s.is_excluded("/tmp/foo"));
        assert_eq!(s.list_exclusions().unwrap().len(), 0);
    }

    #[test]
    fn mark_index_failed() {
        let s = test_store();
        s.upsert_project(&make_project("foo", "/tmp/foo")).unwrap();
        s.mark_index_failed("foo", "Kuzu connection error").unwrap();
        let p = s.get_project("foo").unwrap().unwrap();
        assert_eq!(p.last_error.as_deref(), Some("Kuzu connection error"));
    }

    // ── Metrics tests ───────────────────────────────────────────────────

    #[test]
    fn metrics_empty_project() {
        let s = test_store();
        let m = s.compute_metrics("nonexistent").unwrap();
        assert_eq!(m["session_count"], 0);
        assert_eq!(m["turn_count"], 0);
        assert_eq!(m["rework_rate"], 0.0);
        assert!(m["ftr"].is_null()); // no sessions → no FTR
    }

    #[test]
    fn metrics_ftr_from_sessions() {
        let s = test_store();
        // 3 sessions: 2 completed (FTR=1.0), 1 partial (FTR=0.5)
        s.create_session("s1", "proj", "task 1").unwrap();
        s.update_session("s1", Some("completed"), Some("done"), None, None, None).unwrap();
        s.create_session("s2", "proj", "task 2").unwrap();
        s.update_session("s2", Some("completed"), Some("done"), None, None, None).unwrap();
        s.create_session("s3", "proj", "task 3").unwrap();
        s.update_session("s3", Some("partial"), Some("wip"), None, None, None).unwrap();

        let m = s.compute_metrics("proj").unwrap();
        assert_eq!(m["session_count"], 3);
        // FTR = (1.0 + 1.0 + 0.5) / 3 = 0.833...
        let ftr = m["ftr"].as_f64().unwrap();
        assert!((ftr - 0.833).abs() < 0.01, "FTR should be ~0.83, got {}", ftr);
    }

    #[test]
    fn metrics_turn_count_and_rework() {
        let s = test_store();
        // 5 turns, 2 of which are corrections (revision_requested)
        s.insert_event("e1", "proj", None, "turn", "{}").unwrap();
        s.insert_event("e2", "proj", None, "turn", "{}").unwrap();
        s.insert_event("e3", "proj", None, "revision_requested", "{}").unwrap();
        s.insert_event("e4", "proj", None, "turn", "{}").unwrap();
        s.insert_event("e5", "proj", None, "revision_requested", "{}").unwrap();
        s.insert_event("e6", "proj", None, "turn", "{}").unwrap();
        s.insert_event("e7", "proj", None, "turn", "{}").unwrap();

        let m = s.compute_metrics("proj").unwrap();
        assert_eq!(m["turn_count"], 5);
        // rework_rate = revision_requested / turns = 2/5 = 0.4
        let rr = m["rework_rate"].as_f64().unwrap();
        assert!((rr - 0.4).abs() < 0.01, "rework_rate should be 0.4, got {}", rr);
    }

    #[test]
    fn metrics_tool_adherence() {
        let s = test_store();
        // 4 tool_used events: 3 MCP, 1 non-MCP
        s.insert_event("t1", "proj", None, "tool_used", r#"{"tool":"search","is_mcp":true}"#).unwrap();
        s.insert_event("t2", "proj", None, "tool_used", r#"{"tool":"grep","is_mcp":false}"#).unwrap();
        s.insert_event("t3", "proj", None, "tool_used", r#"{"tool":"get_callers","is_mcp":true}"#).unwrap();
        s.insert_event("t4", "proj", None, "tool_used", r#"{"tool":"get_patterns","is_mcp":true}"#).unwrap();

        let m = s.compute_metrics("proj").unwrap();
        // tool_adherence = mcp_tools / total_tools = 3/4 = 0.75
        let ta = m["tool_adherence"].as_f64().unwrap();
        assert!((ta - 0.75).abs() < 0.01, "tool_adherence should be 0.75, got {}", ta);
    }
}
