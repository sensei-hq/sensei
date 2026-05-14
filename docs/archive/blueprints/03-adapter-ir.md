---
name: Adapter IR Blueprint
description: Intermediate representation design — three node types (doc, module, class) with shared base, worker-compatible pipeline
date: 2026-04-18
status: blueprint
origin: docs/ideas/22-adapter-ir.md
analysis: docs/analysis/02-graph-node-gaps.md
---

# Adapter IR Blueprint

## Design goals

1. **Rich nodes from day one** — params, return types, implements, extends, decorators, docstrings all in the IR. No "add later" — the struct has the fields, adapters populate what they can.
2. **Three node types** — doc, module, class. Not a single unified symbol — structural differences matter.
3. **Optional everything** — frontmatter may be missing from docs, type annotations may be missing from JS. The IR uses `Option<>` everywhere. Processing gracefully handles missing data.
4. **Worker-compatible** — each file produces an IR independently. No shared state between files during parsing. Resolution (edges, parent linking) happens as a separate phase.
5. **One adapter at a time migration** — docs first, then Rust, then others follow the pattern.

---

## IR structure

### Base fields (shared by all node types)

```rust
/// Fields common to every node in the IR — code, docs, config all share these.
struct IRBase {
    name: String,
    file: String,              // repo-relative path
    line_start: u32,
    line_end: u32,

    // Classification (universal — not just doc-specific)
    extension: Option<String>,  // .rs, .md, .py, .svelte
    language: Option<String>,   // rust, markdown, python, svelte
    framework: Option<String>,  // sveltekit, axum, express, django
    node_type: Option<String>,  // function, class, doc, config, test, module
    category: Option<String>,   // design, feature, adapter, handler, utility

    // Content
    docstring: Option<String>,
    is_exported: bool,
    tags: Vec<String>,         // arbitrary classification tags
}
```

IRBase carries enough context for any downstream processor — pattern detection can check `framework`, testability can check `node_type`, traceability can check `category`.

### Document nodes

```rust
/// A document — markdown, text, yaml, config.
struct IRDoc {
    base: IRBase,
    doc_type: Option<String>,      // idea, analysis, blueprint, design, feature, usage, api-spec, changelog
    doc_category: Option<String>,  // from frontmatter or inferred from path

    // Frontmatter (all optional — may be absent in new/external repos)
    frontmatter: HashMap<String, String>,  // raw key-value pairs
    status: Option<String>,        // extracted from frontmatter if present
    origin: Option<String>,        // parent doc path (for traceability)
    date: Option<String>,

    // Structure
    title: Option<String>,         // from first # heading or frontmatter name
    sections: Vec<IRSection>,      // content split by headings — for section-level linking and context delivery
    code_blocks: Vec<IRCodeBlock>, // for example extraction

    // References found in content
    file_references: Vec<String>,  // backtick file paths → COVERS edges
    symbol_references: Vec<String>,// backtick function/type names → MENTIONS edges
    doc_references: Vec<String>,   // links to other docs → TRACES_TO edges
}

struct IRSection {
    heading: String,       // heading text
    level: u8,             // 1-6
    line_start: u32,
    line_end: u32,         // end of section content (before next heading or EOF)
    content_preview: Option<String>, // first 200 chars for context delivery
}

struct IRCodeBlock {
    language: Option<String>,
    content: String,
    line_start: u32,
    line_end: u32,
}
```

### Module nodes (functional)

```rust
/// A module — a file or namespace that contains functions, constants, and imports.
/// Represents: Rust module, Python module, JS/TS file, C file.
struct IRModule {
    base: IRBase,
    language: String,

    functions: Vec<IRFunction>,
    constants: Vec<IRConstant>,
    imports: Vec<IRImport>,
    type_aliases: Vec<IRTypeAlias>,

    // Module-level metadata
    is_test: bool,           // test file detected from path or content
    is_config: bool,         // config file (package.json, Cargo.toml, etc.)
}

/// A free function or a closure/lambda at module level.
struct IRFunction {
    base: IRBase,
    params: Vec<IRParam>,
    return_type: Option<String>,
    is_async: bool,
    decorators: Vec<String>,       // @route, #[test], etc.
    calls: Vec<String>,            // unresolved function names called in body
    complexity: u32,               // cyclomatic complexity estimate
    body_hash: Option<String>,     // normalized body hash for duplicate detection
}

struct IRParam {
    name: String,
    type_: Option<String>,
    default_value: Option<String>,
    is_optional: bool,
}

struct IRImport {
    source: String,          // module path ("./utils", "@rokkit/core", "std::path")
    names: Vec<String>,      // imported names (empty = wildcard/default)
    is_reexport: bool,
}

struct IRConstant {
    base: IRBase,
    type_: Option<String>,
    value_preview: Option<String>, // first 100 chars of value for display
}

struct IRTypeAlias {
    base: IRBase,
    target: String,          // what it aliases to
}
```

