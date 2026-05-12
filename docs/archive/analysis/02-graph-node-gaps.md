---
name: Graph Node Schema — Gap Analysis
description: Current graph nodes are too thin to support pattern detection, traceability, impact analysis, or TDD guidance. Maps what's extracted vs. stored vs. needed.
date: 2026-04-17
status: analysis-complete
origin: docs/ideas/08-codebase-intelligence.md
impacts: 15-pattern-store.md, 17-pattern-knowledge.md, 13-doc-traceability.md, blueprints/01-workflow-engine.md
---

# Graph Node Schema — Gap Analysis

## Summary

The Rust indexer extracts rich data from ASTs (docstrings, return types, parameters, visibility) but **discards most of it before writing to the graph**. The stored nodes are skeletal: `{ name, kind, file, line, sig }`. This is a foundational gap — pattern detection, traceability, impact analysis, TDD guidance, and the workflow engine's locate step all depend on rich nodes.

**Critical bug:** Docstrings are fully extracted by all language adapters but explicitly passed as `None` in `graph_writer.rs:71`. The data exists in the pipeline — it just gets thrown away.

---

## Current vs. needed: Symbol nodes (functions, classes, types)

| Property | Extracted? | Stored? | Needed for |
|----------|-----------|---------|------------|
| `name` | Yes | Yes | Everything |
| `kind` (function, class, etc.) | Yes | Yes | Everything |
| `file` (path) | Yes | Yes | Everything |
| `line` (start) | Yes | Yes | Navigation |
| `line_end` | Yes | **No** | Symbol span, complexity, code generation |
| `sig` (signature) | Yes | Yes (500 char limit) | Search, pattern matching |
| `docstring` / comment | Yes (all languages) | **No** (discarded in graph_writer.rs) | Documentation, intent understanding, rationale extraction |
| `body` (full source) | Yes | **No** (max 10k extracted, not stored) | Detailed analysis, LLM summarization |
| `params` (names + types) | Partially (adapter-specific) | **No** | Function shape, TDD, type checking |
| `return_type` | Partially (TS, Rust) | **No** | Impact analysis, type safety |
| `is_exported` / visibility | Yes | **No** | Public API surface, encapsulation |
| `is_async` | Yes (some adapters) | **No** | Concurrency patterns |
| `decorators` / annotations | Yes (Python, Java) | **No** | Pattern detection (e.g., @route, @test) |
| `implements` / traits | Partially | **No** (only parent_id for methods) | Pattern detection (adapter = implements interface) |
| `inherits` / extends | Partially | **No** | Class hierarchy, pattern detection |
| `generic_params` | No | No | Type-safe refactoring |
| `complexity` | Yes | Yes | Quality metrics |
| `tags` | Yes | Yes | Classification |

**Impact:** Without `implements`, `inherits`, `params`, `return_type`, and `docstring`, the indexer cannot detect design patterns, the AI cannot understand function shape for TDD, and code intelligence is limited to "name + location."

---

## Current vs. needed: File nodes

| Property | Extracted? | Stored? | Needed for |
|----------|-----------|---------|------------|
| `name` | Yes | Yes | Navigation |
| `file` (path) | Yes | Yes | Everything |
| `level` (language) | Yes | Yes | Language-specific handling |
| `tags` (src/test/config) | Yes | Yes | Classification |
| `imports` (raw strings) | Yes | **No** (only unresolved refs, cleared after resolve) | Dependency analysis, import graph |
| `package` / module | Yes | Yes (via CONTAINS edges) | Hierarchy |

---

## Current vs. needed: Doc nodes

| Property | Extracted? | Stored? | Needed for |
|----------|-----------|---------|------------|
| `name` (title) | Yes | Yes | Navigation |
| `file` (path) | Yes | Yes | Everything |
| `doc_type` | Yes | Yes | Classification |
| `doc_category` | Yes | Yes | Classification |
| `description` (frontmatter) | Yes | **No** | Context delivery, search |
| `status` (frontmatter) | No | No | Traceability (is this doc current?) |
| `origin` (frontmatter) | No | No | Traceability (what idea spawned this doc?) |
| `date` (frontmatter) | No | No | Freshness detection, drift |
| `headings` (structure) | No | No | Section-level linking |
| `code_blocks` | No | No | Example extraction |
| `full frontmatter` | Partially | **No** | All metadata-driven features |

