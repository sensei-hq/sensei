# Doc Guide & Traceability System — Implementation Plan

> **For agentic workers:** REQUIRED: Use `superpowers:subagent-driven-development` (if subagents available) or `superpowers:executing-plans` to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish consistent feature/design doc naming, frontmatter, a fully-populated `docs/traceability.yaml`, and an updated `doc-guide` skill with embedded templates.

**Architecture:** Feature docs get `NN-` prefixes and minimal frontmatter (`id`, `type`). Design docs get frontmatter with `implements` links. A single `docs/traceability.yaml` is the machine-readable source of truth for feature items, status, NFRs, and feature→design→code connections. The `doc-guide` skill encodes all conventions and workflows for agents with or without MCP.

**Spec:** `docs/superpowers/specs/2026-03-11-doc-guide-traceability-design.md`

**No code involved** — this plan is entirely docs, YAML, and skill files. "Verify" steps replace test steps.

---

## Chunk 1: Feature Docs — Rename + Frontmatter

**Files:**
- Rename: `docs/features/{indexing,resolution,workflow,traceability,context,benchmarking,cli,documentation,patterns,caching}.md`
- Result: `docs/features/0N-<name>.md` (10 files)

### Task 1: Rename feature docs and add frontmatter

- [ ] **Step 1: Rename all 10 feature files with NN prefix**

```bash
cd docs/features
git mv indexing.md    01-indexing.md
git mv resolution.md  02-resolution.md
git mv workflow.md    03-workflow.md
git mv traceability.md 04-traceability.md
git mv context.md     05-context.md
git mv benchmarking.md 06-benchmarking.md
git mv cli.md         07-cli.md
git mv documentation.md 08-documentation.md
git mv patterns.md    09-patterns.md
git mv caching.md     10-caching.md
```

- [ ] **Step 2: Add frontmatter to each feature doc**

Prepend the following YAML frontmatter block to each file. The `id` is the filename stem without the NN prefix.

`01-indexing.md`:
```yaml
---
id: indexing
type: feature
---
```

`02-resolution.md`:
```yaml
---
id: resolution
type: feature
---
```

`03-workflow.md`:
```yaml
---
id: workflow
type: feature
---
```

`04-traceability.md`:
```yaml
---
id: traceability
type: feature
---
```

`05-context.md`:
```yaml
---
id: context
type: feature
---
```

`06-benchmarking.md`:
```yaml
---
id: benchmarking
type: feature
---
```

`07-cli.md`:
```yaml
---
id: cli
type: feature
---
```

`08-documentation.md`:
```yaml
---
id: documentation
type: feature
---
```

`09-patterns.md`:
```yaml
---
id: patterns
type: feature
---
```

`10-caching.md`:
```yaml
---
id: caching
type: feature
---
```

- [ ] **Step 3: Verify titles have no number prefix**

Each file's `# Title` line must be the plain functional name. Check:
- `01-indexing.md` → `# Indexing` ✓
- `02-resolution.md` → `# Content Compression` — update to `# Resolution`
- `03-workflow.md` → `# Workflow` ✓
- `04-traceability.md` → `# Traceability` ✓
- `05-context.md` → `# Context Management` — update to `# Context`
- `06-benchmarking.md` → `# Benchmarking` ✓
- `07-cli.md` → `# CLI` ✓
- `08-documentation.md` → `# Documentation` ✓
- `09-patterns.md` → `# Pattern Identification` — update to `# Patterns`
- `10-caching.md` → `# Claude Response Caching` — update to `# Caching`

- [ ] **Step 4: Verify frontmatter is the first thing in each file**

Run:
```bash
for f in docs/features/0*.md; do head -3 "$f"; echo "---"; done
```
Expected: every file starts with `---`, then `id:`, then `type: feature`.

- [ ] **Step 5: Commit**

```bash
git add docs/features/
git commit -m "docs(features): add NN prefixes, frontmatter, clean titles"
```

---

## Chunk 2: Design Docs — Titles + Frontmatter + NFR Sections

**Files:** All 16 existing `docs/design/*.md`

### Task 2: Clean design doc titles

Five docs have noisy titles. Fix only those five:

- [ ] **Step 1: Fix `08-benchmarking.md` title**

Change: `# Benchmarking Design` → `# Benchmarking`

- [ ] **Step 2: Fix `11-doc-doctor.md` — rename file and title**

```bash
cd docs/design
git mv 11-doc-doctor.md 11-doc-tools.md
```
Change title: `# Doc Doctorter` → `# Doc Tools`

- [ ] **Step 3: Fix `14-server-package.md` title**

Change: `# Architecture v2 — Server-Mediated Inference, Package Split` → `# Server Package`

- [ ] **Step 4: Fix `15-package-adapters.md` title**

Change: `# Package Adapters — Design` → `# Package Adapters`

- [ ] **Step 5: Fix `16-local-model-indexer.md` title**

Change: `# Local Model Indexer — Design` → `# Local Model Indexer`

- [ ] **Step 6: Verify all design titles are clean**

```bash
for f in docs/design/*.md; do echo "$(basename $f): $(head -1 $f)"; done
```
Expected: every file starts with `# <Name>` — no numbers, no ` — Design` suffixes.

### Task 3: Add frontmatter to all existing design docs

Add YAML frontmatter as the first block in each design doc. The `implements` field maps to feature ids and item ids from the traceability schema.

- [ ] **Step 1: Add frontmatter to `01-architecture.md`**

```yaml
---
id: architecture
type: design
implements: []
---
```
(Architecture is a meta/structural doc — no direct feature item mapping.)

- [ ] **Step 2: Add frontmatter to `02-skills.md`**

```yaml
---
id: skills
type: design
implements: []
---
```

- [ ] **Step 3: Add frontmatter to `03-mcp-server.md`**

```yaml
---
id: mcp-server
type: design
implements:
  - feature: indexing
    items: [llmspec-generation, symbol-map, multi-modal-search, symbol-graph]
  - feature: context
    items: [targeted-slice-loading, recommend-next]
  - feature: workflow
    items: [mcp-offload, session-resume, session-checkpoint]
  - feature: traceability
    items: [on-demand-reporting]
---
```

- [ ] **Step 4: Add frontmatter to `04-llmspec.md`**

```yaml
---
id: llmspec
type: design
implements:
  - feature: indexing
    items: [llmspec-generation]
---
```

- [ ] **Step 5: Add frontmatter to `05-indexing.md`**

```yaml
---
id: indexer
type: design
implements:
  - feature: indexing
    items: [repo-scanner, symbol-map, llms-txt-generation, multi-modal-search, symbol-graph]
---
```

- [ ] **Step 6: Add frontmatter to `06-compression.md`**

```yaml
---
id: resolution-levels
type: design
implements:
  - feature: resolution
    items: [resolution-levels, docstring-stripping, logic-flow-notation, io-pattern-notation, task-to-level-mapping]
---
```

- [ ] **Step 7: Add frontmatter to `07-drift.md`**

