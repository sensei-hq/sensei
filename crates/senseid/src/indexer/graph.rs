use rusqlite::{Connection, params, OptionalExtension};
use std::path::Path;
use crate::types::SymbolKind;

/// Graph database backed by SQLite (interim — will migrate to Kuzu).
/// Uses adjacency list tables for nodes and edges.
pub struct GraphDb {
    conn: Connection,
    /// Path to the graph.db file (for opening additional connections).
    db_file: Option<std::path::PathBuf>,
}

impl GraphDb {
    pub fn open(path: &Path) -> Result<Self, String> {
        let db_path = path.join("graph.db");
        let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open graph DB: {}", e))?;
        // Enable WAL mode for concurrent reader/writer access
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;").ok();
        let db = Self { conn, db_file: Some(db_path) };
        db.ensure_schema()?;
        Ok(db)
    }

    /// Open a new connection to the same database file (for parallel workers).
    pub fn clone_connection(&self) -> Result<Self, String> {
        let db_path = self.db_file.as_ref().ok_or("Cannot clone in-memory DB")?;
        let conn = Connection::open(db_path).map_err(|e| format!("Failed to clone graph DB: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;").ok();
        let db = Self { conn, db_file: self.db_file.clone() };
        db.ensure_schema()?;
        Ok(db)
    }

    /// Get the path to the graph.db file.
    pub fn clear_all(&self) -> Result<(), String> {
        self.conn.execute_batch("
            DELETE FROM functions;
            DELETE FROM files;
            DELETE FROM types;
            DELETE FROM comments;
            DELETE FROM docs;
            DELETE FROM packages;
            DELETE FROM modules;
            DELETE FROM edges;
        ").map_err(|e| e.to_string())?;
        // Also clear community/drift tables if they exist
        self.conn.execute_batch("
            DELETE FROM doc_drift;
        ").ok();
        Ok(())
    }

    pub fn db_path(&self) -> Option<&std::path::Path> {
        self.db_file.as_deref()
    }

    pub fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let db = Self { conn, db_file: None };
        db.ensure_schema()?;
        Ok(db)
    }

    fn ensure_schema(&self) -> Result<(), String> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS functions(
                id TEXT PRIMARY KEY, name TEXT, file TEXT, line INTEGER,
                sig TEXT, body TEXT, docstring TEXT, complexity INTEGER DEFAULT 1,
                tags TEXT DEFAULT '', project TEXT
            );
            CREATE TABLE IF NOT EXISTS files(
                id TEXT PRIMARY KEY, path TEXT, module TEXT, lang TEXT,
                tags TEXT DEFAULT '', project TEXT
            );
            CREATE TABLE IF NOT EXISTS types(
                id TEXT PRIMARY KEY, name TEXT, file TEXT, line INTEGER, kind TEXT, project TEXT
            );
            CREATE TABLE IF NOT EXISTS comments(
                id TEXT PRIMARY KEY, text TEXT, tag TEXT, line INTEGER, file TEXT, project TEXT
            );
            CREATE TABLE IF NOT EXISTS docs(
                id TEXT PRIMARY KEY, path TEXT, title TEXT, doc_type TEXT, project TEXT
            );
            CREATE TABLE IF NOT EXISTS packages(
                id TEXT PRIMARY KEY, name TEXT NOT NULL, version TEXT,
                path TEXT, pkg_type TEXT, project TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS modules(
                id TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT NOT NULL,
                package_id TEXT, project TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS edges(
                from_id TEXT NOT NULL, to_id TEXT NOT NULL, edge_type TEXT NOT NULL,
                weight REAL,
                PRIMARY KEY(from_id, to_id, edge_type)
            );
            CREATE INDEX IF NOT EXISTS idx_fn_project ON functions(project);
            CREATE INDEX IF NOT EXISTS idx_fn_file ON functions(file);
            CREATE INDEX IF NOT EXISTS idx_fn_name ON functions(name);
            CREATE INDEX IF NOT EXISTS idx_type_project ON types(project);
            CREATE INDEX IF NOT EXISTS idx_type_name ON types(name);
            CREATE INDEX IF NOT EXISTS idx_edge_from ON edges(from_id);
            CREATE INDEX IF NOT EXISTS idx_edge_to ON edges(to_id);
            CREATE INDEX IF NOT EXISTS idx_pkg_project ON packages(project);
            CREATE INDEX IF NOT EXISTS idx_mod_project ON modules(project);
            CREATE INDEX IF NOT EXISTS idx_mod_package ON modules(package_id);
        ").map_err(|e| e.to_string())?;

        // Migration: add tags column if missing (for existing DBs)
        self.conn.execute_batch("
            ALTER TABLE files ADD COLUMN tags TEXT DEFAULT '';
            ALTER TABLE functions ADD COLUMN tags TEXT DEFAULT '';
        ").ok(); // Ignore errors (column already exists)

        Ok(())
    }

    pub fn merge_function(
        &self, id: &str, name: &str, file: &str, line: u32,
        sig: &str, body: &str, docstring: &str, complexity: u32, project: &str,
    ) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO functions(id, name, file, line, sig, body, docstring, complexity, project) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, name, file, line, &sig[..sig.len().min(500)], &body[..body.len().min(10000)], &docstring[..docstring.len().min(2000)], complexity, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn merge_file(
        &self, id: &str, path: &str, module: &str, lang: &str, project: &str,
    ) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO files(id, path, module, lang, project) VALUES(?1,?2,?3,?4,?5)",
            params![id, path, module, lang, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn merge_type(
        &self, id: &str, name: &str, file: &str, line: u32, kind: &str, project: &str,
    ) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO types(id, name, file, line, kind, project) VALUES(?1,?2,?3,?4,?5,?6)",
            params![id, name, file, line, kind, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn merge_edge(&self, from_id: &str, to_id: &str, edge_type: &str) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO edges(from_id, to_id, edge_type) VALUES(?1,?2,?3)",
            params![from_id, to_id, edge_type],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete all packages and modules for a project (for clean re-creation).
    pub fn clear_hierarchy(&self, project: &str) -> Result<(), String> {
        // Delete edges from/to packages and modules
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM packages WHERE project = ?1) OR to_id IN (SELECT id FROM packages WHERE project = ?1)",
            params![project],
        ).ok();
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM modules WHERE project = ?1) OR to_id IN (SELECT id FROM modules WHERE project = ?1)",
            params![project],
        ).ok();
        // Also delete HAS_METHOD edges (type→function) for this project
        self.conn.execute(
            "DELETE FROM edges WHERE edge_type = 'HAS_METHOD' AND from_id IN (SELECT id FROM types WHERE project = ?1)",
            params![project],
        ).ok();
        self.conn.execute("DELETE FROM packages WHERE project = ?1", params![project]).ok();
        self.conn.execute("DELETE FROM modules WHERE project = ?1", params![project]).ok();
        Ok(())
    }

    pub fn merge_package(
        &self, id: &str, name: &str, version: Option<&str>,
        path: &str, pkg_type: &str, project: &str,
    ) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO packages(id, name, version, path, pkg_type, project) VALUES(?1,?2,?3,?4,?5,?6)",
            params![id, name, version, path, pkg_type, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn merge_module(
        &self, id: &str, name: &str, path: &str,
        package_id: Option<&str>, project: &str,
    ) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO modules(id, name, path, package_id, project) VALUES(?1,?2,?3,?4,?5)",
            params![id, name, path, package_id, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn count_packages(&self, project: &str) -> Result<u32, String> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM packages WHERE project = ?1", params![project],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn count_modules(&self, project: &str) -> Result<u32, String> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM modules WHERE project = ?1", params![project],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn delete_file(&self, abs_path: &str, project: &str) -> Result<(), String> {
        // Delete edges involving functions from this file
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM functions WHERE file = ?1 AND project = ?2) OR to_id IN (SELECT id FROM functions WHERE file = ?1 AND project = ?2)",
            params![abs_path, project],
        ).ok();
        self.conn.execute("DELETE FROM functions WHERE file = ?1 AND project = ?2", params![abs_path, project]).ok();
        self.conn.execute("DELETE FROM types WHERE file = ?1 AND project = ?2", params![abs_path, project]).ok();
        self.conn.execute("DELETE FROM comments WHERE file = ?1 AND project = ?2", params![abs_path, project]).ok();
        self.conn.execute("DELETE FROM files WHERE path = ?1 AND project = ?2", params![abs_path, project]).ok();
        Ok(())
    }

    pub fn delete_doc(&self, doc_id: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1",
            params![doc_id],
        ).ok();
        self.conn.execute("DELETE FROM docs WHERE id = ?1", params![doc_id]).ok();
        Ok(())
    }

    pub fn merge_doc(
        &self, id: &str, path: &str, title: &str, doc_type: &str, project: &str,
    ) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO docs(id, path, title, doc_type, project) VALUES(?1,?2,?3,?4,?5)",
            params![id, path, title, doc_type, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn find_function_by_name(&self, name: &str, project: &str) -> Result<Option<String>, String> {
        self.conn.query_row(
            "SELECT id FROM functions WHERE name = ?1 AND project = ?2 LIMIT 1",
            params![name, project],
            |row| row.get(0),
        ).optional().map_err(|e| e.to_string())
    }

    pub fn count_symbols(&self, project: &str) -> Result<(u32, u32), String> {
        let fn_count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM functions WHERE project = ?1", params![project],
            |row| row.get(0),
        ).unwrap_or(0);
        let type_count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM types WHERE project = ?1", params![project],
            |row| row.get(0),
        ).unwrap_or(0);
        Ok((fn_count, type_count))
    }

