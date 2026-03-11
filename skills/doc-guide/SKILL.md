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
   → If MCP available:    call find_doc("<topic>")
   → If MCP unavailable:  Glob docs/**/*.md, then Grep frontmatter `id:` fields
                          and first paragraphs for keyword match
2. If found: update the existing doc
3. If not found: create using the workflow below
```

Never create a duplicate. If a doc exists for a topic, update it.

---

## Creating a New Feature Doc

```
1. find_doc("<topic>")                           ← confirm it doesn't exist
2. ls docs/features/ | grep -E '^[0-9]' | sort | tail -1  ← find last NN used
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
3. ls docs/design/ | grep -E '^[0-9]' | sort | tail -1  ← find last NN in range
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
