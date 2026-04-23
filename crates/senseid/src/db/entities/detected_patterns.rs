use rusqlite::params;
use super::super::Store;

impl Store {
    /// Detect patterns by naming convention from hierarchy_nodes in the graph.
    pub fn detect_patterns_from_graph(&self, graph: &rusqlite::Connection, project: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
        let suffixes = [
            ("Adapter", "adapter"), ("Factory", "factory"), ("Observer", "observer"),
            ("Builder", "builder"), ("Strategy", "strategy"), ("Handler", "handler"),
            ("Middleware", "middleware"), ("Provider", "provider"), ("Decorator", "decorator"),
            ("Worker", "worker"), ("Hook", "hook"), ("Plugin", "plugin"),
            ("Controller", "controller"), ("Service", "service"), ("Repository", "repository"),
        ];

        self.conn.execute("DELETE FROM detected_patterns WHERE project=?1", params![project])?;

        let mut results = Vec::new();

        for (suffix, pattern_type) in &suffixes {
            let like_pattern = format!("%{}", suffix);
            let mut stmt = graph.prepare(
                "SELECT name, file, kind FROM hierarchy_nodes WHERE project=?1 AND name LIKE ?2 AND kind IN ('class','struct','interface','type','component')"
            ).map_err(|_e| rusqlite::Error::QueryReturnedNoRows)?;

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

    pub fn match_pattern(&self, project: &str, description: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
        let desc_lower = description.to_lowercase();
        let patterns = self.list_detected_patterns(project)?;

        let mut matches: Vec<serde_json::Value> = patterns.into_iter().filter(|p| {
            let ptype = p["pattern_type"].as_str().unwrap_or("");
            let pname = p["name"].as_str().unwrap_or("");
            desc_lower.contains(ptype)
                || p["instances"].as_array().is_some_and(|insts| {
                    insts.iter().any(|i| {
                        let iname = i["name"].as_str().unwrap_or("").to_lowercase();
                        desc_lower.contains(&iname) || iname.contains(&desc_lower)
                    })
                })
                || desc_lower.contains(&pname.to_lowercase())
        }).collect();

        if matches.is_empty() && !desc_lower.is_empty() {
            matches = self.list_detected_patterns(project)?;
            for m in &mut matches {
                m.as_object_mut().map(|o| o.insert("match_type".into(), serde_json::json!("context")));
            }
        }

        Ok(matches)
    }

    /// Find genuine code duplicates.
    pub fn find_duplicates(&self, graph: &rusqlite::Connection, project: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
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

        let patterns = self.list_detected_patterns(project).unwrap_or_default();
        let pattern_instance_names: Vec<String> = patterns.iter()
            .flat_map(|p| {
                p["instances"].as_array().unwrap_or(&vec![]).iter()
                    .filter_map(|i| i["name"].as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut results = Vec::new();

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
                let a_lower = name_a.to_lowercase();
                let b_lower = name_b.to_lowercase();
                !noise_names.contains(&a_lower.as_str()) && !noise_names.contains(&b_lower.as_str())
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

    /// Analyze project conventions.
    pub fn get_project_conventions(&self, graph: &rusqlite::Connection, project: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
        let mut conventions = Vec::new();

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

    /// Compute aggregate metrics for a project from sessions + events.
    pub fn compute_metrics(&self, project: &str) -> rusqlite::Result<serde_json::Value> {
        let sessions = self.get_sessions(Some(project))?;
        let session_count = sessions.len() as u64;
        let ftr: Option<f64> = if sessions.is_empty() {
            None
        } else {
            let sum: f64 = sessions.iter().filter_map(|s| s["ftr"].as_f64()).sum();
            let count = sessions.iter().filter(|s| s["ftr"].as_f64().is_some()).count();
            if count > 0 { Some(sum / count as f64) } else { None }
        };

        let turn_count = self.count_events(project, Some("turn"))?;
        let revision_count = self.count_events(project, Some("revision_requested"))?;
        let rework_rate = if turn_count > 0 {
            revision_count as f64 / turn_count as f64
        } else {
            0.0
        };

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

    fn test_store() -> Store { Store::open_memory().unwrap() }

    fn graph_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE hierarchy_nodes(id TEXT PRIMARY KEY, name TEXT, kind TEXT, level TEXT, parent_id TEXT, file TEXT, line INTEGER, project TEXT, sig TEXT, body TEXT, docstring TEXT, complexity INTEGER, tags TEXT, doc_type TEXT, doc_category TEXT);
        ").unwrap();
        conn
    }

    #[test]
    fn detect_patterns_by_naming() {
        let s = test_store();
        let g = graph_conn();
        for (name, file) in [("TypeScriptAdapter", "ts.rs"), ("PythonAdapter", "py.rs"), ("RustAdapter", "rust.rs")] {
            g.execute("INSERT INTO hierarchy_nodes(id, name, kind, project, file) VALUES(?1,?2,'class','test',?3)",
                rusqlite::params![format!("type:{}", name), name, file]).unwrap();
        }
        let patterns = s.detect_patterns_from_graph(&g, "test").unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0]["pattern_type"], "adapter");
        assert_eq!(patterns[0]["instance_count"], 3);
    }

    #[test]
    fn detect_requires_two_instances() {
        let s = test_store();
        let g = graph_conn();
        g.execute("INSERT INTO hierarchy_nodes(id, name, kind, project) VALUES('type:SqlAdapter','SqlAdapter','class','test')", []).unwrap();
        assert!(s.detect_patterns_from_graph(&g, "test").unwrap().is_empty());
    }

    #[test]
    fn list_detected_patterns() {
        let s = test_store();
        let g = graph_conn();
        for name in ["FooHandler", "BarHandler", "FooWorker", "BarWorker"] {
            g.execute("INSERT INTO hierarchy_nodes(id, name, kind, project) VALUES(?1,?2,'class','test')",
                rusqlite::params![format!("type:{}", name), name]).unwrap();
        }
        s.detect_patterns_from_graph(&g, "test").unwrap();
        assert_eq!(s.list_detected_patterns("test").unwrap().len(), 2);
    }

    #[test]
    fn match_pattern_by_keyword() {
        let s = test_store();
        let g = graph_conn();
        for name in ["TypeScriptAdapter", "PythonAdapter", "RustAdapter"] {
            g.execute("INSERT INTO hierarchy_nodes(id, name, kind, project) VALUES(?1,?2,'class','test')",
                rusqlite::params![format!("type:{}", name), name]).unwrap();
        }
        s.detect_patterns_from_graph(&g, "test").unwrap();
        let matches = s.match_pattern("test", "add SQL adapter").unwrap();
        assert!(!matches.is_empty());
        assert_eq!(matches[0]["pattern_type"], "adapter");
    }

    #[test]
    fn match_pattern_returns_context_on_no_match() {
        let s = test_store();
        let g = graph_conn();
        for name in ["FooHandler", "BarHandler"] {
            g.execute("INSERT INTO hierarchy_nodes(id, name, kind, project) VALUES(?1,?2,'class','test')",
                rusqlite::params![format!("type:{}", name), name]).unwrap();
        }
        s.detect_patterns_from_graph(&g, "test").unwrap();
        let matches = s.match_pattern("test", "add logging").unwrap();
        assert!(!matches.is_empty());
        assert_eq!(matches[0]["match_type"], "context");
    }

    #[test]
    fn get_pattern_for_symbol() {
        let s = test_store();
        let g = graph_conn();
        for name in ["FooAdapter", "BarAdapter"] {
            g.execute("INSERT INTO hierarchy_nodes(id, name, kind, project) VALUES(?1,?2,'class','test')",
                rusqlite::params![format!("type:{}", name), name]).unwrap();
        }
        s.detect_patterns_from_graph(&g, "test").unwrap();
        let p = s.get_pattern_for("test", "FooAdapter").unwrap();
        assert!(p.is_some());
        assert_eq!(p.unwrap()["pattern_type"], "adapter");
        assert!(s.get_pattern_for("test", "Unknown").unwrap().is_none());
    }

    #[test]
    fn metrics_empty_project() {
        let s = test_store();
        let m = s.compute_metrics("nonexistent").unwrap();
        assert_eq!(m["session_count"], 0);
        assert_eq!(m["turn_count"], 0);
        assert!(m["ftr"].is_null());
    }

    #[test]
    fn metrics_ftr_from_sessions() {
        let s = test_store();
        s.create_session("s1", "proj", "t1").unwrap();
        s.update_session("s1", Some("completed"), None, None, None, None).unwrap();
        s.create_session("s2", "proj", "t2").unwrap();
        s.update_session("s2", Some("completed"), None, None, None, None).unwrap();
        s.create_session("s3", "proj", "t3").unwrap();
        s.update_session("s3", Some("partial"), None, None, None, None).unwrap();

        let m = s.compute_metrics("proj").unwrap();
        assert_eq!(m["session_count"], 3);
        let ftr = m["ftr"].as_f64().unwrap();
        assert!((ftr - 0.833).abs() < 0.01);
    }

    #[test]
    fn metrics_rework_rate() {
        let s = test_store();
        for i in 0..5 { s.insert_event(&format!("t{}", i), "proj", None, "turn", "{}").unwrap(); }
        for i in 0..2 { s.insert_event(&format!("r{}", i), "proj", None, "revision_requested", "{}").unwrap(); }
        let m = s.compute_metrics("proj").unwrap();
        let rr = m["rework_rate"].as_f64().unwrap();
        assert!((rr - 0.4).abs() < 0.01);
    }

    #[test]
    fn metrics_tool_adherence() {
        let s = test_store();
        s.insert_event("t1", "proj", None, "tool_used", r#"{"tool":"search","is_mcp":true}"#).unwrap();
        s.insert_event("t2", "proj", None, "tool_used", r#"{"tool":"grep","is_mcp":false}"#).unwrap();
        s.insert_event("t3", "proj", None, "tool_used", r#"{"tool":"get_callers","is_mcp":true}"#).unwrap();
        s.insert_event("t4", "proj", None, "tool_used", r#"{"tool":"get_patterns","is_mcp":true}"#).unwrap();
        let m = s.compute_metrics("proj").unwrap();
        let ta = m["tool_adherence"].as_f64().unwrap();
        assert!((ta - 0.75).abs() < 0.01);
    }
}