```yaml
---
id: drift-detector
type: design
implements:
  - feature: traceability
    items: [git-change-detection, drift-cross-reference, on-demand-reporting, pre-commit-hook, ci-integration]
---
```

- [ ] **Step 8: Add frontmatter to `08-benchmarking.md`**

```yaml
---
id: benchmarking-runner
type: design
implements:
  - feature: benchmarking
    items: [task-corpus, ab-evaluation, metrics-collection, results-comparison, cli-prompt-comparison, improvement-loop]
---
```

- [ ] **Step 9: Add frontmatter to `09-cli.md`**

```yaml
---
id: cli
type: design
implements:
  - feature: cli
    items: [repo-setup, profile-management, company-profile, context-switching, shared-library-cache, migration, pre-commit-hook, guidelines]
---
```

- [ ] **Step 10: Add frontmatter to `10-project-memory.md`**

```yaml
---
id: project-memory
type: design
implements:
  - feature: workflow
    items: [session-resume, decision-capture, pattern-capture, session-checkpoint, open-items]
  - feature: context
    items: [checkpoint-restore]
---
```

- [ ] **Step 11: Add frontmatter to `11-doc-tools.md`**

```yaml
---
id: doc-tools
type: design
implements:
  - feature: documentation
    items: [doctor-single-doc, doctor-directory, template-auto-detection, doctor-rules]
---
```
Note: `doc-guide`, `find-doc`, `new-feature-scaffold`, `external-doc-reference` will be added when the doc is extended in Chunk 3.

- [ ] **Step 12: Add frontmatter to `12-incremental-indexing.md`**

```yaml
---
id: incremental-indexer
type: design
implements:
  - feature: indexing
    items: [incremental-indexing, force-full-reindex, index-summary]
---
```

- [ ] **Step 13: Add frontmatter to `13-traceability-matrix.md`**

```yaml
---
id: traceability-matrix
type: design
implements:
  - feature: traceability
    items: [coverage-declaration, auto-detection]
---
```

- [ ] **Step 14: Add frontmatter to `14-server-package.md`**

```yaml
---
id: server-package
type: design
implements: []
---
```

- [ ] **Step 15: Add frontmatter to `15-package-adapters.md`**

```yaml
---
id: package-adapters
type: design
implements:
  - feature: indexing
    items: [repo-scanner]
---
```

- [ ] **Step 16: Add frontmatter to `16-local-model-indexer.md`**

```yaml
---
id: local-model-indexer
type: design
implements:
  - feature: indexing
    items: [multi-modal-search]
---
```

- [ ] **Step 17: Verify all design docs have frontmatter**

```bash
for f in docs/design/*.md; do echo "$(basename $f): $(head -1 $f)"; done
```
Expected: every doc starts with `---`.

### Task 4: Add NFR sections to all existing design docs

Each design doc gets a `## Non-Functional Requirements` section after the Overview. The NFRs come from the traceability matrix for the features it implements.

Add this section to each doc. Use the NFR list below per design doc.

- [ ] **Step 1: Add NFRs to `01-architecture.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| maintainability | Component boundaries must be clear enough that any module can be replaced without changing others |
| scalability | Architecture must support adding new language adapters without modifying core indexer |
```

- [ ] **Step 2: Add NFRs to `02-skills.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| maintainability | Skill format must be parseable by agents without special tooling |
| token-efficiency | Skill content must be loadable in under 500 tokens for typical skills |
```

- [ ] **Step 3: Add NFRs to `03-mcp-server.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| performance | Each MCP tool call must return in under 200ms for indexed repos |
| reliability | MCP server must handle malformed input without crashing |
| token-efficiency | Tool responses must include only requested data — no over-fetching |
```

- [ ] **Step 4: Add NFRs to `04-llmspec.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| token-efficiency | Full LLMSpec must be under 600 tokens for a typical project |
| accuracy | Auto-populated fields must correctly reflect the actual repo structure |
| maintainability | Manual edits must survive re-index without being overwritten |
```

- [ ] **Step 5: Add NFRs to `05-indexing.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| performance | Full scan of a 500-file repo must complete in under 30s |
| accuracy | Symbol extraction must correctly capture all exported functions and their signatures |
| scalability | Index artifacts must not grow unbounded — incremental runs update only changed entries |
| reliability | Index must be consistent after interrupted runs (no partial/corrupt state) |
```

- [ ] **Step 6: Add NFRs to `06-compression.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| token-efficiency | L0 output must be under 100 tokens for a typical module |
| accuracy | L2 logic flow must faithfully represent all branches and side effects |
| performance | Level serving must add under 10ms overhead vs raw file read |
```

- [ ] **Step 7: Add NFRs to `07-drift.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | No false positives — docs not linked to changed files must not be flagged |
| reliability | git fallback (mtime/size) must work when .git/ is absent |
| performance | Drift check on a 100-doc repo must complete in under 5s |
```

- [ ] **Step 8: Add NFRs to `08-benchmarking.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Metrics (tokens, turns, tool calls) must be captured without sampling error |
| reproducibility | Same task run twice in the same config must produce comparable results |
| reliability | Failed task runs must be recorded, not silently skipped |
```

- [ ] **Step 9: Add NFRs to `09-cli.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| usability | `skills init` must complete setup with no more than 3 user prompts |
| reliability | All CLI commands must be idempotent — running twice must be safe |
| performance | CLI commands must respond in under 2s for typical repos |
```

- [ ] **Step 10: Add NFRs to `10-project-memory.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| reliability | No decision or pattern must be silently lost during checkpoint |
| token-efficiency | `get_session_context()` must stay under 400 tokens regardless of history length |
| maintainability | Checkpoint format must be human-readable YAML, not binary |
```

- [ ] **Step 11: Add NFRs to `11-doc-tools.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Doc doctor must preserve 100% of existing content — no silent drops |
| usability | Template auto-detection must require zero user input for standard paths |
| maintainability | Templates must be editable plain markdown |
```

- [ ] **Step 12: Add NFRs to `12-incremental-indexing.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| performance | Incremental run touching 5 files must complete in under 3s |
| reliability | Deleted files must be fully removed from all index artifacts |
| accuracy | Incremental results must be identical to a full rescan of the same state |
```

- [ ] **Step 13: Add NFRs to `13-traceability-matrix.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Manual coverage entries must never be overwritten by auto-detection |
| reliability | Matrix must remain valid (no orphan entries) after file renames |
| performance | Matrix generation must add under 5s to a full index run |
```

- [ ] **Step 14: Add NFRs to `14-server-package.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| scalability | Server package must support concurrent requests from multiple agents |
| reliability | Server must restart cleanly after crash without index corruption |
```

- [ ] **Step 15: Add NFRs to `15-package-adapters.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Adapter must correctly identify all source files and skip noise (node_modules, dist) |
| scalability | Adapter must handle repos with 10,000+ files without memory exhaustion |
```

- [ ] **Step 16: Add NFRs to `16-local-model-indexer.md`**

```markdown
## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Semantic search results must have ≥80% relevance for natural language queries |
| performance | Embedding generation must not block the main index run by more than 2x |
| reliability | Missing or unavailable local model must fall back to regex-based indexing |
```

