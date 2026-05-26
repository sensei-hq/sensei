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
    /// Matching is bidirectional substring: keyword "rust" matches stack
    /// tag "Rust" (via lowercasing) AND "rust-toolchain" (via contains),
    /// stack tag "Svelte" matches keyword "svelte" (the keyword is
    /// shorter and contained in the lowercased stack value).
    pub stack_keywords: &'static [&'static str],
    /// Lowercased keys that might identify this MCP in an assistant's MCP
    /// config block (e.g. "mcpServers"."postgres"). If any name in this
    /// list appears as a key in any installed assistant's config, the
    /// entry reports `installed: true`. Users sometimes rename entries,
    /// so list every common alias.
    pub config_names: &'static [&'static str],
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
        stack_keywords: &["postgres", "postgresql", "psql", "supabase", "pgvector", "sqlx"],
        config_names: &["postgres", "postgresql", "pg", "supabase"],
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
        config_names: &["redis", "valkey"],
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
        config_names: &["stripe"],
    },
    McpEntry {
        id: "github-mcp",
        name: "GitHub MCP",
        publisher: "github",
        kind: "devtool",
        summary: "Read repos, file PRs, browse issues without leaving the chat.",
        tools: 28,
        verified: true,
        // Recommended for any project that lives in git — almost everyone.
        stack_keywords: &["github", "git", "rust", "cargo", "svelte", "typescript", "javascript", "python", "go"],
        config_names: &["github", "gh"],
    },
    McpEntry {
        id: "playwright-mcp",
        name: "Playwright MCP",
        publisher: "microsoft",
        kind: "devtool",
        summary: "Drive a headless browser — navigate, click, evaluate.",
        tools: 18,
        verified: true,
        stack_keywords: &["playwright", "tauri", "svelte", "sveltekit", "react", "vue", "next", "vite"],
        config_names: &["playwright"],
    },
    McpEntry {
        id: "filesystem-mcp",
        name: "Filesystem MCP",
        publisher: "anthropic",
        kind: "devtool",
        summary: "Read & write files inside an allow-listed working directory.",
        tools: 8,
        verified: true,
        // No stack affinity — useful for any project, but shouldn't auto-recommend.
        stack_keywords: &[],
        config_names: &["filesystem", "fs"],
    },
];

/// Build the wire shape returned by `GET /api/instruments`.
///
/// `stack` is the union of detected languages/frameworks/runtimes/services
/// (lowercased) supplied by the caller. `installed_keys` is the set of
/// lowercased MCP config keys present across all assistant config files
/// (see `crate::assistants::installed_mcp_keys`).
///
/// An entry is:
///   • `recommended: true` when any of its stack keywords appears as a
///     substring in any stack element.
///   • `installed: true` when any of its `config_names` aliases appears in
///     `installed_keys` — i.e. the user already wired the MCP into at
///     least one assistant.
///   • `selected: true` when recommended-by-stack OR already-installed —
///     installed implies "the user already wants this" without prompting.
pub fn list_for_stack(
    stack: &[String],
    installed_keys: &std::collections::HashSet<String>,
) -> Vec<serde_json::Value> {
    let stack_lower: Vec<String> = stack.iter().map(|s| s.to_lowercase()).collect();
    REGISTRY.iter().map(|m| {
        let recommended = !m.stack_keywords.is_empty()
            && m.stack_keywords.iter().any(|kw|
                stack_lower.iter().any(|s| s.contains(kw))
            );
        let installed = m.config_names.iter().any(|n| installed_keys.contains(*n));
        serde_json::json!({
            "id": m.id,
            "name": m.name,
            "publisher": m.publisher,
            "kind": m.kind,
            "summary": m.summary,
            "tools": m.tools,
            "verified": m.verified,
            "recommended": recommended,
            "installed": installed,
            "selected": recommended || installed,
            "project_count": 0,
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn empty_installed() -> HashSet<String> { HashSet::new() }

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
        let result = list_for_stack(&stack, &empty_installed());
        let postgres = result.iter().find(|m| m["id"] == "postgres-mcp").unwrap();
        assert_eq!(postgres["recommended"], true);
        // Stripe has no postgres/rust keywords — not recommended.
        let stripe = result.iter().find(|m| m["id"] == "stripe-mcp").unwrap();
        assert_eq!(stripe["recommended"], false);
    }

    #[test]
    fn empty_stack_yields_no_recommendations() {
        let result = list_for_stack(&[], &empty_installed());
        assert!(result.iter().all(|m| m["recommended"] == false));
        // Every entry is still returned — recommendation is per-stack, not gating.
        assert_eq!(result.len(), REGISTRY.len());
    }

    #[test]
    fn entries_without_keywords_never_recommend() {
        // The filesystem MCP intentionally has no keywords — generally useful,
        // shouldn't auto-recommend just because the stack list isn't empty.
        let stack = vec!["postgres".to_string()];
        let result = list_for_stack(&stack, &empty_installed());
        let fs = result.iter().find(|m| m["id"] == "filesystem-mcp").unwrap();
        assert_eq!(fs["recommended"], false);
    }

    #[test]
    fn svelte_rust_stack_recommends_playwright_and_github() {
        // Mirrors the sensei dev stack — make sure both the test stack and a
        // common framework get a usable recommendation set instead of an
        // empty list.
        let stack = vec!["Svelte".to_string(), "Rust".to_string()];
        let result = list_for_stack(&stack, &empty_installed());
        let playwright = result.iter().find(|m| m["id"] == "playwright-mcp").unwrap();
        let github = result.iter().find(|m| m["id"] == "github-mcp").unwrap();
        assert_eq!(playwright["recommended"], true, "playwright should match svelte");
        assert_eq!(github["recommended"], true, "github should match svelte/rust");
    }

    #[test]
    fn installed_is_true_when_alias_present_in_assistant_config() {
        let mut installed = HashSet::new();
        installed.insert("postgres".to_string());
        let result = list_for_stack(&[], &installed);
        let postgres = result.iter().find(|m| m["id"] == "postgres-mcp").unwrap();
        assert_eq!(postgres["installed"], true);
        // installed implies selected — the user already wired it in.
        assert_eq!(postgres["selected"], true);
        // Unrelated entries stay uninstalled / unselected.
        let stripe = result.iter().find(|m| m["id"] == "stripe-mcp").unwrap();
        assert_eq!(stripe["installed"], false);
        assert_eq!(stripe["selected"], false);
    }

    #[test]
    fn installed_match_is_case_insensitive_via_lowered_keys() {
        // Caller (assistants::installed_mcp_keys) lowercases keys, so the
        // registry only needs lowercase config_names.
        let mut installed = HashSet::new();
        installed.insert("gh".to_string()); // alternate alias for GitHub MCP
        let result = list_for_stack(&[], &installed);
        let github = result.iter().find(|m| m["id"] == "github-mcp").unwrap();
        assert_eq!(github["installed"], true);
    }
}