### Class nodes (OO)

```rust
/// A class, struct, interface, trait, or enum.
/// Represents: Java class, Rust struct+impl, Python class, TS interface.
struct IRClass {
    base: IRBase,
    class_kind: ClassKind,     // class, struct, interface, trait, enum, component
    language: String,

    methods: Vec<IRMethod>,
    properties: Vec<IRProperty>,
    constants: Vec<IRConstant>,

    // Relationships (names, resolved to IDs later)
    implements: Vec<String>,   // traits/interfaces
    extends: Option<String>,   // parent class
    mixins: Vec<String>,       // mixins/protocols

    decorators: Vec<String>,   // @Component, #[derive], etc.
    generic_params: Vec<String>, // <T, U> etc.
}

enum ClassKind {
    Class,
    Struct,
    Interface,
    Trait,
    Enum,
    Component,   // Svelte/Vue/React component
    Protocol,    // Swift protocol
}

/// A method — function that belongs to a class.
struct IRMethod {
    base: IRBase,
    params: Vec<IRParam>,
    return_type: Option<String>,
    is_async: bool,
    is_static: bool,
    is_abstract: bool,
    visibility: Visibility,
    decorators: Vec<String>,
    calls: Vec<String>,
    complexity: u32,
    body_hash: Option<String>,
}

enum Visibility {
    Public,
    Private,
    Protected,
    Internal,    // Rust pub(crate), Kotlin internal
}

struct IRProperty {
    base: IRBase,
    type_: Option<String>,
    is_readonly: bool,
    visibility: Visibility,
}
```

### Parsed file (what an adapter returns)

```rust
/// The complete IR output for one file.
struct ParsedFile {
    file_path: String,        // repo-relative
    language: String,

    // A file can contain any combination:
    modules: Vec<IRModule>,   // typically 1 for most files
    classes: Vec<IRClass>,    // 0 or more
    docs: Vec<IRDoc>,         // 0 or 1 (markdown files)

    // File-level metadata
    is_test_file: bool,
    is_config_file: bool,
    file_hash: String,        // content hash for change detection
}
```

---

## Pipeline

```
┌──────────────────────────┐
│ Phase 1: Parse            │
│ (language-specific,       │
│  per-file, parallelizable)│
│                           │
│  file → adapter.parse()   │
│         → ParsedFile (IR) │
└──────────┬───────────────┘
           │
           ▼
┌──────────────────────────┐
│ Phase 2: Write to graph   │
│ (common, per-file)        │
│                           │
│  IR → graph_writer.write()│
│       writes all fields   │
│       to hierarchy_nodes  │
└──────────┬───────────────┘
           │
           ▼
┌──────────────────────────┐
│ Phase 3: Resolve edges    │
│ (common, batch)           │
│                           │
│  calls → CALLS edges      │
│  implements → IMPLEMENTS  │
│  extends → EXTENDS        │
│  doc refs → COVERS/TRACES │
└──────────┬───────────────┘
           │
           ▼
┌──────────────────────────┐
│ Phase 4: Detect patterns  │
│ (common, batch)           │
│                           │
│  naming heuristics        │
│  structural analysis      │
│  duplicate detection      │
│  convention analysis      │
└──────────────────────────┘
```

### Worker compatibility

Phases 1-2 are per-file and parallelizable — each file's IR is independent. This maps directly to the existing task queue:

```
scan → repo → folder → file(parse + write) → resolve(batch) → detect(batch)
```

Phases 3-4 are batch operations that run after all files are written.

---

## Graph schema changes

The hierarchy_nodes table needs new columns to store IR fields:

```sql
ALTER TABLE hierarchy_nodes ADD COLUMN params TEXT;       -- JSON: [{name, type_, default_value, is_optional}]
ALTER TABLE hierarchy_nodes ADD COLUMN return_type TEXT;
ALTER TABLE hierarchy_nodes ADD COLUMN is_async INTEGER DEFAULT 0;
ALTER TABLE hierarchy_nodes ADD COLUMN decorators TEXT;    -- JSON array
ALTER TABLE hierarchy_nodes ADD COLUMN implements TEXT;    -- JSON array of names
ALTER TABLE hierarchy_nodes ADD COLUMN extends TEXT;
ALTER TABLE hierarchy_nodes ADD COLUMN visibility TEXT;
ALTER TABLE hierarchy_nodes ADD COLUMN class_kind TEXT;
ALTER TABLE hierarchy_nodes ADD COLUMN body_hash TEXT;     -- for duplicate detection
ALTER TABLE hierarchy_nodes ADD COLUMN generic_params TEXT; -- JSON array
ALTER TABLE hierarchy_nodes ADD COLUMN is_static INTEGER DEFAULT 0;
ALTER TABLE hierarchy_nodes ADD COLUMN is_abstract INTEGER DEFAULT 0;

-- Doc-specific columns
ALTER TABLE hierarchy_nodes ADD COLUMN frontmatter TEXT;   -- JSON: full frontmatter
ALTER TABLE hierarchy_nodes ADD COLUMN status TEXT;
ALTER TABLE hierarchy_nodes ADD COLUMN origin TEXT;
ALTER TABLE hierarchy_nodes ADD COLUMN description TEXT;
ALTER TABLE hierarchy_nodes ADD COLUMN headings TEXT;      -- JSON: [{level, text, line}]
```

