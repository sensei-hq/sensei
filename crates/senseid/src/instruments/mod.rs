//! MCP instrument registry — the catalog of MCP servers sensei knows
//! about for the setup-wizard Instruments stage and beyond.
//!
//! For now this is an embedded static list with stack-affinity keywords.
//! Future iterations should pull from the marketplace repo (where MCPs
//! could live alongside skills/plugins) and check `installed` status
//! against actual ACP configs.

use serde::Serialize;

/// One entry in the MCP registry. Kept `'static`-friendly so the whole
/// table can live as a `const` slice.
#[derive(Debug, Serialize, Clone)]
pub struct McpEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub publisher: &'static str,
    /// Coarse classification: "data" | "api" | "devtool" | "service".
    pub kind: &'static str,
    pub summary: &'static str,
    /// Approximate tool count exposed by this MCP.
    pub tools: u32,
    pub verified: bool,
    /// Lowercased substrings — match against any string in the detected
    /// stack to decide whether the MCP is `recommended` for that stack.
    pub stack_keywords: &'static [&'static str],
}

/// Curated registry. Each entry is `verified: true` — i.e. listed here is
/// an explicit thumbs-up. Third-party / community entries belong in a
/// separate marketplace-fetched section once that source lands.
pub const REGISTRY: &[McpEntry] = &[
    McpEntry {
        id: "postgres-mcp",
        name: "PostgreSQL MCP",
        publisher: "supabase",
        kind: "data",
        summary: "Introspect tables, run read-only SQL, draft migrations.",
        tools: 14,
        verified: true,
        stack_keywords: &["postgres", "postgresql", "psql", "supabase", "pgvector"],
    },
    McpEntry {
        id: "redis-mcp",
        name: "Redis MCP",
        publisher: "redis",
        kind: "data",
        summary: "Inspect keys, run commands, watch streams.",
        tools: 9,
        verified: true,
        stack_keywords: &["redis", "valkey"],
    },
    McpEntry {
        id: "stripe-mcp",
        name: "Stripe MCP",
        publisher: "stripe",
        kind: "api",
        summary: "Look up customers, charges, subscriptions; test mode only.",
        tools: 22,
        verified: true,
        stack_keywords: &["stripe"],
    },
    McpEntry {
        id: "github-mcp",
        name: "GitHub MCP",
        publisher: "github",
        kind: "devtool",
        summary: "Read repos, file PRs, browse issues without leaving the chat.",
        tools: 28,
        verified: true,
        stack_keywords: &["github", "git"],
    },
    McpEntry {
        id: "playwright-mcp",
        name: "Playwright MCP",
        publisher: "microsoft",
        kind: "devtool",
        summary: "Drive a headless browser — navigate, click, evaluate.",
        tools: 18,
        verified: true,
        stack_keywords: &["playwright", "tauri", "sveltekit", "react", "vue", "next"],
    },
    McpEntry {
        id: "filesystem-mcp",
        name: "Filesystem MCP",
        publisher: "anthropic",
        kind: "devtool",
        summary: "Read & write files inside an allow-listed working directory.",
        tools: 8,
        verified: true,
        stack_keywords: &[],
    },
];

/// Build the wire shape returned by `GET /api/instruments`.
///
/// `stack` is the union of detected languages/frameworks/runtimes/services
/// (lowercased) supplied by the caller. An entry is `recommended: true`
/// when any of its stack keywords appears as a substring in any stack
/// element. `installed` is always `false` for now — wiring the real check
/// against ACP MCP configs is a follow-up (we'd ask each assistant whether
/// the entry's MCP id is registered).
pub fn list_for_stack(stack: &[String]) -> Vec<serde_json::Value> {
    let stack_lower: Vec<String> = stack.iter().map(|s| s.to_lowercase()).collect();
    REGISTRY.iter().map(|m| {
        let recommended = !m.stack_keywords.is_empty()
            && m.stack_keywords.iter().any(|kw|
                stack_lower.iter().any(|s| s.contains(kw))
            );
        serde_json::json!({
            "id": m.id,
            "name": m.name,
            "publisher": m.publisher,
            "kind": m.kind,
            "summary": m.summary,
            "tools": m.tools,
            "verified": m.verified,
            "recommended": recommended,
            "installed": false,
            "selected": recommended,
            "project_count": 0,
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_nonempty_and_unique() {
        assert!(!REGISTRY.is_empty());
        let mut ids: Vec<&str> = REGISTRY.iter().map(|m| m.id).collect();
        ids.sort();
        let len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), len, "registry has duplicate ids");
    }

    #[test]
    fn recommended_matches_stack_substring() {
        let stack = vec!["PostgreSQL".to_string(), "Rust".to_string()];
        let result = list_for_stack(&stack);
        let postgres = result.iter().find(|m| m["id"] == "postgres-mcp").unwrap();
        assert_eq!(postgres["recommended"], true);
        // Stripe has no postgres/rust keywords — not recommended.
        let stripe = result.iter().find(|m| m["id"] == "stripe-mcp").unwrap();
        assert_eq!(stripe["recommended"], false);
    }

    #[test]
    fn empty_stack_yields_no_recommendations() {
        let result = list_for_stack(&[]);
        assert!(result.iter().all(|m| m["recommended"] == false));
        // Every entry is still returned — recommendation is per-stack, not gating.
        assert_eq!(result.len(), REGISTRY.len());
    }

    #[test]
    fn entries_without_keywords_never_recommend() {
        // The filesystem MCP intentionally has no keywords — generally useful,
        // shouldn't auto-recommend just because the stack list isn't empty.
        let stack = vec!["postgres".to_string()];
        let result = list_for_stack(&stack);
        let fs = result.iter().find(|m| m["id"] == "filesystem-mcp").unwrap();
        assert_eq!(fs["recommended"], false);
    }
}