- [ ] **Step 17: Commit**

```bash
git add docs/design/
git commit -m "docs(design): clean titles, add frontmatter and NFR sections to all design docs"
```

---

## Chunk 3: New Design Docs

**Files to create:**
- `docs/design/17-pattern-store.md`
- `docs/design/18-response-cache.md`
- `docs/design/19-context-manager.md`
- Extend: `docs/design/11-doc-tools.md`

### Task 5: Create `17-pattern-store.md`

- [ ] **Step 1: Create the file**

`docs/design/17-pattern-store.md`:

```markdown
---
id: pattern-store
type: design
implements:
  - feature: patterns
    items: [pattern-detection, pattern-templates, pattern-capture, pattern-to-skill-export, pattern-search]
---

# Pattern Store

## Overview

The pattern store detects, records, and serves recurring structural patterns from code and design docs. It exposes MCP tools for agents to capture new patterns, search existing ones, and export them as local repo skills. Pattern detection runs as part of the index pass; manual capture and search are on-demand.

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Auto-detected patterns must have 2+ usages — no one-offs |
| maintainability | Patterns file must be human-editable plain markdown |
| reliability | Manually captured patterns must survive re-index without loss |

---

## Data Model

Patterns are stored in `.sensei/patterns.md` as a flat list of named entries:

```
## <pattern-name>

**Description:** <what the pattern is>
**When to use:** <context>
**When NOT to use:** <anti-cases>
**Example files:** `src/path/a.ts`, `src/path/b.ts`
**Source:** auto-detected | manual | design-doc:<path>
```

The pattern name is the stable id used by `find_pattern()` and `add_pattern()`.

---

## Detection Algorithm

Run during `reindex_repo()` after symbol extraction:

```
Step 1: Collect file layouts
  → For each directory: record {dir, files[], exportedSymbols[]}
  → Group directories by structural similarity (same file names, same export shapes)

Step 2: Threshold filter
  → Keep only groups with 2+ matching directories

Step 3: Name inference
  → Use LLM to infer a short pattern name from the shared structure

Step 4: Merge with existing
  → If pattern name already in .sensei/patterns.md: update example files, keep description
  → If new: append entry with TODO description

Step 5: Doc scan
  → For each design doc: extract headings that describe patterns (adapter, strategy, command, observer)
  → Add doc-sourced entries with source: design-doc:<path>
```

---

## MCP Tool Contracts

```typescript
add_pattern(name: string, description: string): void
// Appends or updates pattern in .sensei/patterns.md
// throws: never — always succeeds

find_pattern(query: string): PatternResult | null
// Semantic search over pattern names and descriptions
// returns: { name, description, whenToUse, whenNotToUse, exampleFiles } | null

list_patterns(): PatternSummary[]
// Returns all patterns as { name, description } — no example files
// token cost: under 200 tokens for up to 20 patterns
```

---

## Pattern-to-Skill Export

```
sensei pattern export <name>

Step 1: Load pattern from .sensei/patterns.md
Step 2: LLM prompt: "Convert this pattern into a skill file..."
Step 3: Write to skills/<name>/SKILL.md
Step 4: Report: "Skill created at skills/<name>/SKILL.md"
```

Update flag (`--update`): overwrites existing SKILL.md. Without flag: errors if file exists.

---

## Error Handling

```
Pattern not found:     "Pattern '<name>' not found. Run list_patterns() to see available."
Duplicate on add:      Silently merges — updates timestamp, keeps description
Export without skill:  "No SKILL.md found for '<name>'. Use sensei pattern export to create one."
```

---

## Testing Strategy

```
Unit: src/patterns/store.spec.ts
  - add_pattern creates entry in patterns.md
  - add_pattern deduplicates on same name
  - find_pattern returns null for unknown query
  - list_patterns stays under 200 tokens

E2E: e2e/patterns.e2e.ts
  - Full index on a repo with repeated structure detects pattern
  - Manual pattern survives re-index
  - sensei pattern export creates valid SKILL.md
```
```

- [ ] **Step 2: Verify frontmatter is first and implements matches feature items**

Read the file and confirm `implements.items` matches the 5 items in `09-patterns.md`.

### Task 6: Create `18-response-cache.md`

- [ ] **Step 1: Create the file**

`docs/design/18-response-cache.md`:

```markdown
---
id: response-cache
type: design
implements:
  - feature: caching
    items: [response-capture, response-retrieval, cache-management, session-integration]
---

# Response Cache

## Overview

The response cache persists notable Claude outputs — comparisons, analyses, design decisions — to `.index/response-cache/` and makes them retrievable by semantic query across sessions. Cache entries are tagged with metadata on write and expire via TTL unless pinned. Session context hints surface relevant cached items without loading full content.

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| reliability | No cached response must be lost during a write that is interrupted mid-way |
| token-efficiency | `find_cached_response()` must return only the matched entry — not the full cache |
| security | Cached responses must not be shared across repos unless explicitly configured |

---

## Data Model

Cache entries stored as individual YAML files in `.index/response-cache/<id>.yaml`:

```yaml
id: uuid-v4
label: "indexing approach comparison"        # user-supplied or auto-generated
created: 2026-03-11T14:00:00Z
last_retrieved: 2026-03-11T14:00:00Z
retrieval_count: 3
ttl_days: 90
pinned: false
topic_tags: [indexing, trade-offs, comparison]
summary: "Compared cocoindex vs sensei indexer: sensei has better coverage..."
prompt: "Which indexing approach did we evaluate?"
content: |
  [full Claude response text]
```

Cache index at `.index/response-cache/index.yaml`:
```yaml
entries:
  - id: uuid
    label: "..."
    summary: "..."
    topic_tags: [...]
    created: ...
    ttl_expires: ...
    pinned: false
```

---

## MCP Tool Contracts

```typescript
cache_response(label: string, content: string): CacheEntry
// Saves response to .index/response-cache/<id>.yaml
// Auto-generates topic tags and summary via LLM
// Updates index.yaml
// returns: { id, label, summary }

find_cached_response(query: string): CacheEntry | null
// Semantic search over index.yaml summaries and topic_tags
// Loads full entry only on match — index.yaml used for scoring
// returns: full CacheEntry or null

list_cached_responses(): CacheSummary
// Returns { count, topics[], oldest, newest } from index.yaml
// Does not load full entries
// token cost: under 100 tokens
```

---

## TTL and Maintenance

```
sensei cache gc

Step 1: Load index.yaml
Step 2: For each entry where pinned=false AND last_retrieved > ttl_days ago:
  → Move entry file to .index/response-cache/archived/<id>.yaml
  → Remove from index.yaml
Step 3: Report: "Archived N entries. M entries remain active."

TTL extension on retrieval:
  → Each find_cached_response() hit updates last_retrieved and resets TTL window
