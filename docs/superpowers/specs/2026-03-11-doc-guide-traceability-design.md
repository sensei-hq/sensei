# Doc Guide & Traceability System — Design

**Date:** 2026-03-11
**Status:** Approved — ready for implementation
**Scope:** Items 1–3 only. MCP integration (item 4) is a separate future cycle.

---

## Objective

Establish a consistent documentation system across `docs/features/` and `docs/design/` with distinct naming philosophies, machine-readable traceability via `docs/traceability.yaml`, frontmatter linking docs to the matrix, and an updated `doc-guide` skill that agents can follow with or without MCP.

The end state: a skill + YAML combo that any agent can use to create docs correctly, maintain the traceability matrix as the system grows, and answer backlog queries without loading every file into context. Once validated on sensei, the skill + MCP combo can be applied to other repos.

---

## Naming Conventions

### Feature docs — `docs/features/NN-module-name.md`

- `NN` = zero-padded two-digit sequence, ordered by domain dependency (foundations first)
- `module-name` = hyphen-case **functional** name — user vocabulary, describes what the system does
- Title inside file: **no number** — `# Indexing`, not `# 01 Indexing`
- Examples: `01-indexing.md`, `03-workflow.md`, `09-patterns.md`

### Design docs — `docs/design/NN-component-name.md`

- `NN` = zero-padded sequence, grouped by architectural layer
- `component-name` = hyphen-case **implementation** name — engineering vocabulary, names the class/adapter/tool/utility
- Title inside file: **no number** — `# Indexer`, not `# 05 Indexer`
- Examples: `05-indexer.md`, `12-incremental-indexer.md`, `15-doc-type-adapters.md`

### Key distinction

Feature names are user-vocabulary (`indexing`, `workflow`).
Design names are implementation-vocabulary (`indexer`, `git-change-detector`, `doc-type-adapters`).
They do **not** mirror each other. One design may serve multiple features. One feature may span multiple designs.

### Numbering ranges for design docs

| Range | Layer |
|-------|-------|
| 01–09 | Core — architecture, skills, MCP server, llmspec, indexer, resolution, drift, benchmarking, CLI |
| 10–19 | Feature extensions — project memory, doc tools, incremental indexer, traceability, patterns, caching, context |
| 20–29 | Infrastructure — server package, package adapters, local model indexer |
| 30–39 | Language adapters — JS/TS, Python, Go, Rust |

---

## Frontmatter Spec

### Feature doc frontmatter

```yaml
---
id: indexing
type: feature
---
```

Minimal. `id` is the stable identifier used by `traceability.yaml`. `type` tells MCP what kind of doc this is.

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

`implements` is the only relational data in frontmatter. It declares which feature items this design doc covers. All status, NFRs, and code links live in `traceability.yaml`.

---

## Traceability YAML Schema

**Location:** `docs/traceability.yaml`

```yaml
version: 1

features:
  <id>:
    doc: docs/features/NN-name.md
    title: Human-readable title
    nfrs: [token-efficiency, reliability, accuracy, security, scalability]
    items:
      - id: item-id
        section: "#section-heading"     # anchor into the feature doc
        status: planned | in-progress | done | deferred

design:
  <id>:
    doc: docs/design/NN-name.md
    title: Human-readable title
    implements:
      - feature: <feature-id>
        items: [item-id, item-id]       # which feature items this design covers

code:
  <src/path/file.ts>:
    implements-design: [<design-id>]
    status: planned | in-progress | done
```

### NFR vs functional item distinction

- **`items`** = functional requirements — what the system does (the Gherkin scenarios)
- **`nfrs`** = quality attributes — how well it does it (token-efficiency, reliability, accuracy, security, scalability, maintainability)

`accuracy` is an NFR when it expresses a quality constraint ("search results must be ≥90% relevant"), not a feature.

### Progressive connection model

The matrix is always valid at any stage of growth:

```
Stage 1 — Feature defined:
  features.indexing exists, no design entries reference it
  → "feature with no design" query returns it

Stage 2 — Design doc added:
  design.indexer.implements references features.indexing
  → connection established, feature removed from undesigned backlog

Stage 3 — Code added:
  code.src/indexer/index.ts.implements-design references indexer
  → full chain: feature → design → code

Stage 4 — Code done:
  code entry status updated to done
  feature items status updated to done
```

### MCP-queryable states

| Query | Derivation |
|-------|-----------|
| Features with no design | `features` ids not referenced in any `design[].implements[].feature` |
| Designs with no code | `design` ids not referenced in any `code[].implements-design` |
| Feature spanning multiple designs | Same feature id in 2+ `design[].implements` entries |
| Design covering multiple features | `design[].implements` has 2+ distinct feature ids |
| Backlog (next to implement) | `items` with `status: planned`, ordered by feature priority |
| Specific section to read | `items[].section` anchor → fetch `doc` + jump to `#heading` |