    pub fn count_edges(&self, project: &str) -> Result<u32, String> {
        let count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM edges e WHERE e.from_id IN (
                SELECT id FROM functions WHERE project = ?1
                UNION SELECT id FROM files WHERE project = ?1
                UNION SELECT id FROM types WHERE project = ?1
                UNION SELECT id FROM docs WHERE project = ?1
                UNION SELECT id FROM packages WHERE project = ?1
                UNION SELECT id FROM modules WHERE project = ?1
            )",
            params![project], |row| row.get(0),
        ).unwrap_or(0);
        Ok(count)
    }

    /// Get all nodes for a project (for D3 visualization).
    pub fn get_nodes(&self, project: &str) -> Result<Vec<crate::types::GraphNode>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, 'function' as kind, file, line, complexity FROM functions WHERE project = ?1
             UNION ALL
             SELECT id, name, kind, file, line, 0 FROM types WHERE project = ?1
             UNION ALL
             SELECT id, module as name, 'file' as kind, path as file, 0, 0 FROM files WHERE project = ?1
             UNION ALL
             SELECT id, title as name, 'doc' as kind, path as file, 0, 0 FROM docs WHERE project = ?1
             UNION ALL
             SELECT id, name, 'package' as kind, path as file, 0, 0 FROM packages WHERE project = ?1
             UNION ALL
             SELECT id, name, 'module' as kind, path as file, 0, 0 FROM modules WHERE project = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(crate::types::GraphNode {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                file: row.get(3)?,
                line: row.get(4)?,
                complexity: row.get(5).ok(),
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get all edges for a project.
    pub fn get_edges(&self, project: &str) -> Result<Vec<crate::types::GraphEdge>, String> {
        // Join edges with any node type (functions, files, types, docs) that belongs to this project
        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, e.to_id, e.edge_type FROM edges e
             WHERE e.from_id IN (
                 SELECT id FROM functions WHERE project = ?1
                 UNION SELECT id FROM files WHERE project = ?1
                 UNION SELECT id FROM types WHERE project = ?1
                 UNION SELECT id FROM docs WHERE project = ?1
                 UNION SELECT id FROM packages WHERE project = ?1
                 UNION SELECT id FROM modules WHERE project = ?1
             )"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(crate::types::GraphEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                edge_type: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get call-flow: exported functions as roots, their callees as children.
    /// Returns a layered structure for hierarchical graph visualization.
    pub fn get_call_flow(&self, project: &str) -> Result<serde_json::Value, String> {
        // Layer 0: Files (packages/modules)
        let files: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, module FROM files WHERE project = ?1"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![project], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }).map_err(|e| e.to_string())?;
            rows.filter_map(|r| r.ok()).collect()
        };

        // Layer 1: Exported functions (EXPORTS_FN edges from files)
        let mut exports: Vec<serde_json::Value> = Vec::new();
        for (file_id, module) in &files {
            let mut stmt = self.conn.prepare(
                "SELECT f.id, f.name, f.file, f.line, f.complexity FROM functions f
                 JOIN edges e ON e.to_id = f.id
                 WHERE e.from_id = ?1 AND e.edge_type = 'EXPORTS_FN'"
            ).map_err(|e| e.to_string())?;
            let fns: Vec<serde_json::Value> = stmt.query_map(params![file_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "file": row.get::<_, String>(2)?,
                    "line": row.get::<_, u32>(3)?,
                    "complexity": row.get::<_, u32>(4)?,
                }))
            }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

            if !fns.is_empty() {
                exports.push(serde_json::json!({
                    "module": module,
                    "fileId": file_id,
                    "functions": fns,
                }));
            }
        }

        // Layer 2: Call edges between functions
        let calls: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT e.from_id, e.to_id FROM edges e
                 WHERE e.edge_type = 'CALLS'
                 AND e.from_id IN (SELECT id FROM functions WHERE project = ?1)"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![project], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }).map_err(|e| e.to_string())?;
            rows.filter_map(|r| r.ok()).collect()
        };

        Ok(serde_json::json!({
            "modules": exports,
            "calls": calls.iter().map(|(from, to)| serde_json::json!({"from": from, "to": to})).collect::<Vec<_>>(),
            "moduleCount": files.len(),
            "exportCount": exports.iter().map(|m| m["functions"].as_array().map_or(0, |a| a.len())).sum::<usize>(),
            "callCount": calls.len(),
        }))
    }

    /// Tag a file with framework/pattern tags (comma-separated).
    pub fn tag_file(&self, file_id: &str, tags: &str) -> Result<(), String> {
        self.conn.execute(
            "UPDATE files SET tags = ?2 WHERE id = ?1",
            params![file_id, tags],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Tag a function with pattern tags.
    pub fn tag_function(&self, fn_id: &str, tags: &str) -> Result<(), String> {
        self.conn.execute(
            "UPDATE functions SET tags = ?2 WHERE id = ?1",
            params![fn_id, tags],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Search functions by name pattern (LIKE).
    pub fn search_functions(&self, query: &str, project: &str) -> Result<Vec<crate::types::FunctionDetail>, String> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file, line, sig, docstring, complexity, tags FROM functions WHERE project = ?1 AND name LIKE ?2 LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project, pattern], |row| {
            Ok(crate::types::FunctionDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                file: row.get(2)?,
                line: row.get(3)?,
                signature: row.get::<_, Option<String>>(4)?,
                docstring: row.get::<_, Option<String>>(5)?,
                complexity: row.get(6)?,
                tags: row.get::<_, String>(7).unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Search types by name pattern.
    pub fn search_types(&self, query: &str, project: &str) -> Result<Vec<crate::types::TypeDetail>, String> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file, line, kind FROM types WHERE project = ?1 AND name LIKE ?2 LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project, pattern], |row| {
            Ok(crate::types::TypeDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                file: row.get(2)?,
                line: row.get(3)?,
                kind: row.get(4)?,
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get files by tag pattern.
    pub fn files_by_tag(&self, tag: &str, project: &str) -> Result<Vec<(String, String, String)>, String> {
        let pattern = format!("%{}%", tag);
        let mut stmt = self.conn.prepare(
            "SELECT id, path, tags FROM files WHERE project = ?1 AND tags LIKE ?2 LIMIT 100"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project, pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get callers of a function.
    pub fn callers_of(&self, fn_name: &str, project: &str) -> Result<Vec<crate::types::FunctionDetail>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.name, f.file, f.line, f.sig, f.docstring, f.complexity, f.tags
             FROM functions f
             JOIN edges e ON e.from_id = f.id
             WHERE e.edge_type = 'CALLS'
               AND e.to_id IN (SELECT id FROM functions WHERE name = ?1 AND project = ?2)
               AND f.project = ?2
             LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![fn_name, project], |row| {
            Ok(crate::types::FunctionDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                file: row.get(2)?,
                line: row.get(3)?,
                signature: row.get::<_, Option<String>>(4)?,
                docstring: row.get::<_, Option<String>>(5)?,
                complexity: row.get(6)?,
                tags: row.get::<_, String>(7).unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get callees of a function.
    pub fn callees_of(&self, fn_name: &str, project: &str) -> Result<Vec<crate::types::FunctionDetail>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.name, f.file, f.line, f.sig, f.docstring, f.complexity, f.tags
             FROM functions f
             JOIN edges e ON e.to_id = f.id
             WHERE e.edge_type = 'CALLS'
               AND e.from_id IN (SELECT id FROM functions WHERE name = ?1 AND project = ?2)
               AND f.project = ?2
             LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![fn_name, project], |row| {
            Ok(crate::types::FunctionDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                file: row.get(2)?,
                line: row.get(3)?,
                signature: row.get::<_, Option<String>>(4)?,
                docstring: row.get::<_, Option<String>>(5)?,
                complexity: row.get(6)?,
                tags: row.get::<_, String>(7).unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Store community assignments for nodes.
    pub fn store_communities(&self, communities: &std::collections::HashMap<String, u32>) -> Result<(), String> {
        self.conn.execute_batch(
            "ALTER TABLE functions ADD COLUMN community INTEGER DEFAULT -1;
             ALTER TABLE files ADD COLUMN community INTEGER DEFAULT -1;
             ALTER TABLE types ADD COLUMN community INTEGER DEFAULT -1;"
        ).ok();

        for (node_id, comm) in communities {
            self.conn.execute("UPDATE functions SET community = ?2 WHERE id = ?1", params![node_id, comm]).ok();
            self.conn.execute("UPDATE files SET community = ?2 WHERE id = ?1", params![node_id, comm]).ok();
            self.conn.execute("UPDATE types SET community = ?2 WHERE id = ?1", params![node_id, comm]).ok();
        }
        Ok(())
    }

    /// Get community summary for a project.
    pub fn get_communities(&self, project: &str) -> Result<Vec<super::community::CommunityInfo>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT community, COUNT(*) as cnt, GROUP_CONCAT(name, ', ') as members
             FROM (
                 SELECT community, name FROM functions WHERE project = ?1 AND community >= 0
                 UNION ALL
                 SELECT community, module as name FROM files WHERE project = ?1 AND community >= 0
             )
             GROUP BY community
             ORDER BY cnt DESC"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![project], |row| {
            Ok(super::community::CommunityInfo {
                id: row.get(0)?,
                size: row.get(1)?,
                sample_members: row.get::<_, String>(2)
                    .unwrap_or_default()
                    .split(", ")
                    .take(5)
                    .map(|s| s.to_string())
                    .collect(),
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Find docs that may have drifted because their linked code changed.
    /// Returns (doc_id, doc_path, edge_type, changed_target_id).
    pub fn find_drifted_docs(
        &self, changed_file_ids: &[String], changed_fn_ids: &[String],
    ) -> Result<Vec<DocDrift>, String> {
        let mut drifts = Vec::new();

        // Docs that COVERS a changed file
        for file_id in changed_file_ids {
            let mut stmt = self.conn.prepare(
                "SELECT d.id, d.path, e.edge_type FROM docs d
                 JOIN edges e ON e.from_id = d.id
                 WHERE e.to_id = ?1 AND e.edge_type = 'COVERS'"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![file_id], |row| {
                Ok(DocDrift {
                    doc_id: row.get(0)?,
                    doc_path: row.get(1)?,
                    edge_type: row.get(2)?,
                    changed_target: file_id.clone(),
                })
            }).map_err(|e| e.to_string())?;
            for r in rows { if let Ok(d) = r { drifts.push(d); } }
        }

        // Docs that MENTIONS_FN a changed function
        for fn_id in changed_fn_ids {
            let mut stmt = self.conn.prepare(
                "SELECT d.id, d.path, e.edge_type FROM docs d
                 JOIN edges e ON e.from_id = d.id
                 WHERE e.to_id = ?1 AND e.edge_type = 'MENTIONS_FN'"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![fn_id], |row| {
                Ok(DocDrift {
                    doc_id: row.get(0)?,
                    doc_path: row.get(1)?,
                    edge_type: row.get(2)?,
                    changed_target: fn_id.clone(),
                })
            }).map_err(|e| e.to_string())?;
            for r in rows { if let Ok(d) = r { drifts.push(d); } }
        }

        Ok(drifts)
    }

    /// Store doc drift entries.
    pub fn record_doc_drift(&self, drifts: &[DocDrift], project: &str) -> Result<(), String> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS doc_drift(
                doc_id TEXT NOT NULL, doc_path TEXT, edge_type TEXT,
                changed_target TEXT, detected_at TEXT, project TEXT
            )"
        ).map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().to_rfc3339();
        for drift in drifts {
            self.conn.execute(
                "INSERT OR REPLACE INTO doc_drift(doc_id, doc_path, edge_type, changed_target, detected_at, project) VALUES(?1,?2,?3,?4,?5,?6)",
                params![drift.doc_id, drift.doc_path, drift.edge_type, drift.changed_target, now, project],
            ).ok();
        }
        Ok(())
    }

    /// Get all doc drift entries for a project.
    pub fn get_doc_drift(&self, project: &str) -> Result<Vec<DocDrift>, String> {
        // Table may not exist yet
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS doc_drift(
                doc_id TEXT NOT NULL, doc_path TEXT, edge_type TEXT,
                changed_target TEXT, detected_at TEXT, project TEXT
            )"
        ).ok();

        let mut stmt = self.conn.prepare(
            "SELECT doc_id, doc_path, edge_type, changed_target FROM doc_drift WHERE project = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(DocDrift {
                doc_id: row.get(0)?,
                doc_path: row.get(1)?,
                edge_type: row.get(2)?,
                changed_target: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocDrift {
    pub doc_id: String,
    pub doc_path: String,
    pub edge_type: String,
    pub changed_target: String,
}

pub fn is_function_like(kind: &SymbolKind) -> bool {
    matches!(kind, SymbolKind::Function | SymbolKind::Method | SymbolKind::Component | SymbolKind::Hook)
}

pub fn compute_complexity(body: &str) -> u32 {
    let patterns = ["if ", "else if ", "elif ", "else ", "for ", "while ", "catch ",
        "case ", "&&", "||", "? ", "try ", "match ", "except "];
    let mut n: u32 = 1;
    for pat in &patterns { n += body.matches(pat).count() as u32; }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_db_opens() {
        let db = GraphDb::open_memory().unwrap();
        let (fns, types) = db.count_symbols("test").unwrap();
        assert_eq!(fns, 0);
        assert_eq!(types, 0);
    }

    #[test]
    fn merge_and_count_function() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:test.py:hello:1", "hello", "test.py", 1, "def hello()", "return 42", "Say hello", 1, "proj").unwrap();
        let (fns, _) = db.count_symbols("proj").unwrap();
        assert_eq!(fns, 1);
    }

    #[test]
    fn merge_file_and_edge() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_file("file:a.py", "a.py", "a", "python", "proj").unwrap();
        db.merge_function("fn:a.py:foo:1", "foo", "a.py", 1, "def foo()", "", "", 1, "proj").unwrap();
        db.merge_edge("file:a.py", "fn:a.py:foo:1", "EXPORTS_FN").unwrap();
    }

    #[test]
    fn delete_file_removes_symbols() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:a.py:foo:1", "foo", "/tmp/a.py", 1, "", "", "", 1, "proj").unwrap();
        db.merge_function("fn:a.py:bar:5", "bar", "/tmp/a.py", 5, "", "", "", 1, "proj").unwrap();
        db.merge_function("fn:b.py:baz:1", "baz", "/tmp/b.py", 1, "", "", "", 1, "proj").unwrap();
        assert_eq!(db.count_symbols("proj").unwrap().0, 3);
        db.delete_file("/tmp/a.py", "proj").unwrap();
        assert_eq!(db.count_symbols("proj").unwrap().0, 1);
    }

    #[test]
    fn get_nodes_and_edges() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:a:1", "foo", "a.py", 1, "", "", "", 1, "proj").unwrap();
        db.merge_function("fn:a:5", "bar", "a.py", 5, "", "", "", 1, "proj").unwrap();
        db.merge_edge("fn:a:1", "fn:a:5", "CALLS").unwrap();
        let nodes = db.get_nodes("proj").unwrap();
        assert_eq!(nodes.len(), 2);
        let edges = db.get_edges("proj").unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, "CALLS");
    }

    #[test]
    fn complexity() {
        assert_eq!(compute_complexity("x = 1"), 1);
        assert_eq!(compute_complexity("if x:\n  pass\nelse:\n  pass"), 2); // "else:" != "else "
        assert_eq!(compute_complexity("if x { } else { }"), 3); // "if " + "else " match
    }

    #[test]
    fn is_fn_like() {
        assert!(is_function_like(&SymbolKind::Function));
        assert!(is_function_like(&SymbolKind::Method));
        assert!(!is_function_like(&SymbolKind::Class));
    }

    #[test]
    fn merge_and_count_packages() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_package("pkg:proj:ui", "ui", Some("1.0.0"), "packages/ui", "npm_workspace", "proj").unwrap();
        db.merge_package("pkg:proj:api", "api", None, "packages/api", "npm_workspace", "proj").unwrap();
        assert_eq!(db.count_packages("proj").unwrap(), 2);
        assert_eq!(db.count_packages("other").unwrap(), 0);
    }

    #[test]
    fn merge_and_count_modules() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_module("mod:proj:src/utils", "src/utils", "src/utils", None, "proj").unwrap();
        db.merge_module("mod:proj:src/api", "src/api", "src/api", Some("pkg:proj:api"), "proj").unwrap();
        assert_eq!(db.count_modules("proj").unwrap(), 2);
    }

    #[test]
    fn get_nodes_includes_packages_and_modules() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:a:1", "foo", "a.py", 1, "", "", "", 1, "proj").unwrap();
        db.merge_package("pkg:proj:core", "core", None, "core", "cargo_crate", "proj").unwrap();
        db.merge_module("mod:proj:core/src", "core/src", "core/src", Some("pkg:proj:core"), "proj").unwrap();
        let nodes = db.get_nodes("proj").unwrap();
        let kinds: Vec<&str> = nodes.iter().map(|n| n.kind.as_str()).collect();
        assert!(kinds.contains(&"function"), "should contain function nodes");
        assert!(kinds.contains(&"package"), "should contain package nodes");
        assert!(kinds.contains(&"module"), "should contain module nodes");
    }

    #[test]
    fn containment_edges_queryable() {
        let db = GraphDb::open_memory().unwrap();
        let pkg_id = "pkg:proj:ui";
        let mod_id = "mod:proj:ui/src";
        let fn_id = "fn:a:foo:1";
        let type_id = "type:a:Foo:1";
        db.merge_package(pkg_id, "ui", None, "ui", "npm_workspace", "proj").unwrap();
        db.merge_module(mod_id, "ui/src", "ui/src", Some(pkg_id), "proj").unwrap();
        db.merge_function(fn_id, "foo", "a.ts", 1, "", "", "", 1, "proj").unwrap();
        db.merge_type(type_id, "Foo", "a.ts", 1, "class", "proj").unwrap();
        db.merge_edge("project:proj", pkg_id, "CONTAINS_PKG").unwrap();
        db.merge_edge(pkg_id, mod_id, "CONTAINS_MOD").unwrap();
        db.merge_edge(mod_id, fn_id, "CONTAINS_FN").unwrap();
        db.merge_edge(type_id, fn_id, "HAS_METHOD").unwrap();

        let edges = db.get_edges("proj").unwrap();
        let edge_types: Vec<&str> = edges.iter().map(|e| e.edge_type.as_str()).collect();
        assert!(edge_types.contains(&"CONTAINS_MOD"), "should have CONTAINS_MOD");
        assert!(edge_types.contains(&"CONTAINS_FN"), "should have CONTAINS_FN");
        assert!(edge_types.contains(&"HAS_METHOD"), "should have HAS_METHOD");
    }

    #[test]
    fn clear_all_clears_packages_and_modules() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_package("pkg:proj:a", "a", None, "a", "npm_workspace", "proj").unwrap();
        db.merge_module("mod:proj:b", "b", "b", None, "proj").unwrap();
        assert_eq!(db.count_packages("proj").unwrap(), 1);
        assert_eq!(db.count_modules("proj").unwrap(), 1);
        db.clear_all().unwrap();
        assert_eq!(db.count_packages("proj").unwrap(), 0);
        assert_eq!(db.count_modules("proj").unwrap(), 0);
    }

    #[test]
    fn search_functions_by_name() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:a:hello:1", "hello", "a.py", 1, "def hello()", "", "", 1, "proj").unwrap();
        db.merge_function("fn:a:help:5", "help", "a.py", 5, "def help()", "", "", 1, "proj").unwrap();
        db.merge_function("fn:a:world:10", "world", "a.py", 10, "def world()", "", "", 1, "proj").unwrap();
        let results = db.search_functions("hel", "proj").unwrap();
        assert_eq!(results.len(), 2); // hello + help
        let names: Vec<&str> = results.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"help"));
    }

    #[test]
    fn search_types_by_name() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_type("type:a:UserService:1", "UserService", "a.ts", 1, "class", "proj").unwrap();
        db.merge_type("type:a:UserRole:5", "UserRole", "a.ts", 5, "enum", "proj").unwrap();
        db.merge_type("type:a:Config:10", "Config", "a.ts", 10, "interface", "proj").unwrap();
        let results = db.search_types("User", "proj").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn callers_and_callees() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:a:main:1", "main", "a.py", 1, "", "", "", 1, "proj").unwrap();
        db.merge_function("fn:a:helper:5", "helper", "a.py", 5, "", "", "", 1, "proj").unwrap();
        db.merge_function("fn:a:util:10", "util", "a.py", 10, "", "", "", 1, "proj").unwrap();
        db.merge_edge("fn:a:main:1", "fn:a:helper:5", "CALLS").unwrap();
        db.merge_edge("fn:a:helper:5", "fn:a:util:10", "CALLS").unwrap();
        // callers of helper = [main]
        let callers = db.callers_of("helper", "proj").unwrap();
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].name, "main");
        // callees of helper = [util]
        let callees = db.callees_of("helper", "proj").unwrap();
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "util");
    }

    #[test]
    fn tag_file_and_function() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_file("file:a.ts", "a.ts", "a", "typescript", "proj").unwrap();
        db.merge_function("fn:a:foo:1", "foo", "a.ts", 1, "", "", "", 1, "proj").unwrap();
        db.tag_file("file:a.ts", "react,hooks").unwrap();
        db.tag_function("fn:a:foo:1", "hook").unwrap();
        let files = db.files_by_tag("react", "proj").unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].2.contains("react"));
    }

    #[test]
    fn find_function_by_name() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:a:hello:1", "hello", "a.py", 1, "", "", "", 1, "proj").unwrap();
        let found = db.find_function_by_name("hello", "proj").unwrap();
        assert_eq!(found, Some("fn:a:hello:1".to_string()));
        let not_found = db.find_function_by_name("missing", "proj").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn count_edges_includes_all_node_types() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:a:1", "foo", "a.py", 1, "", "", "", 1, "proj").unwrap();
        db.merge_type("type:a:Foo:1", "Foo", "a.py", 1, "class", "proj").unwrap();
        db.merge_module("mod:proj:src", "src", "src", None, "proj").unwrap();
        db.merge_edge("fn:a:1", "fn:a:1", "CALLS").unwrap(); // self-call
        db.merge_edge("type:a:Foo:1", "fn:a:1", "HAS_METHOD").unwrap();
        db.merge_edge("mod:proj:src", "fn:a:1", "CONTAINS_FN").unwrap();
        assert_eq!(db.count_edges("proj").unwrap(), 3);
    }

    #[test]
    fn get_call_flow_structure() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_file("file:a.py", "a.py", "a", "python", "proj").unwrap();
        db.merge_function("fn:a:hello:1", "hello", "a.py", 1, "", "", "", 1, "proj").unwrap();
        db.merge_function("fn:a:helper:5", "helper", "a.py", 5, "", "", "", 1, "proj").unwrap();
        db.merge_edge("file:a.py", "fn:a:hello:1", "EXPORTS_FN").unwrap();
        db.merge_edge("fn:a:hello:1", "fn:a:helper:5", "CALLS").unwrap();
        let flow = db.get_call_flow("proj").unwrap();
        assert!(flow["moduleCount"].as_u64().unwrap() >= 1);
        assert!(flow["exportCount"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn doc_drift_workflow() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_file("file:a.py", "a.py", "a", "python", "proj").unwrap();
        db.merge_function("fn:a:foo:1", "foo", "a.py", 1, "", "", "", 1, "proj").unwrap();
        db.merge_doc("doc:readme", "README.md", "README", "readme", "proj").unwrap();
        db.merge_edge("doc:readme", "file:a.py", "COVERS").unwrap();
        db.merge_edge("doc:readme", "fn:a:foo:1", "MENTIONS_FN").unwrap();
        // Find drifted docs when file/fn changed
        let drifts = db.find_drifted_docs(&["file:a.py".into()], &["fn:a:foo:1".into()]).unwrap();
        assert_eq!(drifts.len(), 2); // one COVERS + one MENTIONS_FN
        // Record and retrieve
        db.record_doc_drift(&drifts, "proj").unwrap();
        let stored = db.get_doc_drift("proj").unwrap();
        assert_eq!(stored.len(), 2);
    }

    #[test]
    fn store_and_get_communities() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_function("fn:a:1", "foo", "a.py", 1, "", "", "", 1, "proj").unwrap();
        db.merge_function("fn:a:2", "bar", "a.py", 5, "", "", "", 1, "proj").unwrap();
        db.merge_file("file:a.py", "a.py", "a", "python", "proj").unwrap();
        let mut communities = std::collections::HashMap::new();
        communities.insert("fn:a:1".to_string(), 0u32);
        communities.insert("fn:a:2".to_string(), 0u32);
        communities.insert("file:a.py".to_string(), 1u32);
        db.store_communities(&communities).unwrap();
        let result = db.get_communities("proj").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn db_path_returns_some_for_file_db() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = GraphDb::open(dir.path()).unwrap();
        assert!(db.db_path().is_some());
        let mem = GraphDb::open_memory().unwrap();
        assert!(mem.db_path().is_none());
    }

    #[test]
    fn clone_connection_works() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = GraphDb::open(dir.path()).unwrap();
        db.merge_function("fn:a:1", "foo", "a.py", 1, "", "", "", 1, "proj").unwrap();
        let db2 = db.clone_connection().unwrap();
        let (fns, _) = db2.count_symbols("proj").unwrap();
        assert_eq!(fns, 1);
    }
}
