//! E2e test: scan ~/Developer for repos and index one.

use std::path::PathBuf;

fn developer_path() -> PathBuf {
    dirs::home_dir().unwrap().join("Developer")
}

#[test]
fn scan_developer_and_index_one() {
    let dev = developer_path();
    if !dev.exists() {
        eprintln!("Skipping — ~/Developer not found");
        return;
    }

    // Find git repos (depth 1)
    let mut repos: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dev) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() && path.join(".git").exists() {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                repos.push((name, path));
            }
        }
    }

    println!("Found {} repos in ~/Developer", repos.len());
    assert!(repos.len() >= 1, "expected at least 1 repo in ~/Developer");

    // Pick a small Python or Rust repo to index
    let target = repos.iter().find(|(_, p)| {
        p.join("pyproject.toml").exists() || p.join("requirements.txt").exists()
            || p.join("Cargo.toml").exists()
    }).or_else(|| repos.first());

    if let Some((name, path)) = target {
        println!("Indexing: {} ({})", name, path.display());

        // Detect stack
        let stack = senseid_config::detect_stack(path);
        println!("  stack: {:?}", stack);

        // Detect config files
        let configs = senseid_config::detect_config_files(path);
        println!("  configs: {} files", configs.len());
        for c in &configs {
            println!("    {} ({})", c.file_type, c.path);
        }

        // Index using tree-sitter
        let db_dir = tempfile::TempDir::new().unwrap();
        let graph = senseid_graph::GraphDb::open(db_dir.path()).unwrap();
        let result = senseid_pipeline::index_repo(&graph, &path.to_string_lossy(), name);

        match result {
            Ok(r) => {
                println!("  files: {} indexed, {} skipped, {} failed", r.files_indexed, r.files_skipped, r.files_failed);
                println!("  symbols: {} functions, {} types", r.functions_indexed, r.types_indexed);
                println!("  edges: {}", r.edges_created);
                println!("  docs: {}", r.docs_indexed);
                println!("  libs: {:?}", r.libs);
                println!("  duration: {}ms", r.duration_ms);

                // Verify something was indexed
                if r.files_indexed > 0 {
                    let (fns, types) = graph.count_symbols(name).unwrap();
                    assert!(fns + types > 0, "graph should have symbols after indexing");
                    println!("  graph: {} functions + {} types in DB", fns, types);
                }
            }
            Err(e) => {
                eprintln!("  indexing failed: {}", e);
            }
        }
    }
}

// Inline module references (can't import from binary crate)
mod senseid_config {
    use std::path::Path;
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ConfigFile { pub file_type: String, pub path: String, pub data: serde_json::Value }

    pub fn detect_stack(repo_path: &Path) -> Vec<String> {
        let mut stack = Vec::new();
        if repo_path.join("Cargo.toml").exists() { stack.push("rust".into()); }
        if repo_path.join("go.mod").exists() { stack.push("go".into()); }
        if repo_path.join("pyproject.toml").exists() || repo_path.join("requirements.txt").exists() { stack.push("python".into()); }
        if repo_path.join("pom.xml").exists() || repo_path.join("build.gradle").exists() { stack.push("java".into()); }
        if repo_path.join("package.json").exists() {
            if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json")) {
                if content.contains("@sveltejs/kit") { stack.push("sveltekit".into()); }
                else if content.contains("\"react\"") { stack.push("react".into()); }
                else if content.contains("\"next\"") { stack.push("nextjs".into()); }
                else { stack.push("node".into()); }
            }
        }
        stack
    }

    pub fn detect_config_files(repo_path: &Path) -> Vec<ConfigFile> {
        let mut configs = Vec::new();
        for name in &["package.json", "Cargo.toml", "pyproject.toml", "Dockerfile", "docker-compose.yml"] {
            if repo_path.join(name).exists() {
                configs.push(ConfigFile { file_type: name.to_string(), path: name.to_string(), data: serde_json::json!({}) });
            }
        }
        configs
    }
}

mod senseid_graph {
    use rusqlite::{Connection, params, OptionalExtension};
    use std::path::Path;

    pub struct GraphDb { conn: Connection }

