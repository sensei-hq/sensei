use super::GraphDb;
use crate::types::{NodeKind, HierarchyNode};
#[cfg(test)]
use rusqlite::params;

impl GraphDb {
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
        use rusqlite::OptionalExtension;

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
        use rusqlite::OptionalExtension;

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
        use rusqlite::OptionalExtension;

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
}

#[cfg(test)]
mod tests {
    use super::super::GraphDb;

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

        // Acceptance: no extends -> zero EXTENDS edges
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

        // Acceptance: no implements/extends -> zero hierarchy edges
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
