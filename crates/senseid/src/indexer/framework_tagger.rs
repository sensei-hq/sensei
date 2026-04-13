use std::collections::HashMap;
use super::graph::GraphDb;

/// Known framework/lib import patterns → tags
const FRAMEWORK_RULES: &[(&str, &str)] = &[
    // React ecosystem
    ("react", "react"),
    ("react-dom", "react"),
    ("next", "nextjs,react"),
    ("@remix-run", "remix,react"),
    ("gatsby", "gatsby,react"),
    // Vue ecosystem
    ("vue", "vue"),
    ("nuxt", "nuxt,vue"),
    ("pinia", "vue,state"),
    // Svelte ecosystem
    ("svelte", "svelte"),
    ("@sveltejs/kit", "sveltekit,svelte"),
    // Angular
    ("@angular/core", "angular"),
    ("@angular/common", "angular"),
    ("@angular/router", "angular"),
    // Node.js HTTP frameworks
    ("express", "express,http"),
    ("hono", "hono,http"),
    ("fastify", "fastify,http"),
    ("koa", "koa,http"),
    ("@nestjs/core", "nestjs,http"),
    // Python frameworks
    ("flask", "flask,http"),
    ("django", "django,http"),
    ("fastapi", "fastapi,http"),
    // Rust frameworks
    ("axum", "axum,http"),
    ("actix_web", "actix,http"),
    ("rocket", "rocket,http"),
    // Swift frameworks
    ("UIKit", "uikit,ios"),
    ("SwiftUI", "swiftui,ios"),
    ("Foundation", "foundation"),
    ("Vapor", "vapor,http"),
    // Kotlin frameworks
    ("io.ktor", "ktor,http"),
    ("org.springframework", "spring,http"),
    ("android", "android"),
    ("androidx", "android"),
    // Testing
    ("vitest", "testing"),
    ("jest", "testing"),
    ("pytest", "testing"),
    ("playwright", "testing,e2e"),
    ("cypress", "testing,e2e"),
    // State management
    ("redux", "state,redux"),
    ("zustand", "state"),
    ("mobx", "state"),
    // ORM / DB
    ("prisma", "orm,database"),
    ("drizzle-orm", "orm,database"),
    ("typeorm", "orm,database"),
    ("sqlalchemy", "orm,database"),
    ("diesel", "orm,database"),
    // Auth
    ("@auth", "auth"),
    ("next-auth", "auth"),
    ("passport", "auth"),
    // MCP / AI
    ("@modelcontextprotocol", "mcp,ai"),
    ("openai", "ai"),
    ("anthropic", "ai"),
    ("langchain", "ai"),
];

/// Pattern-based function tagging
const FUNCTION_PATTERNS: &[(&str, &str)] = &[
    // React patterns
    ("use", "hook"),          // useEffect, useState, useCustomHook
    ("render", "render"),
    ("handleClick", "handler"),
    ("handle", "handler"),
    ("on", "handler"),        // onClick, onSubmit
    // Middleware / route patterns
    ("middleware", "middleware"),
    ("route", "route"),
    ("controller", "controller"),
    ("handler", "handler"),
    // API patterns
    ("fetch", "api"),
    ("api", "api"),
    ("get", "api"),
    ("post", "api"),
    ("put", "api"),
    ("delete", "api"),
    // Testing patterns
    ("test", "test"),
    ("spec", "test"),
    ("describe", "test"),
    ("it", "test"),
];

/// Tag files based on their imports. Returns count of files tagged.
pub fn tag_files_by_imports(
    graph_db: &GraphDb,
    file_imports: &HashMap<String, Vec<String>>, // file_id -> [import_targets]
) -> u32 {
    let mut count = 0u32;
    for (file_id, imports) in file_imports {
        let mut tags = Vec::new();
        for import in imports {
            for (pattern, tag) in FRAMEWORK_RULES {
                if import.starts_with(pattern) || import.contains(pattern) {
                    for t in tag.split(',') {
                        if !tags.contains(&t.to_string()) {
                            tags.push(t.to_string());
                        }
                    }
                }
            }
        }
        if !tags.is_empty() {
            tags.sort();
            tags.dedup();
            graph_db.tag_file(file_id, &tags.join(",")).ok();
            count += 1;
        }
    }
    count
}

/// Tag functions based on naming patterns. Returns count of functions tagged.
pub fn tag_functions_by_pattern(
    graph_db: &GraphDb,
    functions: &[(String, String)], // (fn_id, fn_name)
) -> u32 {
    let mut count = 0u32;
    for (fn_id, fn_name) in functions {
        let mut tags = Vec::new();
        let lower_name = fn_name.to_lowercase();

        // Hook detection (React/Vue/Svelte)
        if fn_name.starts_with("use") && fn_name.len() > 3
            && fn_name.chars().nth(3).map_or(false, |c| c.is_uppercase())
        {
            tags.push("hook".to_string());
        }

        // Component detection (PascalCase + common suffixes)
        if fn_name.chars().next().map_or(false, |c| c.is_uppercase()) {
            if fn_name.ends_with("Component") || fn_name.ends_with("Page")
                || fn_name.ends_with("View") || fn_name.ends_with("Screen")
                || fn_name.ends_with("Layout") || fn_name.ends_with("Modal")
                || fn_name.ends_with("Dialog") || fn_name.ends_with("Form")
            {
                tags.push("component".to_string());
            }
        }

        // Handler detection
        if lower_name.starts_with("handle") || lower_name.starts_with("on") {
            if fn_name.len() > 4 {
                tags.push("handler".to_string());
            }
        }

        // Middleware
        if lower_name.contains("middleware") {
            tags.push("middleware".to_string());
        }

        // Route / API
        if lower_name.contains("route") || lower_name.contains("endpoint") {
            tags.push("route".to_string());
        }

        // Test
        if lower_name.starts_with("test_") || lower_name.starts_with("test") || lower_name.contains("_test") {
            tags.push("test".to_string());
        }

        if !tags.is_empty() {
            tags.sort();
            tags.dedup();
            graph_db.tag_function(fn_id, &tags.join(",")).ok();
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framework_detection() {
        let graph = GraphDb::open_memory().unwrap();
        graph.merge_file("file:a.ts", "/a.ts", "a", "typescript", "proj").unwrap();

        let mut file_imports = HashMap::new();
        file_imports.insert("file:a.ts".to_string(), vec!["react".to_string(), "next/router".to_string()]);

        let count = tag_files_by_imports(&graph, &file_imports);
        assert_eq!(count, 1);
    }

    #[test]
    fn function_pattern_tagging() {
        let graph = GraphDb::open_memory().unwrap();
        graph.merge_function("fn:1", "useState", "/a.ts", 1, "", "", "", 1, "proj").unwrap();
        graph.merge_function("fn:2", "handleClick", "/a.ts", 5, "", "", "", 1, "proj").unwrap();
        graph.merge_function("fn:3", "fetchData", "/a.ts", 10, "", "", "", 1, "proj").unwrap();

        let fns = vec![
            ("fn:1".to_string(), "useState".to_string()),
            ("fn:2".to_string(), "handleClick".to_string()),
            ("fn:3".to_string(), "fetchData".to_string()),
        ];
        let count = tag_functions_by_pattern(&graph, &fns);
        assert!(count >= 2, "Expected at least 2 tagged, got {}", count);
    }
}