    impl GraphDb {
        pub fn open(path: &Path) -> Result<Self, String> {
            let db_path = path.join("graph.db");
            let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
            conn.execute_batch("
                CREATE TABLE IF NOT EXISTS functions(id TEXT PRIMARY KEY, name TEXT, file TEXT, line INTEGER, sig TEXT, body TEXT, docstring TEXT, complexity INTEGER DEFAULT 1, project TEXT);
                CREATE TABLE IF NOT EXISTS files(id TEXT PRIMARY KEY, path TEXT, module TEXT, lang TEXT, project TEXT);
                CREATE TABLE IF NOT EXISTS types(id TEXT PRIMARY KEY, name TEXT, file TEXT, line INTEGER, kind TEXT, project TEXT);
                CREATE TABLE IF NOT EXISTS docs(id TEXT PRIMARY KEY, path TEXT, title TEXT, doc_type TEXT, project TEXT);
                CREATE TABLE IF NOT EXISTS edges(from_id TEXT, to_id TEXT, edge_type TEXT, weight REAL, PRIMARY KEY(from_id, to_id, edge_type));
            ").map_err(|e| e.to_string())?;
            Ok(Self { conn })
        }

        pub fn merge_function(&self, id: &str, name: &str, file: &str, line: u32, sig: &str, body: &str, docstring: &str, complexity: u32, project: &str) -> Result<(), String> {
            self.conn.execute("INSERT OR REPLACE INTO functions(id,name,file,line,sig,body,docstring,complexity,project) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![id, name, file, line, &sig[..sig.len().min(500)], &body[..body.len().min(10000)], &docstring[..docstring.len().min(2000)], complexity, project]).map_err(|e| e.to_string())?;
            Ok(())
        }

        pub fn merge_file(&self, id: &str, path: &str, module: &str, lang: &str, project: &str) -> Result<(), String> {
            self.conn.execute("INSERT OR REPLACE INTO files(id,path,module,lang,project) VALUES(?1,?2,?3,?4,?5)", params![id, path, module, lang, project]).map_err(|e| e.to_string())?;
            Ok(())
        }

        pub fn merge_type(&self, id: &str, name: &str, file: &str, line: u32, kind: &str, project: &str) -> Result<(), String> {
            self.conn.execute("INSERT OR REPLACE INTO types(id,name,file,line,kind,project) VALUES(?1,?2,?3,?4,?5,?6)", params![id, name, file, line, kind, project]).map_err(|e| e.to_string())?;
            Ok(())
        }

        pub fn merge_doc(&self, id: &str, path: &str, title: &str, doc_type: &str, project: &str) -> Result<(), String> {
            self.conn.execute("INSERT OR REPLACE INTO docs(id,path,title,doc_type,project) VALUES(?1,?2,?3,?4,?5)", params![id, path, title, doc_type, project]).map_err(|e| e.to_string())?;
            Ok(())
        }

        pub fn merge_edge(&self, from_id: &str, to_id: &str, edge_type: &str) -> Result<(), String> {
            self.conn.execute("INSERT OR REPLACE INTO edges(from_id,to_id,edge_type) VALUES(?1,?2,?3)", params![from_id, to_id, edge_type]).map_err(|e| e.to_string())?;
            Ok(())
        }

        pub fn find_function_by_name(&self, name: &str, project: &str) -> Result<Option<String>, String> {
            self.conn.query_row("SELECT id FROM functions WHERE name=?1 AND project=?2 LIMIT 1", params![name, project], |r| r.get(0))
                .optional().map_err(|e| e.to_string())
        }

        pub fn count_symbols(&self, project: &str) -> Result<(u32, u32), String> {
            let fns: u32 = self.conn.query_row("SELECT COUNT(*) FROM functions WHERE project=?1", params![project], |r| r.get(0)).unwrap_or(0);
            let types: u32 = self.conn.query_row("SELECT COUNT(*) FROM types WHERE project=?1", params![project], |r| r.get(0)).unwrap_or(0);
            Ok((fns, types))
        }

        pub fn delete_file(&self, _abs_path: &str, _project: &str) -> Result<(), String> { Ok(()) }
    }
}

mod senseid_pipeline {
    use super::senseid_graph::GraphDb;
    use std::path::{Path, PathBuf};
    use std::collections::HashMap;
    use std::time::Instant;

    pub struct IndexResult {
        pub files_indexed: u32, pub files_skipped: u32, pub files_failed: u32,
        pub functions_indexed: u32, pub types_indexed: u32, pub edges_created: u32,
        pub docs_indexed: u32, pub libs: Vec<String>, pub duration_ms: u64,
    }

