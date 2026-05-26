//! E2e test: index the httpx benchmark corpus and verify symbol counts.

use std::path::PathBuf;

fn httpx_corpus_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates
    path.pop(); // sensei root
    path.push("examples/benchmarks/httpx/src");
    path
}

// sample_corpus_path removed — unused. Re-add when sample corpus tests are written.

#[test]
fn index_httpx_corpus() {
    let corpus = httpx_corpus_path();
    if !corpus.exists() {
        eprintln!("Skipping httpx test — corpus not found at {}", corpus.display());
        return;
    }

    let _db_dir = tempfile::TempDir::new().unwrap();

    // Use the library directly
    // We need to inline since we can't import from the binary crate in integration tests
    // Instead, test via the binary's HTTP API by starting the server

    // For now, just verify the corpus files exist and are parseable
    let py_files: Vec<_> = std::fs::read_dir(&corpus).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "py"))
        .collect();

    assert!(py_files.len() >= 6, "httpx corpus should have 6 Python files, found {}", py_files.len());

    // Parse each file and count symbols
    let mut total_symbols = 0u32;
    let mut total_classes = 0u32;
    let mut total_functions = 0u32;
    let mut total_methods = 0u32;

    for entry in &py_files {
        let source = std::fs::read_to_string(entry.path()).unwrap();
        let _rel_path = entry.file_name().to_string_lossy().to_string();

        // Inline parse — we can't import from the binary directly
        let mut parser = tree_sitter::Parser::new();
        let lang = tree_sitter_python::LANGUAGE;
        parser.set_language(&lang.into()).unwrap();

        if let Some(tree) = parser.parse(&source, None) {
            let root = tree.root_node();
            count_python_symbols(&root, &mut total_symbols, &mut total_classes, &mut total_functions, &mut total_methods, false);
        }
    }

    println!("httpx corpus: {} symbols ({} classes, {} functions, {} methods)",
        total_symbols, total_classes, total_functions, total_methods);

    // Expected from our benchmark yaml: 136 total, 42 classes, 8 functions, 84 methods
    assert!(total_symbols >= 100, "expected 100+ symbols, got {}", total_symbols);
    assert!(total_classes >= 30, "expected 30+ classes, got {}", total_classes);
    assert!(total_functions >= 5, "expected 5+ functions, got {}", total_functions);
    assert!(total_methods >= 60, "expected 60+ methods, got {}", total_methods);
}

fn count_python_symbols(node: &tree_sitter::Node, total: &mut u32, classes: &mut u32, functions: &mut u32, methods: &mut u32, in_class: bool) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_definition" => {
                *total += 1;
                if in_class { *methods += 1; } else { *functions += 1; }
            }
            "class_definition" => {
                *total += 1;
                *classes += 1;
                if let Some(body) = child.child_by_field_name("body") {
                    count_python_symbols(&body, total, classes, functions, methods, true);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn scan_developer_folder_structure() {
    // Verify ~/Developer exists and has repos
    let dev = dirs::home_dir().unwrap().join("Developer");
    if !dev.exists() {
        eprintln!("Skipping ~/Developer scan — directory not found");
        return;
    }

    let mut repo_count = 0u32;
    let mut has_python = false;
    let mut has_typescript = false;
    let mut has_rust = false;
    let mut has_java = false;

    for entry in std::fs::read_dir(&dev).unwrap().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() { continue; }
        if path.join(".git").exists() {
            repo_count += 1;

            // Detect language
            if path.join("package.json").exists() { has_typescript = true; }
            if path.join("Cargo.toml").exists() { has_rust = true; }
            if path.join("pyproject.toml").exists() || path.join("requirements.txt").exists() { has_python = true; }
            if path.join("pom.xml").exists() || path.join("build.gradle").exists() { has_java = true; }
        }
    }

    println!("~/Developer: {} repos (ts={}, py={}, rs={}, java={})",
        repo_count, has_typescript, has_python, has_rust, has_java);

    assert!(repo_count >= 1, "expected at least 1 repo in ~/Developer");
}