```

---

## Session Integration

`get_session_context()` includes at most 3 cache hints:

```
Step 1: Load index.yaml
Step 2: Score entries by tag overlap with current task context
Step 3: Return top 3 as: { label, summary, retrieval_command }
Step 4: Do NOT load full entry content — hints only
```

---

## Error Handling

```
Cache dir missing:     Auto-create .index/response-cache/ on first write
Corrupted entry file:  Skip and log warning — do not crash
Query no match:        Return null — do not error
Pinned entry on gc:    Skip silently
```

---

## Testing Strategy

```
Unit: src/cache/response-cache.spec.ts
  - cache_response creates entry file and updates index
  - find_cached_response returns null for unknown query
  - list_cached_responses stays under 100 tokens
  - gc removes expired entries, keeps pinned

E2E: e2e/cache.e2e.ts
  - Full session: cache → retrieve → confirm content intact
  - TTL extension: retrieve twice, confirm last_retrieved updated
  - Pin: gc does not archive pinned entry
```
```

- [ ] **Step 2: Verify frontmatter implements matches `10-caching.md` items**

### Task 7: Create `19-context-manager.md`

- [ ] **Step 1: Create the file**

`docs/design/19-context-manager.md`:

```markdown
---
id: context-manager
type: design
implements:
  - feature: context
    items: [targeted-slice-loading, token-budget-reporting, checkpoint-restore, recommend-next]
  - feature: workflow
    items: [session-orientation, targeted-context-loading, mcp-offload, task-transition, plan-implementation-efficiency, analysis-gate]
---

# Context Manager

## Overview

The context manager controls what an agent loads and when. It serves named context slices (orientation, patterns, module exports) at token-efficient resolutions, reports budget estimates before loading, prescribes the minimal context for a given task via `recommend_next`, and creates in-session checkpoints to mark task boundaries. It is the runtime complement to project memory — project memory persists across sessions; the context manager governs what is active within one.

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| token-efficiency | `load_context("orientation")` must stay under 250 tokens |
| token-efficiency | `recommend_next()` prescription must stay under 150 tokens |
| reliability | Checkpoint creation must be atomic — no partial writes |
| performance | Any context slice must be served in under 100ms |

---

## Slice Definitions

Named slices served by `load_context(scope)`:

| Scope | Content | Max tokens |
|-------|---------|-----------|
| `"orientation"` | Project name, description, stack, entry points | 250 |
| `"patterns"` | `.sensei/patterns.md` content | 300 |
| `"shortcuts"` | `.index/shortcuts.md` content | 100 |
| `"src/<module>"` | L0 exports for all files under that path | 150 |
| `"<file.ts>"` | File at prescribed resolution level | varies |

---

## MCP Tool Contracts

```typescript
load_context(scope: string, level?: "L0"|"L1"|"L2"|"L3"): ContextSlice
// Returns content for the named scope at the given level (default: L0 for modules)
// Includes token estimate in response
// throws: ScopeNotFoundError if scope doesn't match any known path or name

get_context_summary(): ScopeSummary[]
// Lists available scopes with estimated token cost
// Allows agent to compare costs before loading
// token cost: under 100 tokens

recommend_next(task: string): ContextPrescription
// Maps task description to minimal context slice
// Returns: { actions: [{ tool, args, rationale }] }
// Discovery task → list_exports at L0
// Understanding task → get_file_context at L1/L2
// Edit task → get_file_context at L3 + adjacent modules at L0

checkpoint(note?: string): CheckpointResult
// Saves named in-session boundary marker
// note: optional context summary for the checkpoint
// returns: { name, timestamp }
// Does NOT archive to project memory — use project-memory for cross-session persistence
```

---

## recommend_next Task Mapping

```
Task contains "list", "what", "find", "show" → L0 prescription
Task contains "explain", "understand", "how does" → L1/L2 prescription
Task contains "fix", "add", "change", "update", "implement" → L3 for target + L0 for context
Task unclear or broad → orientation prescription (get_llmspec)
```

---

## Workflow Gate: Analysis Before Implementation

`recommend_next` enforces the analysis gate when it detects an implementation task:

```
Step 1: Detect implementation intent (fix, add, implement, change)
Step 2: Check if get_impact_summary() has been called for this task
Step 3: If not: return prescription that STARTS with get_impact_summary()
         before any L3 loads
Step 4: get_impact_summary(symbol) queries symbol-graph for callers/dependents
         and returns blast radius before any code is written
```

---

## In-Session Checkpoint Protocol

```
checkpoint("auth module done"):
  1. Record timestamp + note to .index/checkpoints/session-current.yaml
  2. Append to checkpoint history (ring buffer, last 10 entries)
  3. Return: "Checkpointed. Resume with get_session_context()."

On task switch:
  1. Agent calls checkpoint(note) — marks end of prior task
  2. Agent calls recommend_next(new_task) — gets fresh context prescription
  3. Agent does NOT re-use context loaded for prior task
```

---

## Error Handling

```
Unknown scope:       "Scope '<scope>' not found. Call get_context_summary() to see available scopes."
Budget exceeded:     Warning in response: "This slice is N tokens — consider narrowing scope."
Checkpoint failure:  Log error, return error message — do not crash agent session
```

---

## Testing Strategy

```
Unit: src/context/context-manager.spec.ts
  - load_context("orientation") returns under 250 tokens
  - load_context("src/nonexistent") throws ScopeNotFoundError
  - recommend_next("fix the bug") returns L3 prescription
  - recommend_next("list functions") returns L0 prescription
  - checkpoint() creates entry in session-current.yaml

E2E: e2e/context.e2e.ts
  - Full session: orient → load module → recommend_next → load prescribed → checkpoint
  - Analysis gate: recommend_next on impl task returns get_impact_summary first
```
```

- [ ] **Step 2: Verify `implements` covers all items from `05-context.md` and the 6 workflow items**

### Task 8: Extend `11-doc-tools.md` with missing sections

The file currently covers only doc-doctor features. Add sections for `doc-guide`, `find-doc`, `new-feature-scaffold`, and `external-doc-reference`.

- [ ] **Step 1: Update frontmatter to include all documentation feature items**

```yaml
---
id: doc-tools
type: design
implements:
  - feature: documentation
    items: [doc-guide, find-doc, new-feature-scaffold, external-doc-reference, doctor-single-doc, doctor-directory, template-auto-detection, doctor-rules]
---
```

- [ ] **Step 2: Add `find_doc` tool contract section**

Append to `11-doc-tools.md`:

```markdown
---

## find_doc Tool Contract

```typescript
find_doc(query: string): DocResult | null
// With MCP: semantic search over indexed doc frontmatter and llms.txt
// Without MCP fallback:
//   1. Glob docs/**/*.md
//   2. Grep frontmatter `id:` fields for keyword match
//   3. Grep doc titles and first paragraph for keyword match
// returns: { path, id, type, title, summary } | null
```

---

## New Feature Scaffold

```
sensei doc new "<Module Name>"

Step 1: Slug = kebab-case(Module Name)
Step 2: NN_feature = next available number in docs/features/
Step 3: NN_design  = next available number in docs/design/ for target layer
Step 4: Create docs/features/NN-<slug>.md from feature-template.md
Step 5: Create docs/design/NN-<slug>.md from design-template.md
Step 6: Add entries to docs/traceability.yaml (features + design sections)
Step 7: Update docs/features/README.md module list
Step 8: Report: "Created docs/features/NN-<slug>.md and docs/design/NN-<slug>.md"

