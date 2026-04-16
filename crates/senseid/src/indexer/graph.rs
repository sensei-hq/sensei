use rusqlite::{Connection, params, OptionalExtension};
use std::path::Path;
use crate::types::{NodeKind, HierarchyNode};

const SCHEMA_VERSION: i32 = 2;

/// Graph database backed by SQLite — unified hierarchy_nodes table.
pub struct GraphDb {
    conn: Connection,
    db_file: Option<std::path::PathBuf>,
}

impl GraphDb {
    pub fn open(path: &Path) -> Result<Self, String> {
        let db_path = path.join("graph.db");
        let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open graph DB: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;").ok();
        let db = Self { conn, db_file: Some(db_path) };
        db.ensure_schema()?;
        Ok(db)
    }

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
            if let Some(sensei_dir) = dirs::home_dir().map(|h| h.join(".sensei").join("projects")) {
                if sensei_dir.exists() {
                    for entry in std::fs::read_dir(&sensei_dir).into_iter().flatten() {
                        if let Ok(entry) = entry {
                            let manifest = entry.path().join("manifest.json");
                            if manifest.exists() {
                                std::fs::remove_file(&manifest).ok();
                            }
                        }
                    }
                    tracing::info!("Schema v{} → v{}: cleared all manifests for full re-index", version, SCHEMA_VERSION);
                }
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

    // ── Node CRUD ────────────────────────────────────────────────────

    /// Insert or replace a node in the hierarchy.
    pub fn merge_node(&self, node: &HierarchyNode) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO hierarchy_nodes(
                id, name, kind, level, parent_id, file, line, project,
                sig, body, docstring, complexity, tags, doc_type, doc_category
            ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                node.id, node.name, node.kind.as_str(), node.level, node.parent_id,
                node.file, node.line, node.project,
                node.sig.as_deref().map(|s| &s[..s.len().min(500)]),
                node.body.as_deref().map(|s| &s[..s.len().min(10000)]),
                node.docstring.as_deref().map(|s| &s[..s.len().min(2000)]),
                node.complexity, node.tags, node.doc_type, node.doc_category,
            ],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a node and all edges involving it.
    pub fn delete_node(&self, id: &str) -> Result<(), String> {
        self.conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", params![id]).ok();
        self.conn.execute("DELETE FROM hierarchy_nodes WHERE id = ?1", params![id]).ok();
        Ok(())
    }

    /// Delete all nodes with a given file path (and their edges).
    pub fn delete_by_file(&self, file_path: &str, project: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE file = ?1 AND project = ?2)
             OR to_id IN (SELECT id FROM hierarchy_nodes WHERE file = ?1 AND project = ?2)",
            params![file_path, project],
        ).ok();
        self.conn.execute("DELETE FROM hierarchy_nodes WHERE file = ?1 AND project = ?2", params![file_path, project]).ok();
        Ok(())
    }

    /// Clear all hierarchy nodes (packages, modules, grouping nodes, doc containment) for a project.
    /// Preserves function/type/file/doc leaf nodes.
    pub fn clear_hierarchy(&self, project: &str) -> Result<(), String> {
        let hierarchy_kinds = "('repo','code-group','doc-group','package','module')";
        self.conn.execute(
            &format!("DELETE FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1 AND kind IN {}) OR to_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1 AND kind IN {})", hierarchy_kinds, hierarchy_kinds),
            params![project],
        ).ok();
        // Also delete containment and HAS_METHOD edges
        self.conn.execute(
            "DELETE FROM edges WHERE edge_type IN ('CONTAINS_PKG','CONTAINS_MOD','CONTAINS_FN','CONTAINS_FILE','CONTAINS_TYPE','CONTAINS_DOC','HAS_METHOD')
             AND (from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1) OR to_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1))",
            params![project],
        ).ok();
        self.conn.execute(
            &format!("DELETE FROM hierarchy_nodes WHERE project = ?1 AND kind IN {}", hierarchy_kinds),
            params![project],
        ).ok();
        Ok(())
    }

    pub fn clear_all(&self) -> Result<(), String> {
        self.conn.execute_batch("DELETE FROM hierarchy_nodes; DELETE FROM edges; DELETE FROM unresolved_refs;")
            .map_err(|e| e.to_string())
    }

    // ── Compatibility wrappers (delegate to merge_node/search_nodes) ──
    // These will be removed once all callers are migrated.

    pub fn merge_function(
        &self, id: &str, name: &str, file: &str, line: u32,
        sig: &str, body: &str, docstring: &str, complexity: u32, project: &str,
    ) -> Result<(), String> {
        self.merge_node(&HierarchyNode::function(
            id.into(), name.into(), NodeKind::Function, file.into(), line,
            if sig.is_empty() { None } else { Some(sig.into()) },
            if body.is_empty() { None } else { Some(body.into()) },
            if docstring.is_empty() { None } else { Some(docstring.into()) },
            complexity, project.into(),
        ))
    }

    pub fn merge_file(
        &self, id: &str, path: &str, module: &str, lang: &str, project: &str,
    ) -> Result<(), String> {
        let mut n = HierarchyNode::group(id.into(), module.into(), NodeKind::File, project.into());
        n.file = Some(path.into());
        n.level = Some(lang.into());
        self.merge_node(&n)
    }

    pub fn merge_type(
        &self, id: &str, name: &str, file: &str, line: u32, kind: &str, project: &str,
    ) -> Result<(), String> {
        let nk = NodeKind::from_str(kind);
        let mut n = HierarchyNode::group(id.into(), name.into(), nk, project.into());
        n.file = Some(file.into());
        n.line = line;
        self.merge_node(&n)
    }

    pub fn merge_doc(
        &self, id: &str, path: &str, title: &str, doc_type: &str, project: &str,
    ) -> Result<(), String> {
        self.merge_node(&HierarchyNode::doc(
            id.into(), title.into(), NodeKind::Doc, path.into(),
            Some(doc_type.into()), None, project.into(),
        ))
    }

    pub fn merge_package(
        &self, id: &str, name: &str, version: Option<&str>, path: &str, pkg_type: &str, project: &str,
    ) -> Result<(), String> {
        let mut n = HierarchyNode::group(id.into(), name.into(), NodeKind::Package, project.into());
        n.level = Some(pkg_type.into());
        n.file = Some(path.into());
        n.docstring = version.map(|v| v.into());
        self.merge_node(&n)
    }

    pub fn merge_module(
        &self, id: &str, name: &str, path: &str, package_id: Option<&str>, project: &str,
    ) -> Result<(), String> {
        let mut n = HierarchyNode::group(id.into(), name.into(), NodeKind::Module, project.into());
        n.file = Some(path.into());
        n.parent_id = package_id.map(|s| s.into());
        self.merge_node(&n)
    }

    pub fn delete_file(&self, abs_path: &str, project: &str) -> Result<(), String> {
        self.delete_by_file(abs_path, project)
    }

    pub fn delete_doc(&self, doc_id: &str) -> Result<(), String> {
        self.delete_node(doc_id)
    }

    pub fn count_symbols(&self, project: &str) -> Result<(u32, u32), String> {
        let counts = self.count_by_kind(project)?;
        let fns = counts.get("function").copied().unwrap_or(0)
            + counts.get("method").copied().unwrap_or(0)
            + counts.get("component").copied().unwrap_or(0)
            + counts.get("hook").copied().unwrap_or(0);
        let types = counts.get("class").copied().unwrap_or(0)
            + counts.get("struct").copied().unwrap_or(0)
            + counts.get("interface").copied().unwrap_or(0)
            + counts.get("enum").copied().unwrap_or(0)
            + counts.get("type").copied().unwrap_or(0);
        Ok((fns, types))
    }

    pub fn count_packages(&self, project: &str) -> Result<u32, String> {
        Ok(self.count_by_kind(project)?.get("package").copied().unwrap_or(0))
    }

    pub fn count_modules(&self, project: &str) -> Result<u32, String> {
        Ok(self.count_by_kind(project)?.get("module").copied().unwrap_or(0))
    }

    pub fn search_functions(&self, query: &str, project: &str) -> Result<Vec<crate::types::FunctionDetail>, String> {
        self.search_nodes(query, project, &["function", "method", "component", "hook"])
    }

    pub fn search_types(&self, query: &str, project: &str) -> Result<Vec<crate::types::TypeDetail>, String> {
        let results = self.search_nodes(query, project, &["class", "struct", "interface", "enum", "type"])?;
        Ok(results.into_iter().map(|f| crate::types::TypeDetail {
            id: f.id, name: f.name, file: f.file, line: f.line, kind: String::new(),
        }).collect())
    }

    pub fn find_function_by_name(&self, name: &str, project: &str) -> Result<Option<String>, String> {
        // Search across all function-like kinds
        self.conn.query_row(
            "SELECT id FROM hierarchy_nodes WHERE name = ?1 AND project = ?2 AND kind IN ('function','method','component','hook') LIMIT 1",
            params![name, project], |row| row.get(0),
        ).optional().map_err(|e| e.to_string())
    }

    pub fn tag_file(&self, id: &str, tags: &str) -> Result<(), String> { self.tag_node(id, tags) }
    pub fn tag_function(&self, id: &str, tags: &str) -> Result<(), String> { self.tag_node(id, tags) }

    pub fn files_by_tag(&self, tag: &str, project: &str) -> Result<Vec<(String, String, String)>, String> {
        self.nodes_by_tag(tag, project, "file")
    }

    // ── Edges ────────────────────────────────────────────────────────

    pub fn merge_edge(&self, from_id: &str, to_id: &str, edge_type: &str) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO edges(from_id, to_id, edge_type) VALUES(?1,?2,?3)",
            params![from_id, to_id, edge_type],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Queries ──────────────────────────────────────────────────────

    /// Get all nodes for a project.
    pub fn get_nodes(&self, project: &str) -> Result<Vec<crate::types::GraphNode>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, COALESCE(file,''), line, complexity, doc_type, level, parent_id FROM hierarchy_nodes WHERE project = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(crate::types::GraphNode {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                file: row.get(3)?,
                line: row.get(4)?,
                complexity: row.get(5).ok(),
                doc_type: row.get::<_, Option<String>>(6)?,
                level: row.get::<_, Option<String>>(7)?,
                parent_id: row.get::<_, Option<String>>(8)?,
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get all edges for a project (where from_id belongs to this project).
    pub fn get_edges(&self, project: &str) -> Result<Vec<crate::types::GraphEdge>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, e.to_id, e.edge_type FROM edges e
             WHERE e.from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)"
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

    /// Count nodes by kind for a project. Returns a map of kind → count.
    pub fn count_by_kind(&self, project: &str) -> Result<std::collections::HashMap<String, u32>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT kind, COUNT(*) FROM hierarchy_nodes WHERE project = ?1 GROUP BY kind"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<std::collections::HashMap<_, _>, _>>().map_err(|e| e.to_string())
    }

    /// Count edges for a project.
    pub fn count_edges(&self, project: &str) -> Result<u32, String> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)",
            params![project], |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    /// Search nodes by name pattern and optional kind filter.
    pub fn search_nodes(&self, query: &str, project: &str, kinds: &[&str]) -> Result<Vec<crate::types::FunctionDetail>, String> {
        let pattern = format!("%{}%", query);
        let kind_clause = if kinds.is_empty() {
            String::new()
        } else {
            let ks: Vec<String> = kinds.iter().map(|k| format!("'{}'", k)).collect();
            format!(" AND kind IN ({})", ks.join(","))
        };
        let sql = format!(
            "SELECT id, name, COALESCE(file,''), line, sig, docstring, COALESCE(complexity,1), COALESCE(tags,''), kind
             FROM hierarchy_nodes WHERE project = ?1 AND name LIKE ?2{} LIMIT 50", kind_clause
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
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

    /// Find a node by exact name and kind.
    pub fn find_node_by_name(&self, name: &str, project: &str, kind: &str) -> Result<Option<String>, String> {
        self.conn.query_row(
            "SELECT id FROM hierarchy_nodes WHERE name = ?1 AND project = ?2 AND kind = ?3 LIMIT 1",
            params![name, project, kind],
            |row| row.get(0),
        ).optional().map_err(|e| e.to_string())
    }

    /// Tag a node (sets the tags field).
    pub fn tag_node(&self, node_id: &str, tags: &str) -> Result<(), String> {
        self.conn.execute(
            "UPDATE hierarchy_nodes SET tags = ?2 WHERE id = ?1",
            params![node_id, tags],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get nodes by tag pattern and kind.
    pub fn nodes_by_tag(&self, tag: &str, project: &str, kind: &str) -> Result<Vec<(String, String, String)>, String> {
        let pattern = format!("%{}%", tag);
        let mut stmt = self.conn.prepare(
            "SELECT id, COALESCE(file,''), COALESCE(tags,'') FROM hierarchy_nodes WHERE project = ?1 AND kind = ?2 AND tags LIKE ?3 LIMIT 100"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project, kind, pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get callers of a function (nodes with CALLS edges pointing to it).
    pub fn callers_of(&self, fn_name: &str, project: &str) -> Result<Vec<crate::types::FunctionDetail>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.name, COALESCE(n.file,''), n.line, n.sig, n.docstring, COALESCE(n.complexity,1), COALESCE(n.tags,'')
             FROM hierarchy_nodes n
             JOIN edges e ON e.from_id = n.id
             WHERE e.edge_type = 'CALLS'
               AND e.to_id IN (SELECT id FROM hierarchy_nodes WHERE name = ?1 AND project = ?2)
               AND n.project = ?2
             LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![fn_name, project], |row| {
            Ok(crate::types::FunctionDetail {
                id: row.get(0)?, name: row.get(1)?, file: row.get(2)?, line: row.get(3)?,
                signature: row.get(4)?, docstring: row.get(5)?, complexity: row.get(6)?,
                tags: row.get::<_, String>(7).unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get callees of a function.
    pub fn callees_of(&self, fn_name: &str, project: &str) -> Result<Vec<crate::types::FunctionDetail>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.name, COALESCE(n.file,''), n.line, n.sig, n.docstring, COALESCE(n.complexity,1), COALESCE(n.tags,'')
             FROM hierarchy_nodes n
             JOIN edges e ON e.to_id = n.id
             WHERE e.edge_type = 'CALLS'
               AND e.from_id IN (SELECT id FROM hierarchy_nodes WHERE name = ?1 AND project = ?2)
               AND n.project = ?2
             LIMIT 50"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![fn_name, project], |row| {
            Ok(crate::types::FunctionDetail {
                id: row.get(0)?, name: row.get(1)?, file: row.get(2)?, line: row.get(3)?,
                signature: row.get(4)?, docstring: row.get(5)?, complexity: row.get(6)?,
                tags: row.get::<_, String>(7).unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Get call-flow: exported functions as roots, their callees as children.
    pub fn get_call_flow(&self, project: &str) -> Result<serde_json::Value, String> {
        // Layer 0: Files
        let files: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, name FROM hierarchy_nodes WHERE project = ?1 AND kind = 'file'"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![project], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }).map_err(|e| e.to_string())?;
            rows.filter_map(|r| r.ok()).collect()
        };

        // Layer 1: Exported functions
        let mut exports: Vec<serde_json::Value> = Vec::new();
        for (file_id, module) in &files {
            let mut stmt = self.conn.prepare(
                "SELECT n.id, n.name, COALESCE(n.file,''), n.line, COALESCE(n.complexity,1) FROM hierarchy_nodes n
                 JOIN edges e ON e.to_id = n.id
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
                exports.push(serde_json::json!({ "module": module, "fileId": file_id, "functions": fns }));
            }
        }

        // Layer 2: Call edges
        let calls: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT e.from_id, e.to_id FROM edges e
                 WHERE e.edge_type = 'CALLS'
                 AND e.from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1 AND kind IN ('function','method','component','hook'))"
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

    // ── Unresolved references (staging for resolve_edges) ──────────

    /// Store an unresolved reference (import, call, parent) for later resolution.
    pub fn add_unresolved_ref(&self, source_id: &str, ref_kind: &str, ref_target: &str, project: &str) -> Result<(), String> {
        self.conn.execute(
            "INSERT INTO unresolved_refs(source_id, ref_kind, ref_target, project) VALUES(?1,?2,?3,?4)",
            params![source_id, ref_kind, ref_target, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get all unresolved references for a project.
    pub fn get_unresolved_refs(&self, project: &str) -> Result<Vec<(String, String, String)>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT source_id, ref_kind, ref_target FROM unresolved_refs WHERE project = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Clear unresolved references for a project (after resolution).
    pub fn clear_unresolved_refs(&self, project: &str) -> Result<(), String> {
        self.conn.execute("DELETE FROM unresolved_refs WHERE project = ?1", params![project])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Clear unresolved references from a specific source node.
    pub fn clear_unresolved_refs_from(&self, source_id: &str, project: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM unresolved_refs WHERE source_id = ?1 AND project = ?2",
            params![source_id, project],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Community detection support ──────────────────────────────────

    pub fn store_communities(&self, communities: &std::collections::HashMap<String, u32>) -> Result<(), String> {
        for (node_id, comm) in communities {
            self.conn.execute("UPDATE hierarchy_nodes SET community = ?2 WHERE id = ?1", params![node_id, comm]).ok();
        }
        Ok(())
    }

    pub fn get_communities(&self, project: &str) -> Result<Vec<super::community::CommunityInfo>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT community, COUNT(*) as cnt, GROUP_CONCAT(name, ', ') as members
             FROM hierarchy_nodes
             WHERE project = ?1 AND community >= 0
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

    // ── Doc drift ────────────────────────────────────────────────────

    pub fn find_drifted_docs(
        &self, changed_file_ids: &[String], changed_fn_ids: &[String],
    ) -> Result<Vec<DocDrift>, String> {
        let mut drifts = Vec::new();
        for file_id in changed_file_ids {
            let mut stmt = self.conn.prepare(
                "SELECT n.id, COALESCE(n.file,''), e.edge_type FROM hierarchy_nodes n
                 JOIN edges e ON e.from_id = n.id
                 WHERE e.to_id = ?1 AND e.edge_type = 'COVERS'"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![file_id], |row| {
                Ok(DocDrift {
                    doc_id: row.get(0)?, doc_path: row.get(1)?,
                    edge_type: row.get(2)?, changed_target: file_id.clone(),
                })
            }).map_err(|e| e.to_string())?;
            for r in rows { if let Ok(d) = r { drifts.push(d); } }
        }
        for fn_id in changed_fn_ids {
            let mut stmt = self.conn.prepare(
                "SELECT n.id, COALESCE(n.file,''), e.edge_type FROM hierarchy_nodes n
                 JOIN edges e ON e.from_id = n.id
                 WHERE e.to_id = ?1 AND e.edge_type = 'MENTIONS_FN'"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![fn_id], |row| {
                Ok(DocDrift {
                    doc_id: row.get(0)?, doc_path: row.get(1)?,
                    edge_type: row.get(2)?, changed_target: fn_id.clone(),
                })
            }).map_err(|e| e.to_string())?;
            for r in rows { if let Ok(d) = r { drifts.push(d); } }
        }
        Ok(drifts)
    }

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

    pub fn get_doc_drift(&self, project: &str) -> Result<Vec<DocDrift>, String> {
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
                doc_id: row.get(0)?, doc_path: row.get(1)?,
                edge_type: row.get(2)?, changed_target: row.get(3)?,
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

/// Compute cyclomatic complexity estimate from code body.
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
    use crate::types::{NodeKind, HierarchyNode};

    fn make_fn(id: &str, name: &str, file: &str, line: u32, project: &str) -> HierarchyNode {
        HierarchyNode::function(
            id.into(), name.into(), NodeKind::Function, file.into(), line,
            None, None, None, 1, project.into(),
        )
    }

    fn make_type(id: &str, name: &str, file: &str, line: u32, kind: NodeKind, project: &str) -> HierarchyNode {
        let mut n = HierarchyNode::group(id.into(), name.into(), kind, project.into());
        n.file = Some(file.into());
        n.line = line;
        n
    }

    #[test]
    fn graph_db_opens() {
        let db = GraphDb::open_memory().unwrap();
        let counts = db.count_by_kind("test").unwrap();
        assert!(counts.is_empty());
    }

    #[test]
    fn merge_and_count_nodes() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:hello:1", "hello", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:world:5", "world", "a.py", 5, "proj")).unwrap();
        let counts = db.count_by_kind("proj").unwrap();
        assert_eq!(counts.get("function"), Some(&2));
    }

    #[test]
    fn merge_edge_and_query() {
        let db = GraphDb::open_memory().unwrap();
        let mut file = HierarchyNode::group("file:a.py".into(), "a".into(), NodeKind::File, "proj".into());
        file.file = Some("a.py".into());
        db.merge_node(&file).unwrap();
        db.merge_node(&make_fn("fn:a:foo:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_edge("file:a.py", "fn:a:foo:1", "EXPORTS_FN").unwrap();
        let edges = db.get_edges("proj").unwrap();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn delete_by_file() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:foo:1", "foo", "/tmp/a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:bar:5", "bar", "/tmp/a.py", 5, "proj")).unwrap();
        db.merge_node(&make_fn("fn:b:baz:1", "baz", "/tmp/b.py", 1, "proj")).unwrap();
        let c = db.count_by_kind("proj").unwrap();
        assert_eq!(c.get("function"), Some(&3));
        db.delete_by_file("/tmp/a.py", "proj").unwrap();
        let c2 = db.count_by_kind("proj").unwrap();
        assert_eq!(c2.get("function"), Some(&1));
    }

    #[test]
    fn get_nodes_and_edges() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:5", "bar", "a.py", 5, "proj")).unwrap();
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
        assert_eq!(compute_complexity("if x { } else { }"), 3);
    }

    #[test]
    fn search_nodes_by_name() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:hello:1", "hello", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:help:5", "help", "a.py", 5, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:world:10", "world", "a.py", 10, "proj")).unwrap();
        let results = db.search_nodes("hel", "proj", &["function"]).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn callers_and_callees() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:main:1", "main", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:helper:5", "helper", "a.py", 5, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:util:10", "util", "a.py", 10, "proj")).unwrap();
        db.merge_edge("fn:a:main:1", "fn:a:helper:5", "CALLS").unwrap();
        db.merge_edge("fn:a:helper:5", "fn:a:util:10", "CALLS").unwrap();
        let callers = db.callers_of("helper", "proj").unwrap();
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].name, "main");
        let callees = db.callees_of("helper", "proj").unwrap();
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "util");
    }

    #[test]
    fn tag_and_find_by_tag() {
        let db = GraphDb::open_memory().unwrap();
        let mut f = HierarchyNode::group("file:a.ts".into(), "a".into(), NodeKind::File, "proj".into());
        f.file = Some("a.ts".into());
        db.merge_node(&f).unwrap();
        db.tag_node("file:a.ts", "react,hooks").unwrap();
        let files = db.nodes_by_tag("react", "proj", "file").unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn find_node_by_name() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:hello:1", "hello", "a.py", 1, "proj")).unwrap();
        let found = db.find_node_by_name("hello", "proj", "function").unwrap();
        assert_eq!(found, Some("fn:a:hello:1".to_string()));
        let missing = db.find_node_by_name("missing", "proj", "function").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn packages_and_modules() {
        let db = GraphDb::open_memory().unwrap();
        let mut pkg = HierarchyNode::group("pkg:proj:ui".into(), "ui".into(), NodeKind::Package, "proj".into());
        pkg.level = Some("npm-workspace".into());
        db.merge_node(&pkg).unwrap();
        let mut module = HierarchyNode::group("mod:proj:ui/src".into(), "ui/src".into(), NodeKind::Module, "proj".into());
        module.parent_id = Some("pkg:proj:ui".into());
        db.merge_node(&module).unwrap();
        let counts = db.count_by_kind("proj").unwrap();
        assert_eq!(counts.get("package"), Some(&1));
        assert_eq!(counts.get("module"), Some(&1));
    }

    #[test]
    fn containment_edges() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&HierarchyNode::group("pkg:proj:ui".into(), "ui".into(), NodeKind::Package, "proj".into())).unwrap();
        db.merge_node(&HierarchyNode::group("mod:proj:src".into(), "src".into(), NodeKind::Module, "proj".into())).unwrap();
        db.merge_node(&make_fn("fn:a:foo:1", "foo", "a.ts", 1, "proj")).unwrap();
        db.merge_node(&make_type("type:a:Foo:1", "Foo", "a.ts", 1, NodeKind::Class, "proj")).unwrap();
        db.merge_edge("pkg:proj:ui", "mod:proj:src", "CONTAINS_MOD").unwrap();
        db.merge_edge("mod:proj:src", "fn:a:foo:1", "CONTAINS_FN").unwrap();
        db.merge_edge("type:a:Foo:1", "fn:a:foo:1", "HAS_METHOD").unwrap();
        let edges = db.get_edges("proj").unwrap();
        let types: Vec<&str> = edges.iter().map(|e| e.edge_type.as_str()).collect();
        assert!(types.contains(&"CONTAINS_MOD"));
        assert!(types.contains(&"CONTAINS_FN"));
        assert!(types.contains(&"HAS_METHOD"));
    }

    #[test]
    fn clear_all_works() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_edge("fn:a:1", "fn:a:1", "CALLS").unwrap();
        db.clear_all().unwrap();
        let counts = db.count_by_kind("proj").unwrap();
        assert!(counts.is_empty());
    }

    #[test]
    fn doc_drift_workflow() {
        let db = GraphDb::open_memory().unwrap();
        let mut f = HierarchyNode::group("file:a.py".into(), "a".into(), NodeKind::File, "proj".into());
        f.file = Some("a.py".into());
        db.merge_node(&f).unwrap();
        db.merge_node(&make_fn("fn:a:foo:1", "foo", "a.py", 1, "proj")).unwrap();
        let doc = HierarchyNode::doc("doc:readme".into(), "README".into(), NodeKind::Doc, "README.md".into(), Some("usage".into()), None, "proj".into());
        db.merge_node(&doc).unwrap();
        db.merge_edge("doc:readme", "file:a.py", "COVERS").unwrap();
        db.merge_edge("doc:readme", "fn:a:foo:1", "MENTIONS_FN").unwrap();
        let drifts = db.find_drifted_docs(&["file:a.py".into()], &["fn:a:foo:1".into()]).unwrap();
        assert_eq!(drifts.len(), 2);
        db.record_doc_drift(&drifts, "proj").unwrap();
        let stored = db.get_doc_drift("proj").unwrap();
        assert_eq!(stored.len(), 2);
    }

    #[test]
    fn communities() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_fn("fn:a:2", "bar", "a.py", 5, "proj")).unwrap();
        let mut comms = std::collections::HashMap::new();
        comms.insert("fn:a:1".to_string(), 0u32);
        comms.insert("fn:a:2".to_string(), 0u32);
        db.store_communities(&comms).unwrap();
        let result = db.get_communities("proj").unwrap();
        assert!(!result.is_empty());
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

    #[test]
    fn count_edges_works() {
        let db = GraphDb::open_memory().unwrap();
        db.merge_node(&make_fn("fn:a:1", "foo", "a.py", 1, "proj")).unwrap();
        db.merge_node(&make_type("type:a:Foo:1", "Foo", "a.py", 1, NodeKind::Class, "proj")).unwrap();
        db.merge_node(&HierarchyNode::group("mod:proj:src".into(), "src".into(), NodeKind::Module, "proj".into())).unwrap();
        db.merge_edge("fn:a:1", "fn:a:1", "CALLS").unwrap();
        db.merge_edge("type:a:Foo:1", "fn:a:1", "HAS_METHOD").unwrap();
        db.merge_edge("mod:proj:src", "fn:a:1", "CONTAINS_FN").unwrap();
        assert_eq!(db.count_edges("proj").unwrap(), 3);
    }

    #[test]
    fn get_call_flow_structure() {
        let db = GraphDb::open_memory().unwrap();
        let mut f = HierarchyNode::group("file:a.py".into(), "a".into(), NodeKind::File, "proj".into());
        f.file = Some("a.py".into());
        db.merge_node(&f).unwrap();
        db.merge_node(&make_fn("fn:a:hello:1", "hello", "a.py", 1, "proj")).unwrap();
        db.merge_edge("file:a.py", "fn:a:hello:1", "EXPORTS_FN").unwrap();
        let flow = db.get_call_flow("proj").unwrap();
        assert!(flow["moduleCount"].as_u64().unwrap() >= 1);
    }
}
