---
name: Pattern Store
description: Detect, surface, enforce, and grow code patterns — so the AI builds on existing foundations instead of reinventing
date: 2026-04-17
status: idea
sources: design/17-pattern-store.md, features/01-codebase-intelligence.md
dependency-of: blueprints/01-workflow-engine.md (locate step, review, guardrails)
---

# Pattern Store

## Problem

The AI reinvents structure that already exists in the codebase. Adapter patterns, task worker wrappers, test fixtures — these are recurring structures that should be codified and reused. With AI churning code at a fast rate, pattern violations are easy to miss. By the time a human catches it, rework is expensive.

The solution isn't just detection — it's a full lifecycle: **detect → surface → enforce → grow**.

## Current state

- PATTERNS.md: manual, maintained by user via `/sensei:pattern-extract`
- Pattern-based development skill: exists in plugin (loads patterns before coding)
- Auto-detection during indexing: planned, not implemented (see idea 08 for phased approach)
- Pattern search/matching: not implemented
- Pattern enforcement: not implemented
- Pattern persistence across re-index: not implemented

---

## Pattern Lifecycle

```
┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
│ Detect   │ ──▶ │ Surface  │ ──▶ │ Enforce  │ ──▶ │  Grow    │
│ (index)  │     │ (MCP/UI) │     │ (build/  │     │ (feedback │
│          │     │          │     │  review)  │     │  loop)   │
└─────────┘     └─────────┘     └─────────┘     └─────────┘
  Indexer          MCP tools       Commands        Guardrails
  detects          expose to       check during    evolve from
  patterns         AI + desktop    coding + review  corrections
```

### 1. Detect (indexer)

The indexer identifies patterns during the index pass (see idea 08 for phased approach):

**What gets stored in the graph:**

```
Pattern node:
  name: "language-adapter"
  type: "adapter"                    # design pattern classification
  trait/interface: "LanguageAdapter"  # what implementations conform to
  instances: 4                       # how many implementations exist
  files: ["ts_adapter.rs", "python_adapter.rs", "rust_adapter.rs", "java_adapter.rs"]
  registration: "adapters/mod.rs"    # where instances are registered
  invariants:                        # what every implementation must do
    - "implement parse_file(path) -> Vec<Symbol>"
    - "implement resolve_imports(symbols) -> Vec<Import>"
    - "register in ADAPTERS map in mod.rs"
```

**Pattern types the indexer should recognize:**

| Category | Patterns | Detection method |
|----------|----------|-----------------|
| **Structural** | Adapter, Facade, Decorator, Proxy, Bridge | Interface + wrapper analysis |
| **Creational** | Factory, Builder, Singleton | Static constructors, builder chains |
| **Behavioral** | Observer, Strategy, Command, State | Event emitters, interchangeable implementations |
| **Project-specific** | Task worker, MCP tool handler, CLI subcommand, API route | Naming + registration pattern matching |
| **File-structural** | Module layout (mod.rs + types.rs + handlers.rs), test fixtures | Directory/file naming patterns |

### 2. Surface (MCP + desktop)

**MCP tools — what the AI sees:**

| Tool | Purpose | Example |
|------|---------|---------|
| `get_patterns(type?)` | List detected patterns, optionally filtered by type | `get_patterns("adapter")` → returns adapter pattern with instances, invariants |
| `get_pattern_for(symbol)` | "What pattern does this symbol belong to?" | `get_pattern_for("TypeScriptAdapter")` → "language-adapter (adapter pattern, 4 instances)" |
| `match_pattern(description)` | "Is there a pattern I should follow for this task?" | `match_pattern("add SQL parsing")` → "language-adapter pattern — follow TypeScriptAdapter as reference" |
| `get_similar(symbol)` | Find structurally similar code | `get_similar("parse_file")` → "4 implementations with similar signature across adapters" |
| `get_duplicates(file?)` | Find exact/near duplicates | `get_duplicates("utils.rs")` → "lines 45-60 duplicate logic in helpers.rs:20-35" |

**Desktop — what the user sees:**

| View | Content |
|------|---------|
| Pattern catalog | All detected patterns with instance count, coverage, and conformance status |
| Pattern detail | One pattern: interface, invariants, all instances, recent violations |
| Pattern coverage | Heatmap: which parts of the codebase follow patterns vs. ad-hoc |
| Duplication map | Clusters of duplicate/similar code with suggested pattern extraction |

### 3. Enforce (build + review)

This is where pattern violations get caught. Two enforcement points:

**A. During `/sensei:build` (proactive — before code is written)**

The locate step already calls `get_patterns()`. The build command instructions should include:

