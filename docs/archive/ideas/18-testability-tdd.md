---
name: Testability & TDD Discipline
description: Guide the AI toward composable, testable code. Surface tests to users for verification before implementation. Ensure TDD means test-DRIVEN, not test-after.
date: 2026-04-17
status: idea
related: 15-pattern-store.md, 17-pattern-knowledge.md, 08-codebase-intelligence.md
impacts: blueprints/01-workflow-engine.md (/sensei:build command)
---

# Testability & TDD Discipline

## Problem

AI-generated code tends toward monolithic functions — it's faster to write one big function than to decompose into testable units. The AI satisfies the requirement but produces code that's hard to test, hard to modify, and hard to verify. When TDD is attempted, the AI writes tests it knows will pass — the human never sees the tests until after implementation, defeating the purpose.

### Observed failure modes

| Failure | Root cause | Cost |
|---------|-----------|------|
| Task workers were monolithic — required multiple rewrite iterations | No decomposition guidance; AI wrote the fastest path | Rework, visual graph inspection cycles |
| Tests were written after implementation, not before | TDD was a stated goal but no enforcement mechanism | Tests validate what was built, not what should be built |
| User never reviewed tests before implementation | No step in the workflow surfaces tests for approval | AI tests may test the wrong behavior |
| Functions had hidden side effects | No guidance on pure vs. side-effect separation | Hard to test, hard to compose |

---

## Principles

### 1. The function is the unit of testability

Every function should have a clear **shape**: params → purpose → returns → uses[]. If a function's shape is too complex (many params, unclear purpose, multiple side effects), it should be decomposed.

```
Shape of a testable function:
  name:    extract_symbols
  params:  (ast: AST, language: Language)
  returns: Vec<Symbol>
  uses:    [extract_functions, extract_classes, extract_types]
  side_effects: none
  testable: YES — pure function, mock-free

Shape of an untestable function:
  name:    process_file
  params:  (path: PathBuf, db: &Database, graph: &Graph, config: &Config)
  returns: Result<(), Error>
  uses:    [read_file, parse, extract, write_db, write_graph, resolve_refs, notify]
  side_effects: writes to db, graph, filesystem
  testable: NO — requires full infrastructure to test
```

### 2. Decompose before you code

Before writing implementation, decompose into:
- **Pure functions** — transform data, no side effects. Unit testable with simple assertions.
- **Orchestrators** — thin wrappers that call pure functions and handle side effects. Integration testable.
- **Adapters** — boundary code that interfaces with external systems (DB, filesystem, HTTP). Mock at this boundary.

### 3. Test-DRIVEN means tests come first AND get human approval

The TDD cycle must include a human verification step:

```
Step 1: AI writes test cases (no implementation)
Step 2: AI PRESENTS tests to user: "Here are the test cases. Do they cover the right behavior?"
Step 3: User reviews, adjusts, approves
Step 4: AI implements to make tests pass
Step 5: AI runs tests, shows results
```

Without Step 2-3, TDD degenerates into "AI writes tests it knows how to pass" — which is just test-after with extra steps.

---

## How the graph supports testability

With rich graph nodes (analysis 02), the graph can surface function shape information that guides decomposition:

### Function shape from graph

```yaml
# What get_symbol() should return (with rich nodes)
symbol: process_file
kind: function
params:
  - name: path, type: PathBuf
  - name: db, type: &Database
  - name: graph, type: &Graph
  - name: config, type: &Config
returns: Result<(), Error>
calls: [read_file, parse_ast, extract_symbols, write_to_db, write_to_graph, resolve_refs]
side_effects: [db_write, graph_write, file_read]
complexity: 12
is_exported: true
```

From this shape, the AI can **automatically identify testability issues**:
- 4 params including 2 mutable references → too many dependencies
- 6 function calls including 3 with side effects → hard to test in isolation
- Complexity 12 → should be decomposed

### MCP tools for testability

| Tool | Purpose |
|------|---------|
| `get_symbol(name, depth=1)` | Returns full shape including params, returns, calls, side_effects |
| `get_complexity(limit, min_complexity)` | Find functions that need decomposition |
| `get_callees(name)` | What does this function call? Are they pure or side-effecting? |