**Impact:** Without frontmatter metadata on doc nodes, traceability matrix (idea 13) can't work — we can't link a blueprint doc to the idea it came from, or detect when a design doc becomes stale because the code it references changed.

---

## Current vs. needed: Edges

| Edge | Exists? | Needed for |
|------|---------|------------|
| `CALLS` | Yes | Call graph |
| `IMPORTS` | Yes | Dependency graph |
| `EXPORTS_FN` / `EXPORTS_TYPE` | Yes | Public API surface |
| `CONTAINS_*` | Yes | Hierarchy |
| `HAS_METHOD` | Yes | Class structure |
| `COVERS` (doc → file) | Yes | Basic traceability |
| `MENTIONS_FN` (doc → function) | Yes | Basic traceability |
| `IMPLEMENTS` (class → interface) | **No** | Pattern detection (adapter, strategy) |
| `EXTENDS` (class → parent) | **No** | Class hierarchy, pattern detection |
| `RETURNS` (function → type) | **No** | Impact analysis, type-safe refactoring |
| `TAKES_PARAM` (function → param) | **No** | Function shape, TDD |
| `DUPLICATES` (symbol → symbol) | **No** | Duplicate detection |
| `SIMILAR_TO` (symbol → symbol) | **No** | Pattern suggestion |
| `TRACES_TO` (doc → doc) | **No** | Requirement → design → code traceability |

---

## The hierarchy gap

Current: `project → file → symbol`
Needed: `solution → repo → package → module → file → symbol → param`

| Level | Current | Needed | Purpose |
|-------|---------|--------|---------|
| Solution/Workspace | No | Yes | Multi-repo (idea 16) |
| Repository | Yes (project) | Yes | Project boundary |
| Package | Partial (CONTAINS_MOD) | Yes | Package-level analysis |
| Module | Yes | Yes | Module boundary |
| File | Yes | Yes | File-level navigation |
| Class/Struct/Interface | Yes (as symbol) | Yes (richer) | Type-level analysis, `implements`, `extends` |
| Function/Method | Yes (as symbol) | Yes (richer) | `params`, `returns`, `uses`, `docstring` |
| Parameter | No | Yes | Function shape, type checking |

---

## What's testable with rich nodes

This is the key connection to TDD. With rich nodes, the indexer itself becomes deeply testable:

**Current tests (thin):**
```rust
// Can only verify: "did we find a symbol named X at line Y?"
assert_eq!(symbol.name, "parse_file");
assert_eq!(symbol.line, 42);
```

**Tests with rich nodes:**
```rust
// Verify the complete shape of a function
let sym = index("fixtures/adapter.rs").find("parse_file");
assert_eq!(sym.name, "parse_file");
assert_eq!(sym.kind, NodeKind::Function);
assert_eq!(sym.params, vec![
    Param { name: "path", type_: "PathBuf" },
    Param { name: "content", type_: "str" },
]);
assert_eq!(sym.return_type, Some("Vec<Symbol>"));
assert_eq!(sym.implements, Some("LanguageAdapter"));
assert!(sym.docstring.unwrap().contains("Parse a source file"));
assert_eq!(sym.is_exported, true);
assert_eq!(sym.is_async, false);

// Verify cross-language consistency
let py_sym = index("fixtures/adapter.py").find("parse_file");
assert_eq!(py_sym.params.len(), sym.params.len());  // same shape
assert_eq!(py_sym.kind, NodeKind::Function);         // same kind
// type tags differ (module vs crate) but structure is consistent
```

**Tests for edges:**
```rust
let graph = index("fixtures/");
// Verify adapter pattern is detectable
let adapters = graph.find_implementations("LanguageAdapter");
assert_eq!(adapters.len(), 4);  // TS, Python, Rust, Java
for adapter in &adapters {
    assert!(graph.has_method(adapter, "parse_file"));
    assert!(graph.has_method(adapter, "resolve_imports"));
}
```

**Tests for docs:**
```rust
let doc = index_doc("fixtures/blueprint.md");
assert_eq!(doc.doc_type, "blueprint");
assert_eq!(doc.frontmatter.get("origin"), Some("ideas/01-workflow.md"));
assert_eq!(doc.frontmatter.get("status"), Some("complete"));
// Verify traceability links
let traces = graph.traces_from(&doc);
assert!(traces.iter().any(|t| t.target == "ideas/01-workflow.md"));
```

---

## Recommended node schema

### Symbol node (enriched)