Or: separate tables for different node types (mirrors the IR structure):

```sql
-- Option B: type-specific tables (cleaner, but more joins)
CREATE TABLE ir_functions(
    node_id TEXT PRIMARY KEY REFERENCES hierarchy_nodes(id),
    params TEXT,           -- JSON
    return_type TEXT,
    is_async INTEGER DEFAULT 0,
    complexity INTEGER DEFAULT 1,
    body_hash TEXT,
    decorators TEXT
);

CREATE TABLE ir_classes(
    node_id TEXT PRIMARY KEY REFERENCES hierarchy_nodes(id),
    class_kind TEXT,
    implements TEXT,        -- JSON array
    extends TEXT,
    generic_params TEXT,
    decorators TEXT
);

CREATE TABLE ir_docs(
    node_id TEXT PRIMARY KEY REFERENCES hierarchy_nodes(id),
    frontmatter TEXT,      -- JSON
    status TEXT,
    origin TEXT,
    description TEXT,
    headings TEXT           -- JSON
);
```

### Recommendation: Option B (separate tables)

- Cleaner schema — each table has only relevant fields
- No null columns for fields that don't apply (a function doesn't have `implements`)
- Easier to query: `JOIN ir_functions ON` when you need function details
- Matches the IR structure (IRFunction, IRClass, IRDoc)

---

## Migration plan

### Step 1: Docs adapter (markdown)

**Why first:** Simplest IR — no AST parsing, just frontmatter + content structure. Tests the full pipeline (IR → graph writer → edge resolution) without tree-sitter complexity.

**What changes:**
- New `IRDoc` struct in IR module
- Doc adapter returns `ParsedFile` with `docs: [IRDoc]`
- Graph writer stores all frontmatter fields
- Edge resolver creates TRACES_TO edges from `origin` field
- Tests: parse a markdown file with frontmatter, verify all fields in graph

### Step 2: Rust adapter

**Why second:** Has the most language features (traits, impl blocks, generics, visibility, async, attributes). If the IR handles Rust, it handles everything.

**What changes:**
- New `IRModule`, `IRClass`, `IRFunction`, `IRMethod` structs
- Rust adapter produces full IR with params, return types, implements (trait impls), decorators (#[test] etc.)
- Graph writer stores all fields in ir_functions/ir_classes tables
- Tests: parse a Rust file with struct+impl+trait, verify params, return types, implements in graph
- Duplicate detection uses body_hash from IR

### Step 3: Pattern established — migrate remaining adapters

Each adapter follows the Rust pattern:
- Python → easy (docstrings, type hints, decorators)
- Java → easy (javadoc, annotations, interfaces)
- TypeScript → medium (OXC parser produces different AST, but IR output is the same)
- Swift/Kotlin → follow Java pattern
- Svelte/Vue → use extract_script_blocks from common, delegate to TS adapter for script content
- SQL → simpler (tables, views, functions — no classes)
- C → simpler (functions, structs, no classes)

---

## What this enables

Once all adapters produce the IR:

| Feature | Before (without IR) | After (with IR) |
|---------|--------------------|-----------------|
| Pattern detection | Query names in graph (limited) | Analyze implements/extends/decorators (accurate) |
| Duplicate detection | Signature string comparison (noisy) | body_hash comparison (precise) |
| Node enrichment | 12 separate fixes per adapter | Already enriched by IR |
| Testability scoring | Can't compute (no params/returns) | IRFunction.params + calls + complexity |
| Doc traceability | Basic file matching | Frontmatter origin → TRACES_TO edges |
| New language | Copy adapter, modify everything | Implement parse() → ParsedFile, done |

---

## Resolved questions

| # | Question | Decision |
|---|----------|----------|
| 1 | Should IR structs replace types.rs or live alongside? | **Replace.** IR is the new types.rs. IRBase includes universal fields: extension, language, framework, type, category — not just doc-specific. Docs have sections as additional structure. |
| 2 | body_hash normalization? | Strip whitespace, normalize indentation. Start simple, refine if too many false positives. |
| 3 | Complexity: during parsing or post-processing? | **During parsing (Phase 1).** AST is already in memory, counting control flow nodes is cheap. Re-reading in Phase 2 would be wasteful. |
| 4 | Partial frontmatter in docs? | **Store raw HashMap AND extract known fields.** Fallback processing for missing fields — infer doc_type from path if not in frontmatter, infer title from first heading, etc. |
