use rusqlite::{Connection, params};
#[cfg(test)]
use rusqlite::OptionalExtension;
use std::path::Path;
use crate::types::{NodeKind, HierarchyNode};

const SCHEMA_VERSION: i32 = 2;

/// Graph database backed by SQLite — unified hierarchy_nodes table.
pub struct GraphDb {
    conn: Connection,
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
        let hierarchy_kinds = "('repo','package','module')";
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

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn merge_file(
        &self, id: &str, path: &str, module: &str, lang: &str, project: &str,
    ) -> Result<(), String> {
        let mut n = HierarchyNode::group(id.into(), module.into(), NodeKind::File, project.into());
        n.file = Some(path.into());
        n.level = Some(lang.into());
        self.merge_node(&n)
    }

    #[cfg(test)]
    pub fn merge_type(
        &self, id: &str, name: &str, file: &str, line: u32, kind: &str, project: &str,
    ) -> Result<(), String> {
        let nk = NodeKind::from_str(kind);
        let mut n = HierarchyNode::group(id.into(), name.into(), nk, project.into());
        n.file = Some(file.into());
        n.line = line;
        self.merge_node(&n)
    }

    #[cfg(test)]
    pub fn merge_doc(
        &self, id: &str, path: &str, title: &str, doc_type: &str, project: &str,
    ) -> Result<(), String> {
        self.merge_node(&HierarchyNode::doc(
            id.into(), title.into(), NodeKind::Doc, path.into(),
            Some(doc_type.into()), None, project.into(),
        ))
    }

    #[cfg(test)]
    pub fn merge_package(
        &self, id: &str, name: &str, version: Option<&str>, path: &str, pkg_type: &str, project: &str,
    ) -> Result<(), String> {
        let mut n = HierarchyNode::group(id.into(), name.into(), NodeKind::Package, project.into());
        n.level = Some(pkg_type.into());
        n.file = Some(path.into());
        n.docstring = version.map(|v| v.into());
        self.merge_node(&n)
    }

    #[cfg(test)]
    pub fn merge_module(
        &self, id: &str, name: &str, path: &str, package_id: Option<&str>, project: &str,
    ) -> Result<(), String> {
        let mut n = HierarchyNode::group(id.into(), name.into(), NodeKind::Module, project.into());
        n.file = Some(path.into());
        n.parent_id = package_id.map(|s| s.into());
        self.merge_node(&n)
    }

    /// Write an IRDoc to the graph — hierarchy_nodes base + ir_docs extension.
    #[cfg(test)]
    pub fn write_ir_doc(&self, doc: &crate::ir::IRDoc, project: &str) -> Result<(), String> {
        let id = format!("doc:{}", doc.base.file);

        // Write base node
        let n = HierarchyNode {
            id: id.clone(),
            name: doc.title.clone().unwrap_or_else(|| doc.base.name.clone()),
            kind: NodeKind::Doc,
            level: doc.base.language.clone(),
            parent_id: None,
            file: Some(doc.base.file.clone()),
            line: 0,
            project: project.into(),
            sig: None,
            body: None,
            docstring: doc.description.clone(),
            complexity: None,
            tags: Some(doc.base.tags.join(",")),
            doc_type: doc.doc_type.clone(),
            doc_category: doc.base.category.clone(),
        };
        self.merge_node(&n)?;

        // Write IR extension
        let sections_json = serde_json::to_string(&doc.sections).unwrap_or_default();
        let code_blocks_json = serde_json::to_string(&doc.code_blocks).unwrap_or_default();
        let frontmatter_json = serde_json::to_string(&doc.frontmatter).unwrap_or_default();
        let file_refs_json = serde_json::to_string(&doc.file_references).unwrap_or_default();
        let sym_refs_json = serde_json::to_string(&doc.symbol_references).unwrap_or_default();
        let doc_refs_json = serde_json::to_string(&doc.doc_references).unwrap_or_default();

        self.conn.execute(
            "INSERT OR REPLACE INTO ir_docs(node_id, frontmatter, status, origin, description, date, sections, code_blocks, file_references, symbol_references, doc_references)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                id, frontmatter_json,
                doc.status, doc.origin, doc.description, doc.date,
                sections_json, code_blocks_json,
                file_refs_json, sym_refs_json, doc_refs_json,
            ],
        ).map_err(|e| e.to_string())?;

