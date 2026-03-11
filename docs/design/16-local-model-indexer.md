---
id: local-model-indexer
type: design
implements:
  - feature: indexing
    items: [multi-modal-search]
---

# Local Model Indexer

## Problem with the Current Approach

The current indexer (`reindex.ts`) uses regex to extract symbols. This is:

- **Fragile** — TypeScript generics, decorators, JSX, multiline signatures, template literals all break it
- **Shallow** — captures structure but not meaning (what a function does, why it exists, how it relates to others)
- **Language-specific** — each language needs its own set of patterns
- **No context** — a regex can't tell a React component from a utility function, or a type alias from a data model

AST parsing would fix fragility but not shallowness, and adds heavy language-specific dependencies (`typescript`, `python-ast`, `tree-sitter` per language) that conflict with sensei's goal of being a lightweight portable tool.

**The root issue:** indexing is a *semantic* task, not a structural one. We want to understand what code *means*, not just what it *looks like*.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Semantic search results must have ≥80% relevance for natural language queries |
| performance | Embedding generation must not block the main index run by more than 2x |
| reliability | Missing or unavailable local model must fall back to regex-based indexing |

---

## Approach: Local Model as Base Analyzer

> **Architecture note:** inference runs inside `@sensei/server`, not the CLI. The CLI sends file paths to the server's `/analyze` endpoint; the server manages model backends and returns `FileAnalysis`. See `14-server-package.md` for the package split and deployment models (local, org shared, cloud).

A small local model — running entirely on device, no API key, no token costs — replaces regex as the first layer of analysis. It reads a file and produces structured JSON:

```
file content
    ↓
@sensei/server  (persistent process, holds warm models)
  └── ModelBackend (Ollama llama3.2:3b / Transformers.js ONNX)
    ↓
FileAnalysis {
  symbols, summary, flows, examples, relations
}
    ↓
symbol-map.json, folder-map.json, traceability.json, llmspec.yaml
```

This one interface covers all languages. New languages cost zero additional implementation. The model understands context — it can infer intent, detect patterns, and capture relationships that regex never could.

---

## ModelBackend Interface

```typescript
interface ModelBackend {
  /** Short identifier shown in logs */
  name: string;

  /** One-time setup: load model, warm up, check health */
  init(): Promise<void>;

  /** Embed text into a vector (for similarity / traceability) */
  embed(text: string): Promise<number[]>;

  /**
   * Extract structured analysis from a file.
   * prompt is backend-agnostic — adapters inject tech-stack context here.
   */
  extract(content: string, instructions: ExtractionInstructions): Promise<FileAnalysis>;
}

interface ExtractionInstructions {
  filePath: string;
  language?: string;       // inferred by adapter, or left blank for model to detect
  techContext?: string;    // e.g. "React component library using TypeScript and Tailwind"
  focusHints?: string[];   // e.g. ["extract React hooks", "note exported component props"]
}
```

### Backends

| Backend | Model | Size | When used |
|---|---|---|---|
| `TransformersBackend` | `Xenova/all-MiniLM-L6-v2` (embed) + `Xenova/LaMini-Flan-T5-783M` (extract) | ~50MB + ~320MB | Default, zero-install after first run |
| `OllamaBackend` | Any Ollama model (default: `llama3.2:3b` or `phi3.5`) | varies | Auto-detected if `ollama` is running |
| `OpenAICompatBackend` | Any OpenAI-compatible endpoint | — | Optional, for remote inference |

Sensei auto-detects available backends at startup (Ollama health-check → Transformers fallback). The active backend is recorded in `.sensei/index-config.json` so incremental runs use the same backend.

---

## FileAnalysis Output Schema