Error: File exists → "NN-<slug>.md already exists. Use sensei doctor to update it."
```

---

## External Doc Reference

External docs cached in `.index/doc-refs/<slug>.md` with metadata sidecar `.index/doc-refs/<slug>.meta.yaml`:

```yaml
url: https://docs.anthropic.com/api/messages
fetched: 2026-03-11T14:00:00Z
ttl_days: 7
```

```typescript
fetch_doc_ref(query: string): DocRefResult
// 1. Slug = kebab-case(query)
// 2. Check .index/doc-refs/<slug>.meta.yaml for TTL
// 3. If fresh: return .index/doc-refs/<slug>.md
// 4. If stale or missing: fetch from web, cache, return
// throws: FetchError if network unavailable and no cache

search_doc_refs(query: string): DocRefMatch[]
// Grep across all .index/doc-refs/*.md for keyword match
// returns: [{ url, section, excerpt }]
```
```

- [ ] **Step 3: Commit**

```bash
git add docs/design/
git commit -m "docs(design): add pattern-store, response-cache, context-manager; extend doc-tools"
```

---

## Chunk 4: Create `docs/traceability.yaml`

**File:** `docs/traceability.yaml` (new)

### Task 9: Write the full traceability matrix

- [ ] **Step 1: Create `docs/traceability.yaml`**