---

## doc-guide Skill Structure

```
skills/doc-guide/
  SKILL.md                ← agent instructions: naming, frontmatter, workflow
  feature-template.md     ← canonical feature doc template (with frontmatter)
  design-template.md      ← canonical design doc template (with frontmatter)
```

### What the skill instructs agents to do

**Before creating any doc:**
```
1. find_doc("<topic>")
   → If MCP available: call find_doc tool
   → If not: Glob docs/**/*.md + Grep frontmatter for id matching topic
2. If found: update existing doc
3. If not found: create using template
```

**When creating a feature doc:**
```
1. Check docs/features/ for last NN used
2. Assign next NN
3. Create docs/features/NN-module-name.md from feature-template.md
4. Add frontmatter: id + type: feature
5. Add entry to docs/traceability.yaml under features:
   - doc path, title, nfrs, items with section anchors, all status: planned
6. Update docs/features/README.md module list
```

**When creating a design doc:**
```
1. Check docs/design/ for last NN in the target layer range
2. Assign next NN
3. Create docs/design/NN-component-name.md from design-template.md
4. Add frontmatter: id + type: design + implements (feature ids + item ids)
5. Add entry to docs/traceability.yaml under design:
   - doc path, title, implements list
6. Update docs/design/README.md
```

**When status changes:**
```
Update docs/traceability.yaml items[].status only.
Do NOT edit status in the doc itself — the YAML is the single source of truth for status.
```

**When code is written:**
```
Add entry to docs/traceability.yaml under code:
  src/path/file.ts:
    implements-design: [design-id]
    status: in-progress
```

---

## Scope of Work

### 1. Rename docs/features/ files — add NN prefixes

| Current | New |
|---------|-----|
| indexing.md | 01-indexing.md |
| resolution.md | 02-resolution.md |
| workflow.md | 03-workflow.md |
| traceability.md | 04-traceability.md |
| context.md | 05-context.md |
| benchmarking.md | 06-benchmarking.md |
| cli.md | 07-cli.md |
| documentation.md | 08-documentation.md |
| patterns.md | 09-patterns.md |
| caching.md | 10-caching.md |

Add frontmatter (`id`, `type: feature`) to each. Verify titles have no numbers.

### 2. Fix docs/design/ titles — remove numbers from headings

All 16 existing design docs: strip number prefix from the `# Title` line.
No file renames needed (current names are already implementation-focused).

### 3. Add missing design docs

Features currently without full design coverage:

| Feature | Missing design coverage | Proposed doc |
|---------|------------------------|--------------|
| patterns | No design doc | `17-pattern-store.md` |
| caching | No design doc | `18-response-cache.md` |
| context | Partially in 10-project-memory; in-session context missing | `19-context-manager.md` |
| documentation | 11-doc-doctor covers reformatting only; find_doc, scaffold, external refs missing | extend `11-doc-doctor.md` → rename to `11-doc-tools.md` |

Add frontmatter + `implements` to all new and existing design docs.

### 4. Add NFR sections to all design docs

Each design doc gets a `## Non-Functional Requirements` section listing the NFRs that apply (from `traceability.yaml`) with acceptance criteria.

### 5. Create docs/traceability.yaml

Fully populated for all 10 features × their design coverage × current status.
`code:` section starts empty (populated as implementation progresses).

### 6. Update skills/doc-guide/

- Rewrite `SKILL.md` with full naming conventions, frontmatter spec, workflow instructions, find_doc fallback
- Add `feature-template.md` with frontmatter placeholder
- Add `design-template.md` with frontmatter placeholder + NFR section

### 7. Update READMEs

- `docs/features/README.md` — update module list with new NN-prefixed names, add traceability.yaml reference
- `docs/design/README.md` — update any title references, add traceability.yaml reference

---

## Validation Test

After implementation, test the skill works without MCP by asking Claude to:

1. "Add a new feature doc for multi-modal retrieval" → verify it picks the right NN, adds frontmatter, updates traceability.yaml
2. "What features have no design doc?" → verify it reads traceability.yaml and returns correct answer
3. "Mark symbol-graph as in-progress" → verify it updates traceability.yaml only, not the feature doc
4. "Create a design doc for the pattern store" → verify it adds frontmatter with correct `implements`, updates traceability.yaml

---

## Future: MCP Integration (separate cycle)

Once the skill is validated:
- `find_doc(query)` — semantic search over indexed frontmatter
- `get_traceability(feature_id)` — return feature + linked design + code status in one call
- `get_backlog()` — all planned items with no code entry
- `check_drift()` — code changed, linked design doc didn't

The skill's `find_doc` fallback (Glob + Grep) is replaced by the MCP tool call. Everything else in the skill stays identical.

After MCP is ready, the skill + MCP combo can be applied to other repos (`rokkit`, etc.) via `sensei init`.
