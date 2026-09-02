//! Curated MCP tool manifests for the sensei server.
//!
//! Data source for `GET /api/mcp/tools`. Each entry captures the shape the
//! Instruments playground consumes (`kind`, `summary`, structured `inputs`,
//! and an `example` with a sample response). Kept in one const table so the
//! response stays deterministic and every tool the daemon dispatches on has
//! matching metadata.
//!
//! **Slice A' (this module)** — hardcoded const. Slice A of the T2 plan
//! moves the manifests into a `sensei.mcp_tool_manifests` table once the
//! `mcp_servers` registry lands (T2 Slice B).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct McpToolManifest {
    /// MCP server namespace this tool belongs to (`"sensei"` for now).
    pub mcp: &'static str,
    /// Unique slug (`"sensei.search"`, `"sensei.get_symbol"`, …).
    pub id: &'static str,
    /// Callable name — what `mcp_call_tool` matches on.
    pub name: &'static str,
    /// `query` (read-only) vs `action` (mutates state).
    pub kind: McpToolKind,
    /// One-line description shown in the playground list.
    pub summary: &'static str,
    /// Structured inputs — one entry per argument the tool accepts.
    pub inputs: Vec<McpToolInput>,
    /// Sample invocation + response — for the playground detail pane.
    pub example: McpToolExample,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpToolKind {
    /// Read-only — safe to run against any project without side effects.
    Query,
    /// Mutates persisted state (creates rows, enqueues tasks, writes memory).
    Action,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct McpToolInput {
    /// Argument name (matches the JSON key `mcp_call_tool` reads).
    pub key: &'static str,
    /// Input control kind.
    pub kind: McpInputKind,
    pub required: bool,
    pub label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<&'static str>,
    /// Only set when `kind = Enum`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<&'static str>>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpInputKind {
    /// Free-form text.
    Text,
    /// Choose one of `options`.
    Enum,
    /// Integer / decimal.
    Number,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct McpToolExample {
    /// A sample response the tool would return for `inputs_used`.
    pub response: &'static str,
}

/// Return the curated list of MCP tool manifests. The set matches every
/// branch in `mcp::mcp_call_tool` so `list_tools` and `call_tool` stay in
/// lockstep — a tool that can be called can always be listed.
pub fn manifests() -> Vec<McpToolManifest> {
    vec![
        // ── Code graph queries ─────────────────────────────────────────
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.search",
            name: "search",
            kind: McpToolKind::Query,
            summary: "Search functions and types by name across a project.",
            inputs: vec![
                McpToolInput {
                    key: "query",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Query",
                    placeholder: Some("PgStore"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "types: [PgStore]\nfunctions: [get_tools_health, get_project_summary, ...]",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_symbol",
            name: "get_symbol",
            kind: McpToolKind::Query,
            summary: "Get details for a function or type by name.",
            inputs: vec![
                McpToolInput {
                    key: "name",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Symbol",
                    placeholder: Some("PgStore"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "PgStore · crates/senseid/src/db/pg_store.rs\n  struct — the daemon's Postgres access layer",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_callers",
            name: "get_callers",
            kind: McpToolKind::Query,
            summary: "Return functions that call the given symbol, with whether the symbol was found and whether the list is complete.",
            inputs: vec![
                McpToolInput {
                    key: "name",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Callee",
                    placeholder: Some("upsert_referenced_library"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "upsert_referenced_library — found, defined at pg_store/library.rs:311\n3 callers (coverage: 3 resolved, 0 unresolved — complete):\n  extract_deps  · libraries.rs:249\n  extract_dep_versions · lib_indexer.rs:88\n  seed_library · pg_store.rs:1914",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_callees",
            name: "get_callees",
            kind: McpToolKind::Query,
            summary: "Return functions the given symbol calls, each tagged internal / external / unknown, with whether the list is complete.",
            inputs: vec![
                McpToolInput {
                    key: "name",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Caller",
                    placeholder: Some("extract_deps"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "extract_deps — found, defined at tasks/handlers/libraries.rs:249\ncalls (coverage: 3 resolved, 1 unresolved — INCOMPLETE, grep to confirm):\n  upsert_referenced_library · internal · pg_store/library.rs:311\n  upsert_project_dependency · internal · pg_store/projects.rs:88\n  parse_cargo_deps · internal · adapters/manifest/cargo.rs:41\n  serde_json::from_str · external",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_file_tags",
            name: "get_file_tags",
            kind: McpToolKind::Query,
            summary: "Return files matching a framework tag (svelte, react, tauri, …).",
            inputs: vec![
                McpToolInput {
                    key: "tag",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Tag",
                    placeholder: Some("svelte"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "27 files tagged 'svelte':\n  src/routes/+page.svelte\n  src/lib/components/Card.svelte\n  ... +25 more",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_communities",
            name: "get_communities",
            kind: McpToolKind::Query,
            summary: "Return code architecture clusters (community detection).",
            inputs: vec![McpToolInput {
                key: "repoId",
                kind: McpInputKind::Text,
                required: true,
                label: "Project",
                placeholder: Some("sensei"),
                default: None,
                options: None,
            }],
            example: McpToolExample {
                response: "4 communities:\n  1. api/ (34 nodes)\n  2. tasks/ (52 nodes)\n  3. adapters/ (18 nodes)\n  4. db/ (23 nodes)",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_doc_drift",
            name: "get_doc_drift",
            kind: McpToolKind::Query,
            summary: "Return docs whose claims contradict the current code.",
            inputs: vec![McpToolInput {
                key: "repoId",
                kind: McpInputKind::Text,
                required: true,
                label: "Project",
                placeholder: Some("sensei"),
                default: None,
                options: None,
            }],
            example: McpToolExample {
                response: "2 drift signals:\n  README.md — references deleted `parse_ini_deps`\n  docs/backlog.md — describes 'pending' feature shipped 2 weeks ago",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.search_lib_docs",
            name: "search_lib_docs",
            kind: McpToolKind::Query,
            summary: "Search across all indexed library documentation.",
            inputs: vec![McpToolInput {
                key: "query",
                kind: McpInputKind::Text,
                required: true,
                label: "Query",
                placeholder: Some("dbd deploy"),
                default: None,
                options: None,
            }],
            example: McpToolExample {
                response: "5 pages match 'dbd deploy':\n  dbd-rs · deploy — apply DDL to the target DB\n  dbd-rs · combine — bundle DDL into init.sql\n  ...",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_lib_docs",
            name: "get_lib_docs",
            kind: McpToolKind::Query,
            summary: "Fetch a library's overview or a specific component's docs.",
            inputs: vec![
                McpToolInput {
                    key: "name",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Library",
                    placeholder: Some("rokkit"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "component",
                    kind: McpInputKind::Text,
                    required: false,
                    label: "Component",
                    placeholder: Some("List"),
                    default: None,
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "rokkit / List:\n  A grouped list component with keyboard navigation.\n  Props: items, value, fields, onselect, onchange",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.list_projects",
            name: "list_projects",
            kind: McpToolKind::Query,
            summary: "List all indexed projects (repositories + standalone folders).",
            inputs: vec![],
            example: McpToolExample {
                response: "32 projects:\n  sensei · Rust + SvelteKit · library\n  rokkit · SvelteKit · library\n  dbd-rs · Rust · tool\n  ...",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.query",
            name: "query",
            kind: McpToolKind::Query,
            summary: "Natural-language query routed across the code graph.",
            inputs: vec![
                McpToolInput {
                    key: "q",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Question",
                    placeholder: Some("what calls scan_root?"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "Use POST /api/query directly for unified NL query handling.",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_project_summary",
            name: "get_project_summary",
            kind: McpToolKind::Query,
            summary: "Return project stats and identifying metadata.",
            inputs: vec![McpToolInput {
                key: "repoId",
                kind: McpInputKind::Text,
                required: true,
                label: "Project",
                placeholder: Some("sensei"),
                default: None,
                options: None,
            }],
            example: McpToolExample {
                response: "sensei:\n  functions: 812   types: 143\n  stack: rust, typescript, svelte\n  role: library",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_metrics",
            name: "get_metrics",
            kind: McpToolKind::Query,
            summary: "Return project quality metrics: FTR, session count, corrections.",
            inputs: vec![McpToolInput {
                key: "repoId",
                kind: McpInputKind::Text,
                required: true,
                label: "Project",
                placeholder: Some("sensei"),
                default: None,
                options: None,
            }],
            example: McpToolExample {
                response: "sensei · 76 sessions · FTR 0.83 · 12 corrections · avg 4.2 turns",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_ftr_daily",
            name: "get_ftr_daily",
            kind: McpToolKind::Query,
            summary: "Daily FTR sparkline data for a project.",
            inputs: vec![
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "days",
                    kind: McpInputKind::Number,
                    required: false,
                    label: "Window (days)",
                    placeholder: None,
                    default: Some("14"),
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "14d FTR (sensei):\n  06/18 0.75  06/19 0.80  06/20 0.83  ...  07/01 0.88",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_hotspots",
            name: "get_hotspots",
            kind: McpToolKind::Query,
            summary: "Files with the highest rework / correction frequency.",
            inputs: vec![
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "days",
                    kind: McpInputKind::Number,
                    required: false,
                    label: "Window (days)",
                    placeholder: None,
                    default: Some("7"),
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "7d hotspots (sensei):\n  scan_logic.rs   6 rework, 2 corrections\n  detector.rs     4 rework\n  libraries.rs    3 rework",
            },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.get_quality_signals",
            name: "get_quality_signals",
            kind: McpToolKind::Query,
            summary: "Roll-up: FTR, pattern compliance, doc drift, test pass rate.",
            inputs: vec![McpToolInput {
                key: "repoId",
                kind: McpInputKind::Text,
                required: true,
                label: "Project",
                placeholder: Some("sensei"),
                default: None,
                options: None,
            }],
            example: McpToolExample {
                response: "sensei signals:\n  FTR 0.83   Doc drift 2   Pattern compliance 0.91   Tests OK",
            },
        },
        // ── Session + library actions ──────────────────────────────────
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.create_session",
            name: "create_session",
            kind: McpToolKind::Action,
            summary: "Open a new work session on a project.",
            inputs: vec![
                McpToolInput {
                    key: "repoId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Project",
                    placeholder: Some("sensei"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "task",
                    kind: McpInputKind::Text,
                    required: false,
                    label: "Task",
                    placeholder: Some("Fix scanner bug"),
                    default: Some("untitled"),
                    options: None,
                },
            ],
            example: McpToolExample { response: "{ ok: true, sessionId: \"6a0c3b1a-…\" }" },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.update_session",
            name: "update_session",
            kind: McpToolKind::Action,
            summary: "Close a session with an outcome verdict.",
            inputs: vec![
                McpToolInput {
                    key: "sessionId",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Session",
                    placeholder: Some("6a0c3b1a-…"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "outcome",
                    kind: McpInputKind::Enum,
                    required: false,
                    label: "Outcome",
                    placeholder: None,
                    default: Some("completed"),
                    options: Some(vec!["completed", "abandoned", "interrupted"]),
                },
                McpToolInput {
                    key: "turns",
                    kind: McpInputKind::Number,
                    required: false,
                    label: "Turns",
                    placeholder: None,
                    default: Some("0"),
                    options: None,
                },
                McpToolInput {
                    key: "corrections",
                    kind: McpInputKind::Number,
                    required: false,
                    label: "Corrections",
                    placeholder: None,
                    default: Some("0"),
                    options: None,
                },
            ],
            example: McpToolExample { response: "{ ok: true }" },
        },
        McpToolManifest {
            mcp: "sensei",
            id: "sensei.add_library",
            name: "add_library",
            kind: McpToolKind::Action,
            summary: "Register a library for doc indexing (local dir, GitHub tree, or llms.txt URL).",
            inputs: vec![
                McpToolInput {
                    key: "name",
                    kind: McpInputKind::Text,
                    required: true,
                    label: "Library",
                    placeholder: Some("rokkit"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "url",
                    kind: McpInputKind::Text,
                    required: false,
                    label: "Source URL",
                    placeholder: Some("https://github.com/jerrythomas/rokkit/tree/main/docs"),
                    default: None,
                    options: None,
                },
                McpToolInput {
                    key: "version",
                    kind: McpInputKind::Text,
                    required: false,
                    label: "Version",
                    placeholder: Some("1.3.1"),
                    default: None,
                    options: None,
                },
            ],
            example: McpToolExample {
                response: "{ ok: true, libName: \"rokkit\", taskId: \"…\", status: \"indexing\" }",
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_manifest_has_the_playground_shape() {
        for m in manifests() {
            assert_eq!(m.mcp, "sensei", "{}: mcp namespace", m.name);
            assert!(m.id.starts_with("sensei."), "{}: id must be namespaced", m.name);
            assert!(!m.summary.is_empty(), "{}: needs a summary", m.name);
            assert!(!m.example.response.is_empty(), "{}: needs an example response", m.name);
        }
    }

    #[test]
    fn manifest_id_matches_the_dotted_name() {
        for m in manifests() {
            assert_eq!(m.id, format!("sensei.{}", m.name), "id/name skew on {}", m.name);
        }
    }

    #[test]
    fn manifest_ids_are_unique() {
        let ids: Vec<&str> = manifests().iter().map(|m| m.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "duplicate manifest ids");
    }

    #[test]
    fn every_enum_input_carries_its_options() {
        for m in manifests() {
            for input in &m.inputs {
                if input.kind == McpInputKind::Enum {
                    assert!(
                        input.options.as_ref().is_some_and(|o| !o.is_empty()),
                        "{}.{}: enum input must supply options",
                        m.name,
                        input.key
                    );
                } else {
                    assert!(
                        input.options.is_none(),
                        "{}.{}: only enum inputs may set options",
                        m.name,
                        input.key
                    );
                }
            }
        }
    }

    #[test]
    fn actions_and_queries_both_present() {
        // Playground needs at least one of each so the kind chip filter is
        // meaningful; guards a regression where every tool got labelled the same.
        let queries = manifests().iter().filter(|m| m.kind == McpToolKind::Query).count();
        let actions = manifests().iter().filter(|m| m.kind == McpToolKind::Action).count();
        assert!(queries > 0);
        assert!(actions > 0);
    }

    #[test]
    fn serialize_shape_uses_lowercase_kind_and_omits_none_fields() {
        // Snapshot the wire shape for one representative tool.
        let m = manifests().into_iter().find(|m| m.name == "search").unwrap();
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["kind"], "query");
        assert_eq!(v["inputs"][0]["kind"], "text");
        // Fields that were `None` should not appear in the wire shape.
        assert!(v["inputs"][0].get("default").is_none());
        assert!(v["inputs"][0].get("options").is_none());
        // Fields that were `Some` should serialise.
        assert_eq!(v["inputs"][0]["placeholder"], "PgStore");
    }

    #[test]
    fn every_listed_tool_has_a_call_dispatch() {
        // Sanity: the list_tools + call_tool sets stay in lockstep. If a new
        // tool lands in manifests() without a mcp_call_tool branch, this test
        // fails loudly.
        const DISPATCHED: &[&str] = &[
            "search",
            "get_symbol",
            "get_callers",
            "get_callees",
            "get_file_tags",
            "get_communities",
            "get_doc_drift",
            "search_lib_docs",
            "get_lib_docs",
            "list_projects",
            "create_session",
            "update_session",
            "add_library",
            "query",
            "get_project_summary",
            "get_metrics",
            "get_ftr_daily",
            "get_hotspots",
            "get_quality_signals",
        ];
        for m in manifests() {
            assert!(
                DISPATCHED.contains(&m.name),
                "{}: listed but not dispatched by mcp_call_tool",
                m.name
            );
        }
    }
}
