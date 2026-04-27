---
name: Adapter Intermediate Representation
description: A common IR that all language parsers produce — separates parsing from processing, enables consistent graph nodes across languages
date: 2026-04-18
status: idea
related: 08-codebase-intelligence.md, 18-testability-tdd.md, 15-pattern-store.md
impacts: analysis/02-graph-node-gaps.md (fixes 4-8 depend on this), design/01-daemon/enhancements.md
---

# Adapter Intermediate Representation

## Problem

Each language adapter independently:
- Walks the AST
- Extracts symbols with ad-hoc field mapping
- Produces `ParsedSymbol` with inconsistent field population
- Some extract docstrings, some don't. Some extract params, some don't.
- Pattern detection, node enrichment, and testability scoring will all need to understand function shape — but the shape varies by adapter

The `common.rs` extraction helped with shared helpers, but the fundamental issue remains: **there's no contract for what an adapter must produce**. Adding a new property (params, return type, implements) means touching every adapter independently.

## Current state

```
Source file → Language Adapter → ParsedFile { symbols, imports, edges }
                                   ↓
                                 ParsedSymbol { name, kind, sig, docstring, line_start, line_end, is_exported, parent }
```

Each adapter populates these fields differently:
- Python: extracts docstrings, has type hints but doesn't store them
- Rust: extracts doc comments, has full type info but doesn't store params/returns
- Java: extracts javadoc, has annotations but doesn't store them
- TypeScript: OXC parser (different library), no docstring extraction
- Swift/Kotlin: similar to Java

## Proposed design

Split parsing into two phases:

```
Phase 1: Parse (language-specific)
  Source → Language Parser → IR (common intermediate representation)

Phase 2: Process (language-agnostic)
  IR → Graph Writer → enriched nodes with consistent fields
  IR → Pattern Detector → pattern candidates
  IR → Testability Scorer → testability metrics
```

### The IR contract

Every adapter must produce this structure. Fields are optional (not every language has all info), but the structure is the same.

```rust
struct ParsedModule {
    language: String,
    file_path: String,

    symbols: Vec<IRSymbol>,
    imports: Vec<IRImport>,
    edges: Vec<IREdge>,
}

struct IRSymbol {
    // Identity
    name: String,
    kind: SymbolKind,  // function, method, class, struct, interface, enum, const, type, component, hook

    // Location
    line_start: u32,
    line_end: u32,

    // Signature and shape
    signature: Option<String>,
    params: Vec<IRParam>,
    return_type: Option<String>,

    // Metadata
    docstring: Option<String>,
    is_exported: bool,
    is_async: bool,
    decorators: Vec<String>,

    // Relationships (expressed as names, resolved to IDs later)
    parent_name: Option<String>,     // class this method belongs to
    implements: Vec<String>,         // traits/interfaces this class implements
    extends: Option<String>,         // parent class

    // Language-specific tags (adapter can add arbitrary metadata)
    tags: HashMap<String, String>,
}

struct IRParam {
    name: String,
    type_: Option<String>,
    default: Option<String>,
}

struct IRImport {
    source: String,        // module path
    names: Vec<String>,    // imported names (empty = wildcard)
    is_reexport: bool,
}

struct IREdge {
    from_name: String,
    to_name: String,
    kind: EdgeKind,  // Calls, Implements, Extends, Uses
}
```

### What changes per adapter

Each adapter only needs to implement:
```rust
trait LanguageAdapter {
    fn language(&self) -> &str;
    fn parse(&self, source: &str, file_path: &str) -> ParsedModule;
}
```

The `ParsedModule` is the IR. All downstream processing (graph writing, pattern detection, enrichment) works on the IR, not on adapter-specific output.

### What stays common

- `common.rs` helpers (field_text, make_symbol, extract_script_blocks)
- Graph writing from IR → hierarchy_nodes (one implementation, all languages)
- Pattern detection from IR (works on IRSymbol fields)
- Testability scoring from IR (uses params, return_type, edges)
- Node enrichment is just "write all IR fields to graph" instead of "add fields one by one"

## Impact

| What | Before | After |
|------|--------|-------|
| Add a new field (e.g., params) | Touch every adapter + graph writer | Add to IR struct, update graph writer once, update adapters that can extract it |
| Add a new language | Copy an existing adapter, modify AST walking | Implement `parse() → ParsedModule`, everything else is free |
| Pattern detection | Query graph nodes with inconsistent fields | Work on IR with consistent fields |
| Node enrichment (analysis 02) | 12 separate fixes across adapters | One pass: write IR fields to graph |
| Testability scoring | Needs per-adapter understanding | Works on IRSymbol.params, return_type, edges |

## Open questions

| # | Question |
|---|----------|
| 1 | Should the IR be the same as the graph node schema, or a separate intermediate? Separate gives flexibility but adds a mapping step. |
| 2 | How do we handle language-specific features that don't map to common fields? The `tags` HashMap is a catch-all, but is that enough? |
| 3 | Should TypeScript switch from OXC to tree-sitter for consistency, or keep OXC and have the adapter produce the same IR? |
| 4 | Migration path: do we rewrite all adapters at once, or one at a time with backward compatibility? |