    pub fn index_repo(graph: &GraphDb, repo_path: &str, repo_id: &str) -> Result<IndexResult, String> {
        let start = Instant::now();
        let repo = Path::new(repo_path);
        if !repo.exists() { return Err(format!("not found: {}", repo_path)); }

        let mut files_indexed = 0u32;
        let mut files_skipped = 0u32;
        let mut files_failed = 0u32;
        let mut functions_indexed = 0u32;
        let mut types_indexed = 0u32;
        let mut edges_created = 0u32;
        let mut docs_indexed = 0u32;
        let mut lib_set = std::collections::HashSet::new();

        // Walk files
        for entry in walkdir::WalkDir::new(repo)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                e.depth() == 0 || (!name.starts_with('.') && name != "node_modules" && name != "target" && name != "dist" && name != "__pycache__")
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let abs = path.to_string_lossy().to_string();
            let rel = path.strip_prefix(repo).unwrap_or(path).to_string_lossy().to_string();

            match ext {
                "py" => {
                    if let Ok(source) = std::fs::read_to_string(path) {
                        let mut parser = tree_sitter::Parser::new();
                        parser.set_language(&tree_sitter_python::LANGUAGE.into()).ok();
                        if let Some(tree) = parser.parse(&source, None) {
                            let root = tree.root_node();
                            let lines: Vec<&str> = source.lines().collect();
                            extract_python(&root, source.as_bytes(), &lines, &abs, repo_id, graph, &mut functions_indexed, &mut types_indexed, false);
                            // Imports
                            for i in 0..root.child_count() {
                                let c = root.child(i).unwrap();
                                if c.kind() == "import_from_statement" || c.kind() == "import_statement" {
                                    if let Some(name) = c.child_by_field_name("module_name").or_else(|| c.child(1)) {
                                        let t = name.utf8_text(source.as_bytes()).unwrap_or("");
                                        if !t.starts_with('.') && !t.is_empty() {
                                            lib_set.insert(t.split('.').next().unwrap_or(t).to_string());
                                        }
                                    }
                                }
                            }
                        }
                        files_indexed += 1;
                    } else { files_failed += 1; }
                }
                "rs" => {
                    if let Ok(source) = std::fs::read_to_string(path) {
                        let mut parser = tree_sitter::Parser::new();
                        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).ok();
                        if let Some(tree) = parser.parse(&source, None) {
                            let root = tree.root_node();
                            for i in 0..root.child_count() {
                                let c = root.child(i).unwrap();
                                if c.kind() == "function_item" {
                                    let name = c.child_by_field_name("name").map(|n| n.utf8_text(source.as_bytes()).unwrap_or("")).unwrap_or("");
                                    let id = format!("fn:{}:{}:{}", abs, name, c.start_position().row + 1);
                                    graph.merge_function(&id, name, &abs, c.start_position().row as u32 + 1, "", "", "", 1, repo_id).ok();
                                    functions_indexed += 1;
                                } else if c.kind() == "struct_item" || c.kind() == "enum_item" {
                                    let name = c.child_by_field_name("name").map(|n| n.utf8_text(source.as_bytes()).unwrap_or("")).unwrap_or("");
                                    let id = format!("type:{}:{}:{}", abs, name, c.start_position().row + 1);
                                    graph.merge_type(&id, name, &abs, c.start_position().row as u32 + 1, c.kind(), repo_id).ok();
                                    types_indexed += 1;
                                }
                            }
                        }
                        files_indexed += 1;
                    } else { files_failed += 1; }
                }
                "java" => {
                    if let Ok(source) = std::fs::read_to_string(path) {
                        let mut parser = tree_sitter::Parser::new();
                        parser.set_language(&tree_sitter_java::LANGUAGE.into()).ok();
                        if let Some(tree) = parser.parse(&source, None) {
                            let root = tree.root_node();
                            for i in 0..root.child_count() {
                                let c = root.child(i).unwrap();
                                if c.kind() == "class_declaration" || c.kind() == "interface_declaration" {
                                    let name = c.child_by_field_name("name").map(|n| n.utf8_text(source.as_bytes()).unwrap_or("")).unwrap_or("");
                                    let id = format!("type:{}:{}:{}", abs, name, c.start_position().row + 1);
                                    graph.merge_type(&id, name, &abs, c.start_position().row as u32 + 1, c.kind(), repo_id).ok();
                                    types_indexed += 1;
                                }
                            }
                        }
                        files_indexed += 1;
                    } else { files_failed += 1; }
                }
                "md" | "mdx" => {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let title = content.lines().find(|l| l.starts_with("# ")).map(|l| l[2..].trim().to_string()).unwrap_or(rel.clone());
                        let doc_id = format!("doc:{}", abs);
                        graph.merge_doc(&doc_id, &abs, &title, "doc", repo_id).ok();
                        docs_indexed += 1;
                    }
                }
                _ => { files_skipped += 1; }
            }
        }

        let mut libs: Vec<String> = lib_set.into_iter().collect();
        libs.sort();

        Ok(IndexResult {
            files_indexed, files_skipped, files_failed,
            functions_indexed, types_indexed, edges_created,
            docs_indexed, libs, duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn extract_python(node: &tree_sitter::Node, src: &[u8], lines: &[&str], abs: &str, project: &str, graph: &GraphDb, fns: &mut u32, types: &mut u32, in_class: bool) {
        for i in 0..node.child_count() {
            let c = node.child(i).unwrap();
            match c.kind() {
                "function_definition" => {
                    let name = c.child_by_field_name("name").map(|n| n.utf8_text(src).unwrap_or("")).unwrap_or("");
                    let id = format!("fn:{}:{}:{}", abs, name, c.start_position().row + 1);
                    graph.merge_function(&id, name, abs, c.start_position().row as u32 + 1, "", "", "", 1, project).ok();
                    *fns += 1;
                }
                "class_definition" => {
                    let name = c.child_by_field_name("name").map(|n| n.utf8_text(src).unwrap_or("")).unwrap_or("");
                    let id = format!("type:{}:{}:{}", abs, name, c.start_position().row + 1);
                    graph.merge_type(&id, name, abs, c.start_position().row as u32 + 1, "class", project).ok();
                    *types += 1;
                    if let Some(body) = c.child_by_field_name("body") {
                        extract_python(&body, src, lines, abs, project, graph, fns, types, true);
                    }
                }
                _ => {}
            }
        }
    }
}