        // Create TRACES_TO edge from origin
        if let Some(ref origin) = doc.origin {
            let origin_id = format!("doc:{}", origin);
            self.merge_edge(&id, &origin_id, "TRACES_TO")?;
        }

        // Create COVERS edges from file references
        for file_ref in &doc.file_references {
            let file_id = format!("file:{}", file_ref);
            self.merge_edge(&id, &file_id, "COVERS")?;
        }

        // Create MENTIONS edges from symbol references
        for sym_ref in &doc.symbol_references {
            // Symbol IDs are harder — just store the name reference for now
            self.conn.execute(
                "INSERT OR IGNORE INTO edges(from_id, to_id, edge_type) VALUES(?1, ?2, 'MENTIONS_FN')",
                rusqlite::params![id, format!("fn:{}:{}", project, sym_ref)],
            ).ok();
        }

        Ok(())
    }

    /// Read an IRDoc back from the graph (for testing and queries).
    #[cfg(test)]
    pub fn read_ir_doc(&self, node_id: &str) -> Result<Option<crate::ir::IRDoc>, String> {
        // Read base node
        let node = self.conn.query_row(
            "SELECT name, file, level, doc_type, doc_category, tags, docstring FROM hierarchy_nodes WHERE id=?1",
            params![node_id],
            |row| Ok(HierarchyNode {
                id: node_id.into(),
                name: row.get(0)?,
                kind: NodeKind::Doc,
                level: row.get(2)?,
                parent_id: None,
                file: row.get(1)?,
                line: 0,
                project: String::new(),
                sig: None, body: None,
                docstring: row.get(6)?,
                complexity: None,
                tags: row.get(5)?,
                doc_type: row.get(3)?,
                doc_category: row.get(4)?,
            }),
        ).optional().map_err(|e| e.to_string())?;

        let node = match node {
            Some(n) => n,
            None => return Ok(None),
        };

        let ir_row = self.conn.query_row(
            "SELECT frontmatter, status, origin, description, date, sections, code_blocks, file_references, symbol_references, doc_references FROM ir_docs WHERE node_id=?1",
            rusqlite::params![node_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            },
        ).optional().map_err(|e| e.to_string())?;

        let (fm_json, status, origin, desc, date, sections_json, blocks_json, file_refs_json, sym_refs_json, doc_refs_json) = match ir_row {
            Some(r) => r,
            None => return Ok(None),
        };

        let frontmatter: std::collections::HashMap<String, String> = fm_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let sections: Vec<crate::ir::IRSection> = sections_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let code_blocks: Vec<crate::ir::IRCodeBlock> = blocks_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let file_references: Vec<String> = file_refs_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let symbol_references: Vec<String> = sym_refs_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let doc_references: Vec<String> = doc_refs_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();

        let title = node.name.clone();
        Ok(Some(crate::ir::IRDoc {
            base: crate::ir::IRBase {
                name: node.name,
                file: node.file.unwrap_or_default(),
                language: node.level,
                category: node.doc_category,
                tags: node.tags.map(|t| t.split(',').map(String::from).collect()).unwrap_or_default(),
                ..Default::default()
            },
            doc_type: node.doc_type,
            frontmatter,
            status,
            origin,
            description: desc,
            date,
            title: Some(title),
            sections,
            code_blocks,
            file_references,
            symbol_references,
            doc_references,
        }))
    }

    /// Write an IRFunction to the graph — hierarchy_nodes base + ir_functions extension.
    /// If `parent_class_id` is Some, this is a method and gets a HAS_METHOD edge.
    pub fn write_ir_function(&self, func: &crate::ir::IRFunction, project: &str, parent_class_id: Option<&str>) -> Result<(), String> {
        let id = format!("fn:{}:{}:{}", project, func.base.file, func.base.name);

        let kind = if parent_class_id.is_some() { NodeKind::Method } else { NodeKind::Function };
        let n = HierarchyNode {
            id: id.clone(),
            name: func.base.name.clone(),
            kind,
            level: func.base.language.clone(),
            parent_id: parent_class_id.map(|s| s.into()),
            file: Some(func.base.file.clone()),
            line: func.base.line_start,
            project: project.into(),
            sig: func.return_type.as_ref().map(|rt| {
                let params_str: Vec<String> = func.params.iter().map(|p| {
                    match &p.type_ {
                        Some(t) => format!("{}: {}", p.name, t),
                        None => p.name.clone(),
                    }
                }).collect();
                format!("({}) -> {}", params_str.join(", "), rt)
            }),
            body: None,
            docstring: func.base.docstring.clone(),
            complexity: Some(func.complexity),
            tags: if func.base.tags.is_empty() { None } else { Some(func.base.tags.join(",")) },
            doc_type: None,
            doc_category: func.base.category.clone(),
        };
        self.merge_node(&n)?;

        // Write IR extension
        let params_json = serde_json::to_string(&func.params).unwrap_or_default();
        let decorators_json = serde_json::to_string(&func.decorators).unwrap_or_default();
        let calls_json = serde_json::to_string(&func.calls).unwrap_or_default();

        self.conn.execute(
            "INSERT OR REPLACE INTO ir_functions(node_id, params, return_type, is_async, complexity, body_hash, decorators, calls)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id, params_json, func.return_type, func.is_async as i32,
                func.complexity, func.body_hash, decorators_json, calls_json,
            ],
        ).map_err(|e| e.to_string())?;

        // Wire method to class
        if let Some(class_id) = parent_class_id {
            self.merge_edge(class_id, &id, "HAS_METHOD")?;
        }

        Ok(())
    }

    /// Read an IRFunction back from the graph.
    #[cfg(test)]
    pub fn read_ir_function(&self, node_id: &str) -> Result<Option<crate::ir::IRFunction>, String> {
        let node = self.conn.query_row(
            "SELECT name, file, line, docstring, tags, level FROM hierarchy_nodes WHERE id=?1",
            params![node_id],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            )),
        ).optional().map_err(|e| e.to_string())?;

        let (name, file, line, docstring, tags, language) = match node {
            Some(n) => n,
            None => return Ok(None),
        };

        let ir_row = self.conn.query_row(
            "SELECT params, return_type, is_async, complexity, body_hash, decorators, calls FROM ir_functions WHERE node_id=?1",
            params![node_id],
            |row| Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            )),
        ).optional().map_err(|e| e.to_string())?;

        let (params_json, return_type, is_async, complexity, body_hash, decorators_json, calls_json) = match ir_row {
            Some(r) => r,
            None => return Ok(None),
        };

        let params: Vec<crate::ir::IRParam> = params_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let decorators: Vec<String> = decorators_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let calls: Vec<String> = calls_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();

        Ok(Some(crate::ir::IRFunction {
            base: crate::ir::IRBase {
                name,
                file: file.unwrap_or_default(),
                line_start: line,
                language,
                docstring,
                tags: tags.map(|t| t.split(',').map(String::from).collect()).unwrap_or_default(),
                ..Default::default()
            },
            params,
            return_type,
            is_async: is_async != 0,
            decorators,
            calls,
            complexity,
            body_hash,
        }))
    }

    /// Write an IRClass to the graph — hierarchy_nodes base + ir_classes extension + edges.
    /// Creates IMPLEMENTS edges, EXTENDS edge, and HAS_METHOD edges for methods.
    pub fn write_ir_class(&self, class: &crate::ir::IRClass, project: &str) -> Result<(), String> {
        let id = format!("class:{}:{}:{}", project, class.base.file, class.base.name);

        let kind = match class.class_kind {
            crate::ir::ClassKind::Struct => NodeKind::Struct,
            crate::ir::ClassKind::Interface => NodeKind::Interface,
            crate::ir::ClassKind::Trait => NodeKind::Interface,
            crate::ir::ClassKind::Enum => NodeKind::Enum,
            _ => NodeKind::Class,
        };

        let n = HierarchyNode {
            id: id.clone(),
            name: class.base.name.clone(),
            kind,
            level: class.base.language.clone(),
            parent_id: None,
            file: Some(class.base.file.clone()),
            line: class.base.line_start,
            project: project.into(),
            sig: None,
            body: None,
            docstring: class.base.docstring.clone(),
            complexity: None,
            tags: if class.base.tags.is_empty() { None } else { Some(class.base.tags.join(",")) },
            doc_type: None,
            doc_category: class.base.category.clone(),
        };
        self.merge_node(&n)?;

        // Write IR extension
        let implements_json = serde_json::to_string(&class.implements).unwrap_or_default();
        let generic_params_json = serde_json::to_string(&class.generic_params).unwrap_or_default();
        let decorators_json = serde_json::to_string(&class.decorators).unwrap_or_default();
        let mixins_json = serde_json::to_string(&class.mixins).unwrap_or_default();
        let class_kind_str = serde_json::to_string(&class.class_kind)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        self.conn.execute(
            "INSERT OR REPLACE INTO ir_classes(node_id, class_kind, implements, extends, generic_params, decorators, mixins)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id, class_kind_str, implements_json, class.extends,
                generic_params_json, decorators_json, mixins_json,
            ],
        ).map_err(|e| e.to_string())?;

        // Create IMPLEMENTS edges
        for iface in &class.implements {
            let target_id = format!("class:{}:*:{}", project, iface);
            self.merge_edge(&id, &target_id, "IMPLEMENTS")?;
        }

        // Create EXTENDS edge
        if let Some(ref parent) = class.extends {
            let target_id = format!("class:{}:*:{}", project, parent);
            self.merge_edge(&id, &target_id, "EXTENDS")?;
        }

        // Write methods as function nodes with HAS_METHOD edges
        for method in &class.methods {
            let method_as_func = crate::ir::IRFunction {
                base: method.base.clone(),
                params: method.params.clone(),
                return_type: method.return_type.clone(),
                is_async: method.is_async,
                decorators: method.decorators.clone(),
                calls: method.calls.clone(),
                complexity: method.complexity,
                body_hash: method.body_hash.clone(),
            };
            self.write_ir_function(&method_as_func, project, Some(&id))?;
        }

        Ok(())
    }

    /// Read an IRClass back from the graph.
    #[cfg(test)]
    pub fn read_ir_class(&self, node_id: &str) -> Result<Option<crate::ir::IRClass>, String> {
        let node = self.conn.query_row(
            "SELECT name, file, line, docstring, tags, level FROM hierarchy_nodes WHERE id=?1",
            params![node_id],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            )),
        ).optional().map_err(|e| e.to_string())?;

        let (name, file, line, docstring, tags, language) = match node {
            Some(n) => n,
            None => return Ok(None),
        };

        let ir_row = self.conn.query_row(
            "SELECT class_kind, implements, extends, generic_params, decorators, mixins FROM ir_classes WHERE node_id=?1",
            params![node_id],
            |row| Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            )),
        ).optional().map_err(|e| e.to_string())?;

        let (class_kind_str, implements_json, extends, generic_params_json, decorators_json, mixins_json) = match ir_row {
            Some(r) => r,
            None => return Ok(None),
        };

        let class_kind: crate::ir::ClassKind = class_kind_str
            .and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok())
            .unwrap_or_default();
        let implements: Vec<String> = implements_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let generic_params: Vec<String> = generic_params_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let decorators: Vec<String> = decorators_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let mixins: Vec<String> = mixins_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();

        Ok(Some(crate::ir::IRClass {
            base: crate::ir::IRBase {
                name,
                file: file.unwrap_or_default(),
                line_start: line,
                language,
                docstring,
                tags: tags.map(|t| t.split(',').map(String::from).collect()).unwrap_or_default(),
                ..Default::default()
            },
            class_kind,
            implements,
            extends,
            generic_params,
            decorators,
            mixins,
            ..Default::default()
        }))
    }

    #[cfg(test)]
    pub fn delete_file(&self, abs_path: &str, project: &str) -> Result<(), String> {
        self.delete_by_file(abs_path, project)
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn find_function_by_name(&self, name: &str, project: &str) -> Result<Option<String>, String> {
        // Search across all function-like kinds
        self.conn.query_row(
            "SELECT id FROM hierarchy_nodes WHERE name = ?1 AND project = ?2 AND kind IN ('function','method','component','hook') LIMIT 1",
            params![name, project], |row| row.get(0),
        ).optional().map_err(|e| e.to_string())
    }

    #[cfg(test)]
    pub fn tag_file(&self, id: &str, tags: &str) -> Result<(), String> { self.tag_node(id, tags) }
    #[cfg(test)]
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
            "SELECT id, name, kind, COALESCE(file,''), line, complexity, doc_type, level, parent_id, tags FROM hierarchy_nodes WHERE project = ?1"
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
                tags: row.get::<_, Option<String>>(9)?,
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
    #[cfg(test)]
    pub fn find_node_by_name(&self, name: &str, project: &str, kind: &str) -> Result<Option<String>, String> {
        self.conn.query_row(
            "SELECT id FROM hierarchy_nodes WHERE name = ?1 AND project = ?2 AND kind = ?3 LIMIT 1",
            params![name, project, kind],
            |row| row.get(0),
        ).optional().map_err(|e| e.to_string())
    }

    /// Tag a node (sets the tags field).
    #[cfg(test)]
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

    // ── Branch graph operations ────────────────────────────────────

    /// Clone all nodes and edges from one project to another (for branch snapshots).
    /// Source: repo_id (current branch), Target: repo_id@branch_name
    pub fn clone_project_graph(&self, source_project: &str, target_project: &str) -> Result<u32, String> {
        // Clone nodes
        let count: u32 = self.conn.execute(
            "INSERT OR REPLACE INTO hierarchy_nodes(id, name, kind, level, parent_id, file, line, project, sig, body, docstring, complexity, tags, doc_type, doc_category)
             SELECT
                REPLACE(id, ?1, ?2), name, kind, level,
                CASE WHEN parent_id IS NOT NULL THEN REPLACE(parent_id, ?1, ?2) ELSE NULL END,
                file, line, ?2, sig, body, docstring, complexity, tags, doc_type, doc_category
             FROM hierarchy_nodes WHERE project = ?1",
            params![source_project, target_project],
        ).map_err(|e| e.to_string())? as u32;

        // Clone edges (only those between nodes in the source project)
        self.conn.execute(
            "INSERT OR REPLACE INTO edges(from_id, to_id, edge_type, weight)
             SELECT REPLACE(from_id, ?1, ?2), REPLACE(to_id, ?1, ?2), edge_type, weight
             FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)",
            params![source_project, target_project],
        ).ok();

        Ok(count)
    }

    /// Delete all nodes and edges for a project (branch cleanup).
    pub fn delete_project_graph(&self, project: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)
             OR to_id IN (SELECT id FROM hierarchy_nodes WHERE project = ?1)",
            params![project],
        ).ok();
        self.conn.execute("DELETE FROM hierarchy_nodes WHERE project = ?1", params![project]).ok();
        self.conn.execute("DELETE FROM unresolved_refs WHERE project = ?1", params![project]).ok();
        Ok(())
    }

    /// Check if a project graph exists (has any nodes).
    pub fn project_exists(&self, project: &str) -> bool {
        self.conn.query_row(
            "SELECT COUNT(*) FROM hierarchy_nodes WHERE project = ?1 LIMIT 1",
            params![project], |row| row.get::<_, u32>(0),
        ).unwrap_or(0) > 0
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    // ── IR Doc write/read tests ──────────────────────────────────────

    #[test]
    fn write_and_read_ir_doc_with_frontmatter() {
        let db = GraphDb::open_memory().unwrap();

        let mut fm = std::collections::HashMap::new();
        fm.insert("name".into(), "Workflow System".into());
        fm.insert("status".into(), "complete".into());

        let doc = crate::ir::IRDoc {
            base: crate::ir::IRBase {
                name: "Workflow System".into(),
                file: "docs/ideas/01-workflow.md".into(),
                language: Some("markdown".into()),
                category: Some("idea".into()),
                tags: vec!["doc".into()],
                ..Default::default()
            },
            doc_type: Some("idea".into()),
            frontmatter: fm,
            status: Some("complete".into()),
            origin: Some("conversation".into()),
            description: Some("A phased development workflow".into()),
            date: Some("2026-04-17".into()),
            title: Some("Workflow System".into()),
            sections: vec![
                crate::ir::IRSection { heading: "Problem".into(), level: 2, line_start: 5, line_end: 15, content_preview: Some("AI-assisted...".into()) },
                crate::ir::IRSection { heading: "Solution".into(), level: 2, line_start: 16, line_end: 30, content_preview: None },
            ],
            file_references: vec!["src/main.rs".into()],
            symbol_references: vec!["parse_file".into()],
            doc_references: vec!["docs/analysis/01.md".into()],
            ..Default::default()
        };

        db.write_ir_doc(&doc, "test-proj").unwrap();

        // Read back
        let read = db.read_ir_doc("doc:docs/ideas/01-workflow.md").unwrap().unwrap();
        assert_eq!(read.title, Some("Workflow System".into()));
        assert_eq!(read.doc_type, Some("idea".into()));
        assert_eq!(read.status, Some("complete".into()));
        assert_eq!(read.origin, Some("conversation".into()));
        assert_eq!(read.description, Some("A phased development workflow".into()));
        assert_eq!(read.date, Some("2026-04-17".into()));
        assert_eq!(read.sections.len(), 2);
        assert_eq!(read.sections[0].heading, "Problem");
        assert_eq!(read.sections[1].line_start, 16);
        assert_eq!(read.file_references, vec!["src/main.rs"]);
        assert_eq!(read.symbol_references, vec!["parse_file"]);
        assert_eq!(read.frontmatter["name"], "Workflow System");
    }

    #[test]
    fn write_ir_doc_without_frontmatter() {
        let db = GraphDb::open_memory().unwrap();

        let doc = crate::ir::IRDoc {
            base: crate::ir::IRBase {
                name: "README".into(),
                file: "README.md".into(),
                ..Default::default()
            },
            title: Some("Project Readme".into()),
            ..Default::default()
        };

        db.write_ir_doc(&doc, "test-proj").unwrap();

        let read = db.read_ir_doc("doc:README.md").unwrap().unwrap();
        assert_eq!(read.title, Some("Project Readme".into()));
        assert!(read.frontmatter.is_empty());
        assert!(read.status.is_none());
        assert!(read.sections.is_empty());
    }

    #[test]
    fn ir_doc_creates_traces_to_edge() {
        let db = GraphDb::open_memory().unwrap();

        let doc = crate::ir::IRDoc {
            base: crate::ir::IRBase {
                name: "Blueprint".into(),
                file: "docs/blueprints/01.md".into(),
                ..Default::default()
            },
            origin: Some("docs/ideas/01.md".into()),
            ..Default::default()
        };

        db.write_ir_doc(&doc, "test-proj").unwrap();

        // Check TRACES_TO edge exists
        let count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='doc:docs/blueprints/01.md' AND to_id='doc:docs/ideas/01.md' AND edge_type='TRACES_TO'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn ir_doc_creates_covers_edges() {
        let db = GraphDb::open_memory().unwrap();

        let doc = crate::ir::IRDoc {
            base: crate::ir::IRBase {
                name: "Design".into(),
                file: "docs/design/arch.md".into(),
                ..Default::default()
            },
            file_references: vec!["src/main.rs".into(), "src/lib.rs".into()],
            ..Default::default()
        };

        db.write_ir_doc(&doc, "test-proj").unwrap();

        let count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='doc:docs/design/arch.md' AND edge_type='COVERS'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn read_nonexistent_ir_doc() {
        let db = GraphDb::open_memory().unwrap();
        let result = db.read_ir_doc("doc:nonexistent.md").unwrap();
        assert!(result.is_none());
    }

    // ── IR Function write/read tests ────────────────────────────────

    #[test]
    fn write_and_read_ir_function() {
        let db = GraphDb::open_memory().unwrap();

        let func = crate::ir::IRFunction {
            base: crate::ir::IRBase {
                name: "parse_file".into(),
                file: "src/parser.rs".into(),
                line_start: 10,
                line_end: 50,
                language: Some("rust".into()),
                node_type: Some("function".into()),
                docstring: Some("Parse a source file into IR".into()),
                is_exported: true,
                tags: vec!["parser".into()],
                ..Default::default()
            },
            params: vec![
                crate::ir::IRParam { name: "path".into(), type_: Some("&Path".into()), ..Default::default() },
                crate::ir::IRParam { name: "content".into(), type_: Some("&str".into()), is_optional: false, default_value: None },
            ],
            return_type: Some("Result<IRParsedFile>".into()),
            is_async: false,
            decorators: vec!["#[instrument]".into()],
            calls: vec!["read_file".into(), "tokenize".into()],
            complexity: 8,
            body_hash: Some("abc123".into()),
        };

        db.write_ir_function(&func, "test-proj", None).unwrap();

        let read = db.read_ir_function("fn:test-proj:src/parser.rs:parse_file").unwrap().unwrap();
        assert_eq!(read.base.name, "parse_file");
        assert_eq!(read.params.len(), 2);
        assert_eq!(read.params[0].name, "path");
        assert_eq!(read.params[0].type_, Some("&Path".into()));
        assert_eq!(read.return_type, Some("Result<IRParsedFile>".into()));
        assert!(!read.is_async);
        assert_eq!(read.complexity, 8);
        assert_eq!(read.body_hash, Some("abc123".into()));
        assert_eq!(read.decorators, vec!["#[instrument]"]);
        assert_eq!(read.calls, vec!["read_file", "tokenize"]);
    }

    #[test]
    fn write_ir_function_async_minimal() {
        let db = GraphDb::open_memory().unwrap();

        let func = crate::ir::IRFunction {
            base: crate::ir::IRBase {
                name: "fetch_data".into(),
                file: "src/api.ts".into(),
                line_start: 1,
                line_end: 5,
                ..Default::default()
            },
            is_async: true,
            ..Default::default()
        };

        db.write_ir_function(&func, "proj", None).unwrap();

        let read = db.read_ir_function("fn:proj:src/api.ts:fetch_data").unwrap().unwrap();
        assert_eq!(read.base.name, "fetch_data");
        assert!(read.is_async);
        assert!(read.params.is_empty());
        assert!(read.return_type.is_none());
    }

    #[test]
    fn read_nonexistent_ir_function() {
        let db = GraphDb::open_memory().unwrap();
        let result = db.read_ir_function("fn:proj:missing.rs:nope").unwrap();
        assert!(result.is_none());
    }

    // ── IR Class write/read tests ───────────────────────────────────

    #[test]
    fn write_and_read_ir_class_with_implements() {
        let db = GraphDb::open_memory().unwrap();

        let class = crate::ir::IRClass {
            base: crate::ir::IRBase {
                name: "RustAdapter".into(),
                file: "src/adapters/rust_lang.rs".into(),
                line_start: 15,
                line_end: 200,
                language: Some("rust".into()),
                node_type: Some("struct".into()),
                category: Some("adapter".into()),
                docstring: Some("Adapter for Rust source files".into()),
                is_exported: true,
                ..Default::default()
            },
            class_kind: crate::ir::ClassKind::Struct,
            implements: vec!["LanguageAdapter".into()],
            extends: None,
            methods: vec![
                crate::ir::IRMethod {
                    base: crate::ir::IRBase {
                        name: "parse".into(),
                        file: "src/adapters/rust_lang.rs".into(),
                        line_start: 20,
                        line_end: 80,
                        ..Default::default()
                    },
                    params: vec![
                        crate::ir::IRParam { name: "source".into(), type_: Some("&str".into()), ..Default::default() },
                    ],
                    return_type: Some("ParsedFile".into()),
                    ..Default::default()
                },
            ],
            decorators: vec!["#[derive(Debug)]".into()],
            generic_params: vec![],
            ..Default::default()
        };

        db.write_ir_class(&class, "test-proj").unwrap();

        // Read back class
        let read = db.read_ir_class("class:test-proj:src/adapters/rust_lang.rs:RustAdapter").unwrap().unwrap();
        assert_eq!(read.base.name, "RustAdapter");
        assert_eq!(read.class_kind, crate::ir::ClassKind::Struct);
        assert_eq!(read.implements, vec!["LanguageAdapter"]);
        assert!(read.extends.is_none());
        assert_eq!(read.decorators, vec!["#[derive(Debug)]"]);

        // Check IMPLEMENTS edge was created
        let count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='class:test-proj:src/adapters/rust_lang.rs:RustAdapter' AND edge_type='IMPLEMENTS'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);

        // Acceptance: no extends → zero EXTENDS edges
        let extends_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='class:test-proj:src/adapters/rust_lang.rs:RustAdapter' AND edge_type='EXTENDS'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(extends_count, 0, "class with no extends should have zero EXTENDS edges");
    }

    #[test]
    fn write_ir_class_with_extends() {
        let db = GraphDb::open_memory().unwrap();

        let class = crate::ir::IRClass {
            base: crate::ir::IRBase {
                name: "Dog".into(),
                file: "models.py".into(),
                line_start: 10,
                line_end: 30,
                ..Default::default()
            },
            class_kind: crate::ir::ClassKind::Class,
            extends: Some("Animal".into()),
            implements: vec!["Serializable".into(), "Comparable".into()],
            ..Default::default()
        };

        db.write_ir_class(&class, "proj").unwrap();

        // Check EXTENDS edge
        let extends_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='class:proj:models.py:Dog' AND edge_type='EXTENDS'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(extends_count, 1);

        // Check IMPLEMENTS edges (one per interface)
        let impl_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='class:proj:models.py:Dog' AND edge_type='IMPLEMENTS'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(impl_count, 2);

        // Read back and verify
        let read = db.read_ir_class("class:proj:models.py:Dog").unwrap().unwrap();
        assert_eq!(read.extends, Some("Animal".into()));
        assert_eq!(read.implements.len(), 2);
    }

    #[test]
    fn write_ir_class_creates_method_nodes_and_has_method_edges() {
        let db = GraphDb::open_memory().unwrap();

        let class = crate::ir::IRClass {
            base: crate::ir::IRBase {
                name: "UserService".into(),
                file: "src/services/user.ts".into(),
                line_start: 1,
                line_end: 100,
                ..Default::default()
            },
            class_kind: crate::ir::ClassKind::Class,
            methods: vec![
                crate::ir::IRMethod {
                    base: crate::ir::IRBase {
                        name: "findById".into(),
                        file: "src/services/user.ts".into(),
                        line_start: 5,
                        line_end: 20,
                        ..Default::default()
                    },
                    params: vec![
                        crate::ir::IRParam { name: "id".into(), type_: Some("string".into()), ..Default::default() },
                    ],
                    return_type: Some("Promise<User>".into()),
                    is_async: true,
                    ..Default::default()
                },
                crate::ir::IRMethod {
                    base: crate::ir::IRBase {
                        name: "create".into(),
                        file: "src/services/user.ts".into(),
                        line_start: 22,
                        line_end: 40,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        db.write_ir_class(&class, "proj").unwrap();

        // Check HAS_METHOD edges
        let method_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='class:proj:src/services/user.ts:UserService' AND edge_type='HAS_METHOD'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(method_count, 2);

        // Check method nodes exist in ir_functions
        let fn_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM ir_functions WHERE node_id LIKE 'fn:proj:src/services/user.ts:%'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(fn_count, 2);
    }

    #[test]
    fn write_ir_class_with_mixins_and_generics() {
        let db = GraphDb::open_memory().unwrap();

        let class = crate::ir::IRClass {
            base: crate::ir::IRBase {
                name: "Repository".into(),
                file: "src/repo.ts".into(),
                line_start: 1,
                line_end: 50,
                ..Default::default()
            },
            class_kind: crate::ir::ClassKind::Class,
            generic_params: vec!["T".into(), "ID".into()],
            mixins: vec!["Cacheable".into(), "Loggable".into()],
            ..Default::default()
        };

        db.write_ir_class(&class, "proj").unwrap();

        let read = db.read_ir_class("class:proj:src/repo.ts:Repository").unwrap().unwrap();
        assert_eq!(read.generic_params, vec!["T", "ID"]);
        assert_eq!(read.mixins, vec!["Cacheable", "Loggable"]);

        // Acceptance: no implements/extends → zero hierarchy edges
        let edge_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='class:proj:src/repo.ts:Repository' AND edge_type IN ('IMPLEMENTS','EXTENDS')",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(edge_count, 0, "class with no implements/extends should create zero hierarchy edges");
    }

    #[test]
    fn read_nonexistent_ir_class() {
        let db = GraphDb::open_memory().unwrap();
        let result = db.read_ir_class("class:proj:missing.rs:Nope").unwrap();
        assert!(result.is_none());
    }
}
