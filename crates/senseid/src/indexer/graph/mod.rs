mod nodes;
mod edges;
mod wrappers;
mod ir_readwrite;
mod branches;
mod refs;
mod community;
mod doc_drift;

use rusqlite::Connection;
use rusqlite::params;
use std::path::Path;

const SCHEMA_VERSION: i32 = 2;

/// Graph database backed by SQLite — unified hierarchy_nodes table.
pub struct GraphDb {
    pub(crate) conn: Connection,
    db_file: Option<std::path::PathBuf>,
}

impl GraphDb {
    pub fn conn_ref(&self) -> &Connection { &self.conn }

    pub fn open(path: &Path) -> Result<Self, String> {
        let db_path = path.join("graph.db");
        let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open graph DB: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;").ok();
        let db = Self { conn, db_file: Some(db_path) };
        db.ensure_schema()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn clone_connection(&self) -> Result<Self, String> {
        let db_path = self.db_file.as_ref().ok_or("Cannot clone in-memory DB")?;
        let conn = Connection::open(db_path).map_err(|e| format!("Failed to clone graph DB: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;").ok();
        let db = Self { conn, db_file: self.db_file.clone() };
        db.ensure_schema()?;
        Ok(db)
    }

    pub fn db_path(&self) -> Option<&std::path::Path> {
        self.db_file.as_deref()
    }

    #[cfg(test)]
    pub fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let db = Self { conn, db_file: None };
        db.ensure_schema()?;
        Ok(db)
    }

    fn ensure_schema(&self) -> Result<(), String> {
        // Check schema version — if old or missing, drop and recreate
        let version: i32 = self.conn.query_row(
            "SELECT version FROM schema_version LIMIT 1",
            [], |row| row.get(0),
        ).unwrap_or(0);

        if version < SCHEMA_VERSION {
            // Drop old tables (v1 schema)
            self.conn.execute_batch("
                DROP TABLE IF EXISTS functions;
                DROP TABLE IF EXISTS files;
                DROP TABLE IF EXISTS types;
                DROP TABLE IF EXISTS comments;
                DROP TABLE IF EXISTS docs;
                DROP TABLE IF EXISTS packages;
                DROP TABLE IF EXISTS modules;
                DROP TABLE IF EXISTS doc_drift;
                DROP TABLE IF EXISTS hierarchy_nodes;
                DROP TABLE IF EXISTS edges;
                DROP TABLE IF EXISTS schema_version;
            ").ok();
            // Clear all manifests so files are re-parsed on next index
            if let Some(sensei_dir) = dirs::home_dir().map(|h| h.join(".sensei").join("projects"))
                && sensei_dir.exists() {
                    for entry in std::fs::read_dir(&sensei_dir).into_iter().flatten().flatten() {
                        let manifest = entry.path().join("manifest.json");
                        if manifest.exists() {
                            std::fs::remove_file(&manifest).ok();
                        }
                    }
                    tracing::info!("Schema v{} → v{}: cleared all manifests for full re-index", version, SCHEMA_VERSION);
                }
        }

        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS hierarchy_nodes(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                level TEXT,
                parent_id TEXT,
                file TEXT,
                line INTEGER DEFAULT 0,
                project TEXT NOT NULL,
                sig TEXT,
                body TEXT,
                docstring TEXT,
                complexity INTEGER,
                tags TEXT,
                doc_type TEXT,
                doc_category TEXT
            );
            CREATE TABLE IF NOT EXISTS edges(
                from_id TEXT NOT NULL, to_id TEXT NOT NULL, edge_type TEXT NOT NULL,
                weight REAL,
                PRIMARY KEY(from_id, to_id, edge_type)
            );
            CREATE TABLE IF NOT EXISTS schema_version(version INTEGER);
            CREATE INDEX IF NOT EXISTS idx_hn_project ON hierarchy_nodes(project);
            CREATE INDEX IF NOT EXISTS idx_hn_kind ON hierarchy_nodes(project, kind);
            CREATE INDEX IF NOT EXISTS idx_hn_name ON hierarchy_nodes(name);
            CREATE INDEX IF NOT EXISTS idx_hn_file ON hierarchy_nodes(file);
            CREATE INDEX IF NOT EXISTS idx_hn_parent ON hierarchy_nodes(parent_id);

            -- IR extension tables (Option B: type-specific, cleaner than column bloat)
            CREATE TABLE IF NOT EXISTS ir_docs(
                node_id TEXT PRIMARY KEY REFERENCES hierarchy_nodes(id),
                frontmatter TEXT,       -- JSON: raw key-value pairs
                status TEXT,
                origin TEXT,            -- parent doc path for TRACES_TO
                description TEXT,
                date TEXT,
                sections TEXT,          -- JSON: [{heading, level, line_start, line_end, content_preview}]
                code_blocks TEXT,       -- JSON: [{language, content, line_start, line_end}]
                file_references TEXT,   -- JSON: [path, ...]
                symbol_references TEXT, -- JSON: [name, ...]
                doc_references TEXT     -- JSON: [path, ...]
            );

            CREATE TABLE IF NOT EXISTS ir_functions(
                node_id TEXT PRIMARY KEY REFERENCES hierarchy_nodes(id),
                params TEXT,            -- JSON: [{name, type_, default_value, is_optional}]
                return_type TEXT,
                is_async INTEGER DEFAULT 0,
                complexity INTEGER DEFAULT 1,
                body_hash TEXT,
                decorators TEXT,        -- JSON array
                calls TEXT              -- JSON: [name, ...]
            );

            CREATE TABLE IF NOT EXISTS ir_classes(
                node_id TEXT PRIMARY KEY REFERENCES hierarchy_nodes(id),
                class_kind TEXT,
                implements TEXT,         -- JSON array
                extends TEXT,
                generic_params TEXT,     -- JSON array
                decorators TEXT,         -- JSON array
                mixins TEXT             -- JSON array
            );
            CREATE TABLE IF NOT EXISTS unresolved_refs(
                source_id TEXT NOT NULL,
                ref_kind TEXT NOT NULL,
                ref_target TEXT NOT NULL,
                project TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_edge_from ON edges(from_id);
            CREATE INDEX IF NOT EXISTS idx_edge_to ON edges(to_id);
            CREATE INDEX IF NOT EXISTS idx_uref_project ON unresolved_refs(project);
        ").map_err(|e| e.to_string())?;

        // Set schema version
        self.conn.execute("DELETE FROM schema_version", []).ok();
        self.conn.execute("INSERT INTO schema_version(version) VALUES(?1)", params![SCHEMA_VERSION]).ok();

        // Migration: add community column if missing
        self.conn.execute_batch("ALTER TABLE hierarchy_nodes ADD COLUMN community INTEGER DEFAULT -1;").ok();

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocDrift {
    pub doc_id: String,
    pub doc_path: String,
    pub edge_type: String,
    pub changed_target: String,
}

/// Compute cyclomatic complexity estimate from code body.
pub fn compute_complexity(body: &str) -> u32 {
    let patterns = ["if ", "else if ", "elif ", "else ", "for ", "while ", "catch ",
        "case ", "&&", "||", "? ", "try ", "match ", "except "];
    let mut n: u32 = 1;
    for pat in &patterns { n += body.matches(pat).count() as u32; }
    n
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::types::{NodeKind, HierarchyNode};

    pub fn make_fn(id: &str, name: &str, file: &str, line: u32, project: &str) -> HierarchyNode {
        HierarchyNode::function(
            id.into(), name.into(), NodeKind::Function, file.into(), line,
            None, None, None, 1, project.into(),
        )
    }

    pub fn make_type(id: &str, name: &str, file: &str, line: u32, kind: NodeKind, project: &str) -> HierarchyNode {
        let mut n = HierarchyNode::group(id.into(), name.into(), kind, project.into());
        n.file = Some(file.into());
        n.line = line;
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_helpers::*;

    #[test]
    fn graph_db_opens() {
        let db = GraphDb::open_memory().unwrap();
        let counts = db.count_by_kind("test").unwrap();
        assert!(counts.is_empty());
    }

    #[test]
    fn complexity() {
        assert_eq!(compute_complexity("x = 1"), 1);
        assert_eq!(compute_complexity("if x { } else { }"), 3);
    }

    #[test]
    fn db_path_and_clone() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = GraphDb::open(dir.path()).unwrap();
        assert!(db.db_path().is_some());
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        let db2 = db.clone_connection().unwrap();
        let c = db2.count_by_kind("proj").unwrap();
        assert_eq!(c.get("function"), Some(&1));
    }
}
