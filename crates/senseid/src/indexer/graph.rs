use rusqlite::{Connection, params, OptionalExtension};
use std::path::Path;
use crate::types::SymbolKind;

/// Graph database backed by SQLite (interim — will migrate to Kuzu).
/// Uses adjacency list tables for nodes and edges.
pub struct GraphDb {
    conn: Connection,
}

impl GraphDb {
    pub fn open(path: &Path) -> Result<Self, String> {
        let db_path = path.join("graph.db");
        let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open graph DB: {}", e))?;
        let db = Self { conn };
        db.ensure_schema()?;
        Ok(db)
    }

    pub fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let db = Self { conn };
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
            "SELECT COUNT(*) FROM edges e JOIN functions f ON e.from_id = f.id WHERE f.project = ?1",
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
             SELECT id, module as name, 'file' as kind, path as file, 0, 0 FROM files WHERE project = ?1"
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
}