### Testability score (computable from graph)

For each function, compute a testability score based on:

| Factor | Good (testable) | Bad (monolith) |
|--------|-----------------|-----------------|
| Param count | 0-3 | 4+ |
| Side effects | 0 | 1+ |
| Complexity | 1-5 | 10+ |
| Dependencies (callees) | 0-3 pure functions | 4+ or any external |
| Return type | concrete value | void/unit with side effects |

This could be surfaced via MCP: `get_testability(symbol)` → score + recommendations.

---

## Impact on `/sensei:build` command

The build command's TDD cycle should include decomposition guidance and test approval:

```markdown
## /sensei:build — TDD cycle

### Step 1: Understand the task
Read the issue/plan item. Call `match_pattern()` for applicable patterns.

### Step 2: Locate relevant code
[existing locate step]

### Step 3: Design the decomposition
Before writing any code, plan the function structure:
- What pure functions are needed? (data transformations, no side effects)
- What orchestrator ties them together? (thin, handles side effects)
- What boundaries exist? (DB, filesystem, HTTP — mock here)

Present the decomposition to the user:
"I'll create 3 functions:
 - `extract_symbols(ast) → Vec<Symbol>` (pure, unit testable)
 - `write_symbols(symbols, graph)` (side-effect, integration testable)
 - `process_file(path, graph)` (orchestrator, calls both)
Does this decomposition make sense?"

### Step 4: Write tests FIRST — present for approval
Write test cases for each pure function. Show them to the user:
"Here are the test cases for `extract_symbols`:
 - given a Rust file with 2 functions → returns 2 symbols with correct names
 - given an empty file → returns empty vec
 - given a file with nested functions → returns only top-level
 - given a file with impl block → returns methods with parent_id set

Do these cover the right behavior? Anything to add or change?"

Wait for user approval before implementing.

### Step 5: Implement to pass tests
Write implementation. Run tests. Show results.

### Step 6: Verify
Run full test suite. Check no regressions.
Log: `log_event("tests_presented", { count: N, approved: true })`
Log: `log_event("tests_passed", { count: N, failures: 0 })`
```

---

## Impact on pattern knowledge (idea 17)

Testability is itself a **pattern option** that should be documented:

### Pattern option: Code structure for testability

| Option | Description | When to use | Tradeoff |
|--------|-------------|-------------|----------|
| **Monolith** | Single function does everything | Quick prototype, throwaway code | Fast to write, impossible to test in isolation |
| **Decomposed pure + orchestrator** | Pure functions for logic, thin orchestrator for side effects | Production code, anything that needs testing | Slightly more files/functions, but each is independently testable |
| **Hexagonal / ports-adapters** | Core logic has zero external dependencies, adapters at boundary | Libraries, long-lived systems | Most flexible, most setup |

The AI should default to "decomposed pure + orchestrator" unless the user explicitly chooses otherwise (e.g., for a quick experiment).

---

## Impact on events and metrics

| Event | When | Data |
|-------|------|------|
| `tests_presented` | AI shows tests to user before implementation | test_count, function_names, approved (bool) |
| `tests_passed` | After implementation, tests run | test_count, pass_count, fail_count |
| `decomposition_proposed` | AI proposes function decomposition | function_count, pure_count, side_effect_count |

**New metric:**

| Metric | Formula |
|--------|---------|
| Test-first rate | `tests_presented` events where `approved=true` before corresponding `tests_passed` / total tasks |
| Decomposition quality | Average testability score of new functions |

---

## Open questions

| # | Question |
|---|----------|
| 1 | Should the AI refuse to implement without test approval, or just strongly nudge? (Consistent with D3: soft gates, nudge don't block) |
| 2 | How does the user approve tests in auto-mode? Auto-mode skips deliberation — but TDD requires deliberation. Tension. |
| 3 | Should testability score be computed during indexing (stored on nodes) or on-demand (MCP query)? |
| 4 | For the sensei codebase specifically: which task workers need decomposition? Should we create issues for each? |