```markdown
## Step 2b: Check for applicable patterns

After locating relevant code, check if a pattern applies:

1. Call `match_pattern(task_description)` — does a pattern exist for this kind of work?
2. If yes: load the pattern, show the user:
   "Found pattern: language-adapter (4 instances). Following TypeScriptAdapter as reference."
   Follow the pattern exactly — same file structure, same interface, same registration.
3. If unsure: ASK the user: "I found a possible pattern match (adapter). Should I follow it, or is this case different?"
4. Log: `log_event("pattern_checked", { pattern: "language-adapter", matched: true, followed: true })`

Do NOT skip this step. If you're about to create a new file that looks similar to existing files, check for patterns first.
```

**B. During `/sensei:review` (reactive — after code is written)**

The review command should include pattern violation detection:

```markdown
## Pattern conformance check

1. Call `get_patterns()` to load all project patterns
2. For each modified file in the current changeset:
   a. Call `get_pattern_for(symbol)` for key symbols in the file
   b. If the symbol belongs to a pattern: check that the implementation follows invariants
   c. If the symbol looks similar to a pattern but doesn't follow it: flag as potential violation
3. Call `get_duplicates()` on modified files — flag any new duplication introduced
4. Report findings:
   - "✅ SqlAdapter correctly follows language-adapter pattern"
   - "⚠️ parse_sql() duplicates logic from parse_json() — consider using shared helper"
   - "❌ New file sql.rs doesn't implement LanguageAdapter trait — should it be an adapter?"
```

**C. Via `/sensei:patterns` (on-demand introspection)**

User asks "what patterns exist?" or "what pattern should I use for X?" at any time:

```markdown
## Pattern introspection

1. Call `get_patterns()` — list all detected patterns with instance counts
2. If user asks about a specific task: call `match_pattern(description)`
3. Show: pattern name, instances, invariants, reference implementation
4. If user asks "should I use pattern X?": compare task requirements against pattern invariants, advise
```

### 4. Grow (feedback loop)

Patterns evolve from corrections and new discoveries:

| Trigger | Action |
|---------|--------|
| User corrects AI: "you should have used the adapter pattern" | AI asks clarifying questions → adds to `.sensei/guardrails.md` AND indexes the correction as a pattern enforcement rule |
| `/sensei:pattern-extract` discovers a new pattern | Adds to PATTERNS.md → next index pass picks it up → becomes enforceable |
| `/sensei:review` finds repeated violations | Suggests: "3 files violate the adapter pattern this week — should I add a guardrail?" |
| Indexer detects a new recurring structure | Surfaces as candidate pattern during `/sensei:analyze` → user confirms or dismisses |

**The guardrails ↔ patterns connection:**

Guardrails (`.sensei/guardrails.md`) and patterns (PATTERNS.md + graph) are complementary:
- **Patterns** describe WHAT the structure looks like (interface, files, registration)
- **Guardrails** describe WHEN to use them ("always use adapter pattern for language parsers")
- Pattern detection feeds guardrail growth: when the AI violates a pattern and gets corrected, the correction becomes a guardrail

---

## Impact on events and metrics

| Event | When | Data |
|-------|------|------|
| `pattern_checked` | `/sensei:build` locate step | pattern_name, matched (bool), followed (bool) |
| `pattern_violation` | `/sensei:review` | pattern_name, file, violation_type, severity |
| `pattern_extracted` | `/sensei:pattern-extract` | pattern_name, instances, source_files |
| `duplicate_found` | `/sensei:review` | file_a, file_b, similarity_score, lines |

**New metrics:**

| Metric | Formula |
|--------|---------|
| Pattern adherence | `pattern_checked` where followed=true / total pattern_checked |
| Pattern coverage | files belonging to patterns / total files |
| Duplication trend | `duplicate_found` events over time (should decrease) |
| Pattern growth | cumulative `pattern_extracted` events |

---

## Open questions

| # | Question |
|---|----------|
| 1 | How do we distinguish a "pattern" from mere repetition? Threshold: 2+ instances with shared interface/registration? |
| 2 | Should patterns be global (across projects) or project-specific? Both — project-specific by default, promotable to global. |
| 3 | How does this interact with guardrails? Patterns describe structure; guardrails describe when to use them. Complementary. |
| 4 | Should pattern violation block commits (strict mode) or just warn (advisory mode)? Configurable per project. |
| 5 | How often does the indexer refresh pattern detection? On every index, or only on demand? |
| 6 | Can semgrep custom rules serve as both pattern detector AND enforcer? (detect during index, enforce during review) |
