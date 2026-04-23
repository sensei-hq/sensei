use super::GraphDb;
use rusqlite::params;

#[cfg(test)]
use crate::types::{NodeKind, HierarchyNode};

impl GraphDb {
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
        use rusqlite::OptionalExtension;
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
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::super::GraphDb;
    use crate::types::{NodeKind, HierarchyNode};

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