```typescript
interface FileAnalysis {
  path: string;
  language: string;             // detected by model
  contentHash: string;          // sha256 of source — cache key for incremental
  analyzedAt: string;           // ISO timestamp

  // Primary index content
  symbols: AnalyzedSymbol[];
  summary: string;              // 1-2 sentence file purpose
  role?: string;                // "component" | "service" | "util" | "config" | "test" | ...

  // Richer artifacts (generated when model has capacity)
  flows?: Flow[];               // key execution paths through the file
  examples?: string[];          // usage examples found or inferred
  relations?: Relation[];       // explicit references to other files/modules

  // Embedding (for traceability / similarity search)
  embedding?: number[];         // from embed(summary + symbol names)
}

interface AnalyzedSymbol {
  name: string;
  kind: "function" | "class" | "type" | "const" | "interface" | "enum" | "method" | "hook" | "component";
  signature: string;            // L0 — the "what" (concise)
  description: string;          // L1 — brief plain-English explanation
  visibility: "public" | "internal";
  tags?: string[];              // ["async", "pure", "deprecated", "exported"]
}

interface Flow {
  name: string;                 // e.g. "happy path", "error path"
  steps: string[];              // ordered plain-English steps
}

interface Relation {
  kind: "imports" | "calls" | "implements" | "extends" | "covers";
  target: string;               // relative path or module name
}
```

This maps directly to the existing symbol-map levels:
- `symbol.signature` → **L0** (~10 tokens per symbol)
- `symbol.description` → **L1** (adds semantics)
- `flows` + `examples` → **L2** (logic flow, deferred per file if model is slow)
- full source → **L3** (never stored, fetched on demand)

---

## Incremental Indexing with Local Models

The per-file `contentHash` is the cache key. On each run:

```
for each file in scope:
  hash = sha256(fileContent)
  if hash === existingAnalysis.contentHash:
    skip  ← no model call needed
  else:
    analysis = backend.extract(content, instructions)
    store analysis + hash
```

**First run** (no cache): every file is analyzed. Cost scales with repo size and model speed.

**Subsequent runs** (incremental): only changed files are re-analyzed. For a typical edit, this is 1–5 files regardless of repo size.

**Git-aware delta**: the existing `git diff <lastCommit>..HEAD` pre-filter still applies — unchanged files never even reach the hash check.

**Analysis cache** is stored in `.sensei/analysis-cache.json` (or per-file in `.sensei/cache/<sha>.json` for large repos). It is gitignored — regenerated locally, never committed.

---

## Tech-Stack Adapters

Adapters wrap the base `ModelBackend` with tech-stack awareness. They don't change *how* inference runs — they enrich the `ExtractionInstructions` passed to it:

```typescript
interface TechAdapter {
  name: string;
  /** Which files this adapter handles */
  matches(filePath: string, packageInfo?: PackageInfo): boolean;
  /** Inject tech-specific context and hints into extraction instructions */
  enrich(filePath: string, packageInfo: PackageInfo | undefined): Partial<ExtractionInstructions>;
}
```

### Examples

**JS/TS adapter:**
```typescript
{
  name: "js-ts",
  matches: (p) => /\.(ts|tsx|js|jsx|mjs)$/.test(p),
  enrich: (p, pkg) => ({
    language: p.endsWith("x") ? "tsx/jsx" : "typescript",
    techContext: `${pkg?.stack.join(", ")} project`,
    focusHints: [
      "extract all exported symbols with their TypeScript signatures",
      pkg?.stack.includes("react") ? "note React hooks (use*) and components" : "",
      "note any @param / @returns JSDoc",
    ].filter(Boolean),
  }),
}
```

**React adapter** (extends JS/TS):
```typescript
focusHints: [
  "identify React components (PascalCase functions returning JSX)",
  "extract prop types / interfaces",
  "note custom hooks (use* prefix)",
  "identify context providers and consumers",
]
```

**Python adapter:**
```typescript
{
  name: "python",
  matches: (p) => p.endsWith(".py"),
  enrich: () => ({
    language: "python",
    focusHints: [
      "extract classes and public methods (no leading underscore)",
      "extract top-level functions",
      "note type annotations if present",
      "identify FastAPI/Django route decorators",
    ],
  }),
}
```

Adapters are discovered from:
1. Built-in list (js-ts, python, go, rust, ruby)
2. Per-repo `custom` array in `.sensei/llmspec.yaml` (future)

---

## Package Hierarchy and Folder Map

The local model also replaces pure-regex package info extraction. For the folder-map scanner:

```
for each discovered package.json / pyproject.toml / go.mod / Cargo.toml:
  read manifest text
  adapter.extractPackageInfo(manifestContent, readmeContent?)
    → { name, description, role, stack, entryPoints, scripts, dependencies }
```