```yaml
version: 1

# ─────────────────────────────────────────────────────────────
# FEATURES
# Status values: planned | in-progress | done | deferred
# ─────────────────────────────────────────────────────────────

features:

  indexing:
    doc: docs/features/01-indexing.md
    title: Indexing
    nfrs: [performance, reliability, accuracy, scalability]
    items:
      - id: repo-scanner
        section: "#repo-scanner"
        status: planned
      - id: llmspec-generation
        section: "#llmspec-generation"
        status: planned
      - id: symbol-map
        section: "#symbol-map"
        status: planned
      - id: llms-txt-generation
        section: "#llms-txt-generation"
        status: planned
      - id: multi-modal-search
        section: "#multi-modal-search"
        status: planned
      - id: symbol-graph
        section: "#symbol-graph"
        status: planned
      - id: incremental-indexing
        section: "#incremental-indexing"
        status: done
      - id: force-full-reindex
        section: "#force-full-reindex"
        status: planned
      - id: index-summary
        section: "#index-summary-output"
        status: planned

  resolution:
    doc: docs/features/02-resolution.md
    title: Resolution
    nfrs: [token-efficiency, accuracy, performance]
    items:
      - id: resolution-levels
        section: "#resolution-levels"
        status: planned
      - id: docstring-stripping
        section: "#docstring-and-comment-stripping"
        status: planned
      - id: logic-flow-notation
        section: "#logic-flow-notation"
        status: planned
      - id: io-pattern-notation
        section: "#io-pattern-notation"
        status: planned
      - id: task-to-level-mapping
        section: "#task-to-level-mapping"
        status: planned

  workflow:
    doc: docs/features/03-workflow.md
    title: Workflow
    nfrs: [reliability, token-efficiency, maintainability]
    items:
      - id: session-orientation
        section: "#session-orientation-protocol"
        status: planned
      - id: targeted-context-loading
        section: "#targeted-context-loading"
        status: planned
      - id: mcp-offload
        section: "#mcp-offload-protocol"
        status: planned
      - id: task-transition
        section: "#task-transition-protocol"
        status: planned
      - id: plan-implementation-efficiency
        section: "#plan-to-implementation-efficiency"
        status: planned
      - id: analysis-gate
        section: "#analysis-before-implementation-gate"
        status: planned
      - id: session-resume
        section: "#session-resume"
        status: planned
      - id: decision-capture
        section: "#decision-capture"
        status: planned
      - id: pattern-capture
        section: "#pattern-capture"
        status: planned
      - id: session-checkpoint
        section: "#session-checkpoint"
        status: planned
      - id: open-items
        section: "#open-items-tracking"
        status: planned

  traceability:
    doc: docs/features/04-traceability.md
    title: Traceability
    nfrs: [accuracy, reliability, performance]
    items:
      - id: coverage-declaration
        section: "#declare-coverage-in-llmspec"
        status: planned
      - id: auto-detection
        section: "#auto-detection-from-doc-content"
        status: planned
      - id: git-change-detection
        section: "#git-based-change-detection"
        status: planned
      - id: drift-cross-reference
        section: "#precise-drift-cross-reference"
        status: planned
      - id: on-demand-reporting
        section: "#on-demand-drift-reporting"
        status: planned
      - id: pre-commit-hook
        section: "#pre-commit-hook-integration"
        status: planned
      - id: ci-integration
        section: "#ci-integration"
        status: planned

  context:
    doc: docs/features/05-context.md
    title: Context
    nfrs: [token-efficiency, reliability, performance]
    items:
      - id: targeted-slice-loading
        section: "#targeted-slice-loading"
        status: planned
      - id: token-budget-reporting
        section: "#token-budget-reporting"
        status: planned
      - id: checkpoint-restore
        section: "#checkpoint-and-restore"
        status: planned
      - id: recommend-next
        section: "#recommend_next"
        status: planned

  benchmarking:
    doc: docs/features/06-benchmarking.md
    title: Benchmarking
    nfrs: [accuracy, reproducibility, reliability]
    items:
      - id: task-corpus
        section: "#task-corpus"
        status: planned
      - id: ab-evaluation
        section: "#ab-evaluation"
        status: planned
      - id: metrics-collection
        section: "#metrics-collection"
        status: planned
      - id: results-comparison
        section: "#results-comparison"
        status: planned
      - id: cli-prompt-comparison
        section: "#cli-prompt-comparison"
        status: planned
      - id: improvement-loop
        section: "#improvement-loop"
        status: planned

  cli:
    doc: docs/features/07-cli.md
    title: CLI
    nfrs: [usability, reliability, performance]
    items:
      - id: repo-setup
        section: "#repo-setup"
        status: planned
      - id: profile-management
        section: "#profile-management"
        status: planned
      - id: company-profile
        section: "#company-profile-management"
        status: planned
      - id: context-switching
        section: "#context-switching"
        status: planned
      - id: shared-library-cache
        section: "#shared-library-cache"
        status: planned
      - id: migration
        section: "#migration-from-agents-folder"
        status: planned
      - id: pre-commit-hook
        section: "#pre-commit-hook"
        status: planned
      - id: guidelines
        section: "#guidelines-editability"
        status: planned

  documentation:
    doc: docs/features/08-documentation.md
    title: Documentation
    nfrs: [accuracy, maintainability, usability]
    items:
      - id: doc-guide
        section: "#doc-guide"
        status: planned
      - id: find-doc
        section: "#find-doc"
        status: planned
      - id: new-feature-scaffold
        section: "#new-feature-scaffold"
        status: planned
      - id: external-doc-reference
        section: "#external-doc-reference"
        status: planned
      - id: doctor-single-doc
        section: "#doctor-a-single-doc"
        status: planned
      - id: doctor-directory
        section: "#doctor-a-directory"
        status: planned
      - id: template-auto-detection
        section: "#template-auto-detection"
        status: planned
      - id: doctor-rules
        section: "#doctor-rules"
        status: planned

  patterns:
    doc: docs/features/09-patterns.md
    title: Patterns
    nfrs: [accuracy, maintainability, reliability]
    items:
      - id: pattern-detection
        section: "#pattern-detection"
        status: planned
      - id: pattern-templates
        section: "#pattern-templates"
        status: planned
      - id: pattern-capture
        section: "#pattern-capture"
        status: planned
      - id: pattern-to-skill-export
        section: "#pattern-to-skill-export"
        status: planned
      - id: pattern-search
        section: "#pattern-search"
        status: planned

  caching:
    doc: docs/features/10-caching.md
    title: Caching
    nfrs: [reliability, token-efficiency, security]
    items:
      - id: response-capture
        section: "#response-capture"
        status: planned
      - id: response-retrieval
        section: "#response-retrieval"
        status: planned
      - id: cache-management
        section: "#cache-management"
        status: planned
      - id: session-integration
        section: "#integration-with-session-resume"
        status: planned

# ─────────────────────────────────────────────────────────────
# DESIGN
# Each entry declares which feature items it implements
# ─────────────────────────────────────────────────────────────

design:

  architecture:
    doc: docs/design/01-architecture.md
    title: Architecture
    implements: []

  skills:
    doc: docs/design/02-skills.md
    title: Skills
    implements: []

  mcp-server:
    doc: docs/design/03-mcp-server.md
    title: MCP Server
    implements:
      - feature: indexing
        items: [llmspec-generation, symbol-map, multi-modal-search, symbol-graph]
      - feature: context
        items: [targeted-slice-loading, recommend-next]
      - feature: workflow
        items: [mcp-offload, session-resume, session-checkpoint]
      - feature: traceability
        items: [on-demand-reporting]

  llmspec:
    doc: docs/design/04-llmspec.md
    title: LLMSpec
    implements:
      - feature: indexing
        items: [llmspec-generation]

  indexer:
    doc: docs/design/05-indexing.md
    title: Indexer
    implements:
      - feature: indexing
        items: [repo-scanner, symbol-map, llms-txt-generation, multi-modal-search, symbol-graph]

  resolution-levels:
    doc: docs/design/06-compression.md
    title: Resolution Levels
    implements:
      - feature: resolution
        items: [resolution-levels, docstring-stripping, logic-flow-notation, io-pattern-notation, task-to-level-mapping]

  drift-detector:
    doc: docs/design/07-drift.md
    title: Drift Detector
    implements:
      - feature: traceability
        items: [git-change-detection, drift-cross-reference, on-demand-reporting, pre-commit-hook, ci-integration]

  benchmarking-runner:
    doc: docs/design/08-benchmarking.md
    title: Benchmarking Runner
    implements:
      - feature: benchmarking
        items: [task-corpus, ab-evaluation, metrics-collection, results-comparison, cli-prompt-comparison, improvement-loop]

  cli:
    doc: docs/design/09-cli.md
    title: CLI
    implements:
      - feature: cli
        items: [repo-setup, profile-management, company-profile, context-switching, shared-library-cache, migration, pre-commit-hook, guidelines]

  project-memory:
    doc: docs/design/10-project-memory.md
    title: Project Memory
    implements:
      - feature: workflow
        items: [session-resume, decision-capture, pattern-capture, session-checkpoint, open-items]
      - feature: context
        items: [checkpoint-restore]

  doc-tools:
    doc: docs/design/11-doc-tools.md
    title: Doc Tools
    implements:
      - feature: documentation
        items: [doc-guide, find-doc, new-feature-scaffold, external-doc-reference, doctor-single-doc, doctor-directory, template-auto-detection, doctor-rules]

  incremental-indexer:
    doc: docs/design/12-incremental-indexing.md
    title: Incremental Indexer
    implements:
      - feature: indexing
        items: [incremental-indexing, force-full-reindex, index-summary]

  traceability-matrix:
    doc: docs/design/13-traceability-matrix.md
    title: Traceability Matrix
    implements:
      - feature: traceability
        items: [coverage-declaration, auto-detection]

  server-package:
    doc: docs/design/14-server-package.md
    title: Server Package
    implements: []

  package-adapters:
    doc: docs/design/15-package-adapters.md
    title: Package Adapters
    implements:
      - feature: indexing
        items: [repo-scanner]

  local-model-indexer:
    doc: docs/design/16-local-model-indexer.md
    title: Local Model Indexer
    implements:
      - feature: indexing
        items: [multi-modal-search]

  pattern-store:
    doc: docs/design/17-pattern-store.md
    title: Pattern Store
    implements:
      - feature: patterns
        items: [pattern-detection, pattern-templates, pattern-capture, pattern-to-skill-export, pattern-search]

  response-cache:
    doc: docs/design/18-response-cache.md
    title: Response Cache
    implements:
      - feature: caching
        items: [response-capture, response-retrieval, cache-management, session-integration]

  context-manager:
    doc: docs/design/19-context-manager.md
    title: Context Manager
    implements:
      - feature: context
        items: [targeted-slice-loading, token-budget-reporting, checkpoint-restore, recommend-next]
      - feature: workflow
        items: [session-orientation, targeted-context-loading, mcp-offload, task-transition, plan-implementation-efficiency, analysis-gate]

# ─────────────────────────────────────────────────────────────
# CODE
# Populated as implementation progresses
# ─────────────────────────────────────────────────────────────

code: {}
```

- [ ] **Step 2: Verify YAML is valid**

```bash
node -e "const yaml = require('js-yaml'); yaml.load(require('fs').readFileSync('docs/traceability.yaml', 'utf8')); console.log('valid')"
```
Expected: `valid`

- [ ] **Step 3: Spot-check coverage gaps**

Read traceability.yaml and confirm:
- Every feature item appears in at least one `design[].implements[].items` list
- `workflow.session-orientation`, `workflow.targeted-context-loading`, `workflow.mcp-offload`, `workflow.task-transition`, `workflow.plan-implementation-efficiency`, `workflow.analysis-gate` are covered by `context-manager`
- `indexing.incremental-indexing` is covered by `incremental-indexer` (status: done)

- [ ] **Step 4: Commit**

```bash
git add docs/traceability.yaml
git commit -m "docs: add traceability.yaml — full feature/design matrix with NFRs and status"
```

---

## Chunk 5: Update `doc-guide` Skill

**Files:**
- Modify: `skills/doc-guide/SKILL.md`
- Create: `skills/doc-guide/feature-template.md`
- Create: `skills/doc-guide/design-template.md`

### Task 10: Rewrite `skills/doc-guide/SKILL.md`

- [ ] **Step 1: Rewrite SKILL.md**

Replace entire contents with:

```markdown
---
name: doc-guide
description: Use when creating, updating, or locating docs in this repo. Encodes the feature/design naming split, frontmatter spec, traceability.yaml workflow, and how to find existing docs before creating new ones.
---

# Doc Guide

## The Two-Doc System

Every capability has two documents with distinct purposes:

| Doc type | Location | Naming philosophy | Answers |
|----------|----------|-------------------|---------|
| **Feature doc** | `docs/features/NN-module-name.md` | Functional — user vocabulary | What it does and why |
| **Design doc** | `docs/design/NN-component-name.md` | Implementation — engineering vocabulary | How it is built |

**Critical difference:** Feature names describe the system from the user's perspective (`indexing`, `workflow`, `caching`). Design names describe the implementation component (`indexer`, `drift-detector`, `response-cache`). They do not mirror each other.

**Rule:** Writing about user needs, scenarios, or status → feature doc. Writing about schemas, algorithms, APIs, or error handling → design doc.

---

## Naming Conventions

### Feature docs

```
docs/features/NN-module-name.md

NN  = zero-padded sequence, ordered by domain dependency (01, 02, ...)
name = hyphen-case functional name

Examples:
  01-indexing.md
  03-workflow.md
  10-caching.md
```

Title inside file: **no number** — `# Indexing`, not `# 01 Indexing`

### Design docs

```
docs/design/NN-component-name.md

NN   = zero-padded sequence, grouped by layer (see ranges below)
name = hyphen-case implementation component name

Examples:
  05-indexer.md           ← the Indexer component
  07-drift-detector.md    ← the Drift Detector component
  18-response-cache.md    ← the Response Cache component
```

Title inside file: **no number** — `# Indexer`, not `# 05 Indexer`

### Design doc numbering ranges

| Range | Layer |
|-------|-------|
| 01–09 | Core — architecture, skills, MCP server, llmspec, indexer, resolution, drift, benchmarking, CLI |
| 10–19 | Feature extensions — project memory, doc tools, incremental indexer, traceability, patterns, caching, context |
| 20–29 | Infrastructure — server package, package adapters, local model indexer |
| 30–39 | Language adapters — JS/TS, Python, Go, Rust |

---

## Frontmatter

Every doc starts with a YAML frontmatter block. This is how MCP indexes and links docs.

### Feature doc frontmatter

```yaml
---
id: indexing
type: feature
---
```

### Design doc frontmatter

```yaml
---
id: indexer
type: design
implements:
  - feature: indexing
    items: [repo-scanner, llmspec-generation, multi-modal-search]
---
```

`implements` declares which feature items this design doc covers.
All status, NFRs, and code links live in `docs/traceability.yaml` — not in frontmatter.

---

## Before Creating Any Doc

Always search first:

```
1. find_doc("<topic>")
   → If MCP available:  call find_doc("<topic>")
   → If MCP unavailable: Glob docs/**/*.md, then Grep frontmatter `id:` fields
                          and first paragraphs for keyword match
2. If found: update the existing doc
3. If not found: create using the workflow below
```

Never create a duplicate. If a doc exists for a topic, update it.

---

## Creating a New Feature Doc

```
1. find_doc("<topic>")                           ← confirm it doesn't exist
2. ls docs/features/ | sort | tail -2            ← find last NN used
3. NN = last NN + 1 (zero-padded)
4. name = hyphen-case functional module name
5. cp skills/doc-guide/feature-template.md docs/features/NN-name.md
6. Fill in: id (= name), title, problem statement, features, scenarios
7. Add entry to docs/traceability.yaml under features:
     <id>:
       doc: docs/features/NN-name.md
       title: <Title>
       nfrs: [<list quality attributes>]
       items:
         - id: <item-id>
           section: "#<section-heading>"
           status: planned
8. Update docs/features/README.md — add to module list
```

---

## Creating a New Design Doc

```
1. find_doc("<topic>")                           ← confirm it doesn't exist
2. Identify target layer range (01–09 core, 10–19 extensions, etc.)
3. ls docs/design/ | grep "^NN-" | sort | tail -1  ← find last NN in range
4. NN = last NN in range + 1
5. name = hyphen-case component name (engineering vocabulary)
6. cp skills/doc-guide/design-template.md docs/design/NN-name.md
7. Fill in: id (= name), implements (feature ids + item ids), overview, schema,
            algorithm, API contracts, NFRs, error handling, testing
8. Add entry to docs/traceability.yaml under design:
     <id>:
       doc: docs/design/NN-name.md
       title: <Title>
       implements:
         - feature: <feature-id>
           items: [<item-ids>]
9. Update docs/design/README.md
```

---

## Updating Status

**Only** update status in `docs/traceability.yaml`. Do not add status tables to docs.

```
When work starts on a feature item:
  docs/traceability.yaml → features.<id>.items[n].status: in-progress

When a feature item is complete:
  docs/traceability.yaml → features.<id>.items[n].status: done

When code is written that implements a design:
  docs/traceability.yaml → code:
    src/path/file.ts:
      implements-design: [<design-id>]
      status: in-progress
```

---

## Answering Traceability Queries

Read `docs/traceability.yaml` to answer:

| Question | How to derive |
|----------|---------------|
| Features with no design | feature ids not in any `design[].implements[].feature` |
| Designs with no code | design ids not in any `code[].implements-design` |
| What's in the backlog? | All `items` with `status: planned` |
| What's next to implement? | First `planned` item in priority feature order |
| Which design covers X? | Search `design[].implements[].items` for item-id X |
| Which feature does design Y implement? | `design.Y.implements[].feature` |

---

## What Belongs Where

**Feature doc contains:**
- Frontmatter (`id`, `type: feature`)
- One-paragraph problem statement (human audience, no implementation details)
- Gherkin scenarios (Given/When/Then) per feature section
- No status tables — status lives in traceability.yaml

**Feature doc does NOT contain:**
- Implementation details (algorithms, schemas, code snippets)
- API contracts or function signatures

**Design doc contains:**
- Frontmatter (`id`, `type: design`, `implements`)
- 2–3 sentence overview
- Non-Functional Requirements table
- Data structures, file layouts, schemas
- Algorithm/flow descriptions
- Function signatures and MCP tool contracts
- Error messages and handling
- Test strategy

**Design doc does NOT contain:**
- User-facing "why" reasoning
- Gherkin scenarios
- Status tracking

---

## Quick Reference

```
find_doc("query")                         ← find existing doc by topic
cp skills/doc-guide/feature-template.md  ← start new feature doc
cp skills/doc-guide/design-template.md   ← start new design doc
docs/traceability.yaml                    ← single source of truth for status + links
docs/features/README.md                  ← feature module index
docs/design/README.md                    ← design component index
```
```

- [ ] **Step 2: Verify skill loads correctly**

```bash
head -5 skills/doc-guide/SKILL.md
```
Expected: frontmatter block starting with `---`, then `name: doc-guide`.

### Task 11: Create `skills/doc-guide/feature-template.md`

- [ ] **Step 1: Write the template**

```markdown
---
id: <module-name>
type: feature
---

# <Module Name>

[One paragraph: what user need or pain point this module addresses, and why it matters. Human audience. No implementation details.]