```rust
struct SymbolNode {
    // Identity
    id: String,              // "fn:path:name:line"
    name: String,
    kind: NodeKind,          // function, method, class, struct, interface, enum, const, type, component, hook

    // Location
    file: String,            // repo-relative path
    line: u32,
    line_end: u32,

    // Signature & shape
    sig: String,             // declaration line
    params: Vec<Param>,      // name + type for each parameter
    return_type: Option<String>,

    // Metadata
    docstring: Option<String>,
    complexity: Option<u32>,
    is_exported: bool,
    is_async: bool,
    decorators: Vec<String>, // @route, @test, etc.

    // Relationships (stored as edges, not inline)
    // parent_id → HAS_METHOD edge
    // implements → IMPLEMENTS edge
    // extends → EXTENDS edge

    // Classification
    project: String,
    tags: Option<String>,
}

struct Param {
    name: String,
    type_: Option<String>,
    default: Option<String>,
}
```

### Doc node (enriched)

```rust
struct DocNode {
    // Identity
    id: String,              // "doc:path"
    name: String,            // title from frontmatter or filename

    // Location
    file: String,            // repo-relative path

    // Classification
    doc_type: Option<String>,     // idea, analysis, blueprint, experiment, plan, design, feature
    doc_category: Option<String>,

    // Frontmatter (all fields preserved)
    description: Option<String>,
    status: Option<String>,       // brainstorm, idea, complete, stale
    date: Option<String>,
    origin: Option<String>,       // parent doc path → TRACES_TO edge

    // Content structure
    headings: Vec<String>,        // h2/h3 headings for section-level linking

    // Classification
    project: String,
    tags: Option<String>,
}
```

---

## Prioritized fix list

### Quick wins (data exists, just needs storage)

| # | Fix | Effort | Impact |
|---|-----|--------|--------|
| 1 | **Store docstrings** — add docstring to SymbolResult, fix graph_writer.rs:71 | Tiny | High — unlocks intent understanding, rationale extraction |
| 2 | **Store line_end** — already extracted, add to graph write | Tiny | Medium — enables symbol span display |
| 3 | **Store is_exported** — already extracted, add to graph write | Tiny | Medium — enables public API surface analysis |

### Medium effort (adapter changes needed)

| # | Fix | Effort | Impact |
|---|-----|--------|--------|
| 4 | **Extract + store params** (name, type per param) | Medium | High — enables function shape, TDD guidance |
| 5 | **Extract + store return_type** | Medium | High — enables impact analysis, type checking |
| 6 | **Store doc frontmatter** (description, status, date, origin) | Medium | High — enables traceability, freshness detection |
| 7 | **Extract + store implements/extends** | Medium | Critical — enables pattern detection |
| 8 | **Extract + store decorators/annotations** | Low-Medium | Medium — enables route/test/hook detection |

### Higher effort (new edge types)

| # | Fix | Effort | Impact |
|---|-----|--------|--------|
| 9 | **IMPLEMENTS edge** (class → interface/trait) | Medium | Critical — pattern detection |
| 10 | **EXTENDS edge** (class → parent) | Medium | High — class hierarchy |
| 11 | **TRACES_TO edge** (doc → doc via frontmatter origin) | Medium | High — traceability matrix |
| 12 | **DUPLICATES edge** (symbol → symbol) | High | Medium — duplicate detection |

---

## Impact on other ideas

| Idea | Dependency on rich nodes | Minimum needed |
|------|-------------------------|----------------|
| 08 Codebase Intelligence | Pattern detection needs `implements`, `extends`, `decorators` | Fixes 7, 8, 9, 10 |
| 13 Doc Traceability | Traceability matrix needs doc frontmatter + TRACES_TO edge | Fixes 6, 11 |
| 14 Context Delivery | Function shape for resolution levels needs `params`, `return_type` | Fixes 4, 5 |
| 15 Pattern Store | Can't detect patterns without `implements`, class hierarchies | Fixes 7, 9, 10 |
| 17 Pattern Knowledge | Library pattern matching needs function shape | Fixes 4, 5, 7 |
| Blueprint: locate step | `get_patterns()` MCP tool needs pattern-detectable nodes | Fixes 7, 8, 9 |
| Blueprint: review step | Duplicate/violation detection needs rich comparison | Fixes 4, 5, 7, 12 |

**Conclusion:** Fixes 1-7 and 9 are prerequisites for most of the intelligence layer. Fixes 1-3 are near-zero effort (data exists, just not stored). The indexer foundation needs enriching before the workflow engine can deliver its full value.