The model reads the manifest as text and returns structured JSON — no field-by-field regex parsing. It can infer `role` from context ("this package has a `bin` entry and its name starts with `cli-`"), detect monorepo patterns ("this has a `workspaces` field"), and produce a coherent description even when `description` is absent from the manifest.

---

## What the Model Produces (vs. Current Regex)

| Artifact | Regex approach | Local model |
|---|---|---|
| Symbol names | ✓ (for known patterns) | ✓ (all languages, all patterns) |
| Symbol signatures | Partial (breaks on generics) | ✓ |
| Symbol descriptions | ✗ | ✓ |
| File summary | ✗ | ✓ |
| File role | ✗ | ✓ |
| Execution flows | ✗ | ✓ |
| Usage examples | ✗ | ✓ (inferred from tests / JSDoc) |
| Cross-file relations | ✗ | ✓ (from import statements + context) |
| Embeddings for similarity | ✗ | ✓ |
| Package hierarchy | Partial (package.json only) | ✓ (all manifests, inferred role) |
| Traceability links | Manual `@covers` only | ✓ (from relations + embeddings) |

---

## Artifact Pipeline

```
sensei index
  ↓
1. PackageScanner       → folder-map.json     (package hierarchy, tech stack)
2. FileAnalyzer         → analysis-cache/      (per-file analysis, embeddings)
3. SymbolMapBuilder     → symbol-map.json      (L0/L1/L2 from FileAnalysis)
4. TraceabilityBuilder  → traceability.json    (from relations + embedding similarity)
5. LlmspecGenerator     → llmspec.yaml         (auto-populated sections: packages, stack, shortcuts)
6. LlmsTxtGenerator     → llms.txt             (from llmspec + symbol map)
```

Each step reads the output of previous steps. Steps 1–2 are the model-heavy steps. Steps 3–6 are fast transforms over the cached analysis.

---

## Traceability via Embeddings

With per-file embeddings, traceability gains a signal that requires no explicit annotation:

```
embed(doc) → doc_vec
embed(code_summary) → code_vec
cosine_similarity(doc_vec, code_vec) > threshold → "likely covers"
```

Combined with explicit `Relation { kind: "covers" }` entries from the model's output (when it detects a doc describes a file), this gives a rich traceability graph without requiring `@covers` tags.

Drift detection uses the same embeddings: re-embed after a code change and compare to the stored doc embedding. Cosine distance drift → candidate for human review.

---

## Model Size and Startup

| Concern | Answer |
|---|---|
| First-run download | Transformers.js caches to `~/.cache/huggingface/hub/`. One-time ~400MB. |
| Cold start per run | ~1–2s model load for Transformers.js; ~100ms for Ollama (already running) |
| Per-file inference | ~50–500ms depending on file size and model. Parallelised across CPU cores. |
| Repo of 500 files (first run) | ~5–15 min depending on hardware. Subsequent: seconds. |
| No internet required | After initial model download, fully offline |

---

## Implementation Plan

1. `src/model/types.ts` — `ModelBackend`, `ExtractionInstructions`, `FileAnalysis`, `TechAdapter` interfaces
2. `src/model/transformers-backend.ts` — Transformers.js ONNX backend (embed + extract)
3. `src/model/ollama-backend.ts` — Ollama REST backend (auto-detected)
4. `src/model/resolve-backend.ts` — auto-detect available backend at startup
5. `src/adapters/js-ts.ts`, `python.ts`, `go.ts` — tech adapters (replaces regex in reindex.ts)
6. `src/tools/file-analyzer.ts` — orchestrates backend + adapters, manages analysis cache
7. `src/tools/reindex.ts` — replace `extractExports` / `extractHeuristic` / `extractMarkdownSymbols` with `FileAnalyzer`
8. `src/tools/traceability.ts` — embedding-based traceability (replaces manual `@covers` only)
9. Wire `PackageScanner` (from package-adapters design) into `file-analyzer.ts` for tech context

Tests: mock `ModelBackend` for unit tests; real backend in integration tests (opt-in via `SENSEI_TEST_MODEL=1`).