## Features

### <Feature Name>

[One sentence describing what this feature does from the user's perspective.]

```gherkin
Feature: <Feature Name>

  Scenario: <Happy path — primary use case>
    Given <precondition>
    When <action>
    Then <observable outcome>
    And <secondary outcome if needed>

  Scenario: <Edge case or error case>
    Given <precondition>
    When <action>
    Then <outcome>
```

### <Feature Name>

[One sentence.]

```gherkin
Feature: <Feature Name>

  Scenario: <Description>
    Given <precondition>
    When <action>
    Then <outcome>
```

---

> This is a **feature doc** — what and why, not how.
> Implementation details belong in `docs/design/`.
> Status lives in `docs/traceability.yaml` — do not add a status table here.
```

### Task 12: Create `skills/doc-guide/design-template.md`

- [ ] **Step 1: Write the template**

```markdown
---
id: <component-name>
type: design
implements:
  - feature: <feature-id>
    items: [<item-id>, <item-id>]
---

# <Component Name>

## Overview

[2–3 sentences: what this component does, its role in the system, and the key design decision that shapes everything else.]

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| <nfr-name> | <acceptance criterion — measurable> |

---

## <Data Model / Schema / File Layout>

[File layouts, data schemas, type definitions, storage formats.]

```
example/
  structure.yaml     ← purpose
  file.ts            ← purpose
```

---

## <Algorithm / Flow / Protocol>

[Sequences, decision logic, state machines. Use pseudocode or numbered steps.]

```
Step 1: <action>
  → sub-step
Step 2: <action>
```

---

## <API / Tool Contracts>

[Function signatures, MCP tool contracts, CLI command specs.]

```typescript
functionName(param: Type): ReturnType
// param: description
// returns: description
// throws: ErrorType when X
```

---

## Error Handling

```
Missing X:  "X not found. Run Y first."
Invalid Y:  "Y is invalid. Expected Z."
```

---

## Testing Strategy

```
Unit: src/<module>/<component>.spec.ts
  - <key test case>
  - <key test case>

E2E: e2e/<feature>.e2e.ts
  - <key integration scenario>
```

---

## Open Questions

| Question | Status |
|----------|--------|
| <question> | 🔲 Open |

---

> This is a **design doc** — how it works, not what it does.
> User-facing needs belong in `docs/features/`.
> Status lives in `docs/traceability.yaml` — do not add a status table here.
```

- [ ] **Step 2: Commit**

```bash
git add skills/doc-guide/
git commit -m "docs(skill): rewrite doc-guide with naming conventions, frontmatter, traceability workflow, and templates"
```

---

## Chunk 6: Update READMEs + Validate

### Task 13: Update `docs/features/README.md`

- [ ] **Step 1: Update module list to use NN-prefixed names and add traceability reference**

Replace the Modules section with:

```markdown
## Modules

See [`docs/traceability.yaml`](../traceability.yaml) for feature items, design links, status, and NFRs.

- [01 — Indexing](01-indexing.md) — Scan, extract, orientation artifacts, incremental updates, multi-modal search, symbol graph
- [02 — Resolution](02-resolution.md) — Token-efficient code representations at four levels (L0–L3)
- [03 — Workflow](03-workflow.md) — Session protocol, project memory, decision capture, cross-session continuity
- [04 — Traceability](04-traceability.md) — Doc-to-code coverage map, git-based drift detection, CI integration
- [05 — Context](05-context.md) — Targeted slice loading, token budget, task-scoped context prescriptions
- [06 — Benchmarking](06-benchmarking.md) — A/B comparisons, metrics, CLI prompt comparison, improvement loop
- [07 — CLI](07-cli.md) — Repo setup, profile management, context switching, shared library cache
- [08 — Documentation](08-documentation.md) — Doc guide skill, find_doc, scaffold, doc-doctor, external doc refs
- [09 — Patterns](09-patterns.md) — Detect, capture, search, and export recurring patterns as local repo skills
- [10 — Caching](10-caching.md) — Persist and retrieve notable Claude responses across sessions
```

- [ ] **Step 2: Replace Feature Status table**

Remove the old detailed status table. Add:

```markdown
## Feature Status

Status is tracked in [`docs/traceability.yaml`](../traceability.yaml).

To query current status:
- Read `docs/traceability.yaml` and filter by `status: planned | in-progress | done | deferred`
- Features with no design coverage: feature ids absent from all `design[].implements[].feature` entries
- Implementation backlog: all `items` with `status: planned`
```

### Task 14: Update `docs/design/README.md`

- [ ] **Step 1: Add new design docs to the table**

Add to the Feature Extensions table:

```markdown
| [17-pattern-store](./17-pattern-store.md) | Pattern detection, capture, search, and skill export |
| [18-response-cache](./18-response-cache.md) | Response caching: capture, retrieval, TTL, session hints |
| [19-context-manager](./19-context-manager.md) | In-session context slices, token budget, recommend_next, analysis gate |
```

- [ ] **Step 2: Update 11 reference**

Change `11-doc-doctor` → `11-doc-tools` in the table.

- [ ] **Step 3: Update numbering ranges table**

```markdown
| Range | Layer |
|-------|-------|
| 01–09 | Core — architecture, skills, MCP server, llmspec, indexer, resolution, drift, benchmarking, CLI |
| 10–19 | Feature extensions — project memory, doc tools, incremental indexer, traceability, patterns, caching, context |
| 20–29 | Infrastructure — server package, package adapters, local model indexer |
| 30–39 | Language adapters — JS/TS, Python, Go, Rust |
```

- [ ] **Step 4: Add traceability reference**

Add below the Document Relationships section:

```markdown
## Traceability

Full feature→design→code mapping, status, and NFRs: [`docs/traceability.yaml`](../traceability.yaml)
```

- [ ] **Step 5: Commit**

```bash
git add docs/features/README.md docs/design/README.md
git commit -m "docs: update feature and design READMEs with NN prefixes and traceability reference"
```

### Task 15: Validate the skill works without MCP

- [ ] **Step 1: Test query — features with no design**

Read `docs/traceability.yaml`. For each feature id, check whether it appears in any `design[].implements[].feature`. Expected result: all 10 features have at least one design entry (verify `patterns`, `caching`, and `context` are now covered by the new docs).

- [ ] **Step 2: Test query — backlog count**

Count all `items` with `status: planned` across all features in `docs/traceability.yaml`. Expected: the large majority planned, with `indexing.incremental-indexing` as `done`.

- [ ] **Step 3: Test naming convention**

```bash
ls docs/features/   # should show 01-indexing.md ... 10-caching.md + README.md
ls docs/design/     # should show 01-architecture.md ... 19-context-manager.md + README.md
```

- [ ] **Step 4: Test frontmatter presence**

```bash
for f in docs/features/0*.md docs/design/0*.md docs/design/1*.md; do
  first=$(head -1 "$f")
  [ "$first" = "---" ] || echo "MISSING FRONTMATTER: $f"
done
```
Expected: no output (all files have frontmatter).

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "docs: complete doc-guide + traceability system implementation"
```
