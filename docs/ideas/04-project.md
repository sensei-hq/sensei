# Project

A deep view into a single project. Understand your code, its patterns, and how AI assistants work with it. The project page is where you go to explore structure, enforce conventions, track documentation health, and customize how Sensei works in your codebase.

---

## Project dashboard

The project dashboard is the per-project entry point. It surfaces everything Sensei knows about one codebase through a set of tabs:

| Tab | What it shows |
|-----|---------------|
| **Overview** | Project identity, first-try-resolution trend, session count, repo list, recommendations, pattern summary |
| **Code Graph** | Interactive visualization of symbols, calls, dependencies, and structural hotspots |
| **Patterns** | Detected and imported patterns with enforcement status and conformance tracking |
| **Libraries** | Third-party and internal libraries with docs, usage patterns, and API references |
| **Sessions** | Session history with outcomes, corrections, and replay |
| **Traceability** | Doc-to-code links, drift detection, and coverage matrix |
| **Extensions** | Skills, agents, personas, inference settings, and benchmarking |

The overview tab gives you a single-screen health check: what is working, what is not, and what Sensei suggests next. Recommendations include supporting evidence (for example, "3 sessions corrected this week") and a projected impact on first-try resolution.

---

## Code intelligence

Sensei indexes your codebase into a symbol graph that goes beyond keyword search. The graph captures functions, types, modules, call relationships, dependencies, and structural patterns.

### Lens modes

The graph visualization offers three lens modes:

- **Dependency** -- force-directed layout of call and import relationships. Nodes are sized by fan-in plus fan-out.
- **Matrix** -- heatmap showing which modules call which. Dense clusters stand out immediately.
- **Clusters** -- community-detection bubbles grouping tightly coupled code. Useful for spotting module boundaries and misplaced files.

### Overlays

Each lens mode supports five overlays that color-code nodes:

| Overlay | What it reveals |
|---------|-----------------|
| **Complexity** | Cyclomatic complexity per function. Red nodes need decomposition. |
| **Coverage** | Which functions are well-tested and which have no tests at all. |
| **Frequency** | How often a symbol appears in recent sessions. Hot paths stand out. |
| **Age** | Time since last change. Stale code drifts from documentation. |
| **Ownership** | Who last touched each file. Useful for knowing who to ask. |

### Interactive navigation

Click any node to see its details: file path, fan-in and fan-out counts, rework frequency, and whether it has been flagged as a hotspot. From a selected node you can explore callers, callees, and the full call chain. You can also launch a refactor prompt directly from a selected node.

---

## Patterns

Sensei detects patterns from your code and supplements them with industry catalogs so the AI builds on existing foundations instead of reinventing structure.

### Lifecycle

Patterns follow a four-stage lifecycle:

```
Detect  -->  Surface  -->  Enforce  -->  Grow
```

1. **Detect** -- the indexer identifies recurring structures: adapters, factories, workers, observers, builders, and project-specific conventions. Detection starts with naming heuristics and advances through structural analysis to custom rule matching.
2. **Surface** -- detected patterns are exposed to the AI through tools and to you through the pattern catalog. Each pattern includes its instances, invariants, and a reference implementation.
3. **Enforce** -- during coding, the AI checks for applicable patterns before writing new code. During review, it catches violations and duplication. Enforcement is configurable: advisory (warn) or strict (block).
4. **Grow** -- patterns evolve from corrections. When you correct the AI ("you should have used the adapter pattern"), the correction feeds back into the pattern store. Repeated violations trigger suggestions to add guardrails.

### Pattern catalog

The catalog shows all detected patterns with instance counts, coverage, and conformance status. You can browse by category (structural, creational, behavioral, project-specific), drill into any pattern to see its interface and all instances, and promote a suggested pattern to an enforced rule.

Sensei also imports patterns from industry sources and library conventions. When both a codebase pattern and an industry pattern apply, the AI presents both with tradeoffs and lets you choose.

### Enforcement during build and review

During a build, the AI checks for applicable patterns before writing code. If it finds a match, it presents the pattern and follows the established structure. If the match is uncertain, it asks you before proceeding.

During review, the AI checks modified files against known patterns, flags violations, and detects newly introduced duplication. A conformance report shows which patterns were followed and which were missed.

### First-encounter presentation

When the AI encounters an architectural decision for the first time in a project -- for example, how to handle string localization or data loading -- it presents the available options with tradeoffs instead of picking silently. Once you choose, the decision is recorded and followed automatically in subsequent sessions.

---

## Search and context delivery

Sensei replaces keyword-only search with a hybrid approach that combines three signals in a single query.

### Search modes

| Mode | What it does |
|------|-------------|
| **Full-text** | Stemmed, ranked search with phrase proximity. Finds "refresh token rotation" even if words are spread across a file. |
| **Semantic** | Embedding-based similarity. Understands natural language queries like "where do we handle auth token refresh?" without requiring exact keywords. |
| **Structural** | Graph-aware queries. "What calls this function" and "what module owns this symbol" are answered from the graph, not by reading files. |

A keyword fallback layer ensures Sensei never returns fewer results than a raw text search. Each result carries a confidence signal so the AI knows whether to trust the answer or keep looking.

### Resolution levels

Context is served at the depth the task requires:

| Level | Content | When used |
|-------|---------|-----------|
| **L0** | Signature only -- function name, parameters, return type | Orientation. The AI needs to know a function exists. |
| **L1** | IO pattern -- inputs, outputs, and contract | The AI needs to understand what a function does, not how. |
| **L2** | Logic flow -- summarized implementation | The AI needs to understand the approach before modifying. |
| **L3** | Full source | The AI needs to edit or deeply reason about the code. |

### Token budget

Every context delivery is token-budgeted. Sensei walks the call graph outward from the task, prioritizing the most relevant paths, and stops when the budget is reached. It tracks what the AI has already seen in the current session to avoid resending the same context.

### Task-relevant selection

Context is ranked by relevance to the current task, not just proximity in the file tree. Recent diffs, traceability links, and semantic similarity all contribute to the ranking.

---

## Libraries

Sensei tracks third-party and internal libraries so the AI uses current documentation instead of hallucinating outdated APIs.

### What Sensei knows about a library

For each library, Sensei shows:

- **Top symbols and usage counts** -- which parts of the library your project actually uses.
- **Call sites** -- file, line, and snippet for every place the library is referenced.
- **Attached rules** -- any project-specific conventions for using the library.
- **Documentation** -- fetched and indexed docs, API references, and usage patterns.

### Library intelligence pipeline

Sensei detects dependencies from your project manifests, checks whether documentation has been indexed, and fetches it if not. When the library ships its own docs in a machine-readable format, Sensei uses them directly. Otherwise it fetches from a community registry of library doc URLs.

For internal shared libraries, Sensei indexes them from source alongside external dependencies.

### Usage patterns

Beyond raw API docs, Sensei extracts usage patterns -- the conventions specific to a library. For example, a UI component library might expect a consistent props interface, specific internal wrappers, and a registration step. The AI learns these conventions and follows them when creating new code that uses the library.

---

## Traceability

Sensei links design documents to code symbols and tracks whether those links are still valid.

### How it works

During indexing, Sensei builds a bidirectional map between documentation files and the code symbols they describe. This map is maintained automatically as both code and docs change.

### Drift detection

When code changes break a documented contract -- a renamed function, a changed signature, a removed module -- Sensei flags the drift. You configure the response: warn (surface in the dashboard) or block (prevent the AI from using stale docs as context).

### Coverage matrix

The traceability view shows a coverage matrix: which docs are tracked, how many are current, how many have drifted, and how many are broken. You can filter by status and drill into any document to see its individual symbol references and their health.

### Doc tools

Sensei includes a doc doctor that lints documentation for completeness and consistency. When code exists with no design doc at all, Sensei can generate a starting-point document from the code for you to review and refine.

---

## Testability

Sensei scores your code for testability and guides the AI toward writing code that is easy to test.

### Testability scoring

Each function gets a testability score based on its shape: parameter count, side effects, cyclomatic complexity, and dependency count. Functions with high scores are pure, focused, and independently testable. Functions with low scores are flagged with specific recommendations: "split into pure logic plus orchestrator," "reduce parameters," "isolate side effects."

### Decomposition guidance

Before writing implementation code, the AI proposes a decomposition: which functions will be pure (data transformations, no side effects), which will be orchestrators (thin wrappers that handle side effects), and which will be adapters (boundary code for external systems). You review and approve the decomposition before coding begins.

### TDD enforcement

Sensei enforces a test-driven workflow. The AI writes test cases first and presents them to you for review before writing any implementation. You can adjust, add, or remove test cases. Only after approval does the AI implement the code to make the tests pass. This ensures tests validate what should be built, not what was built.

### Test-first presentation

The workflow is explicit:

1. AI writes test cases (no implementation).
2. AI presents tests: "Do these cover the right behavior?"
3. You review and approve.
4. AI implements to make tests pass.
5. AI runs the full suite and shows results.

Without step 2 and 3, TDD degenerates into test-after with extra steps. Sensei makes the human review step non-optional.

---

## Working with assistants

Sensei integrates with your AI assistant (Claude Code, Cursor, or others) invisibly. There is no separate Sensei interface during a coding session -- everything happens inside the assistant you are already using.

### How a session works

When you open your AI assistant, Sensei hooks fire automatically. A small context payload (around 300 tokens) is loaded into the session with project identity, current workflow phase, active patterns, recent decisions, project rules, and the active persona. This context steers the assistant without you needing to repeat project conventions.

### Workflow phases

Sessions move through phases, each with a corresponding command:

```
brainstorm --> analyze --> blueprint --> plan --> build --> validate
```

Each phase instructs the assistant on protocol, required tool calls, and expected outputs. During brainstorm, the focus is on exploring the problem space. During build, pattern checks, decomposition, and TDD are enforced. During validate, the assistant verifies the work against acceptance criteria.

### Context injection

On session start, `get_session_context()` delivers approximately 300 tokens covering:

- Project and repo identity
- Current workflow phase and active task
- Patterns to follow
- Recent decisions from prior sessions
- Project rules and the active persona

This is enough for the assistant to work with full project awareness from the first turn.

### Correction detection

When you redirect the assistant ("no, account for clock skew"), Sensei records the correction: which turn it happened, what was corrected, and how it was resolved. Corrections feed into analytics and pattern growth -- repeated corrections on the same topic trigger a recommendation to create a rule or persona.

### Post-session analytics

When a session ends, Sensei records the outcome (first-try resolution, corrected, or abandoned), token counts, tool usage, and phase transitions. This data flows into the observatory where you can track trends over time.

---

## Extensions

The extensions tab is where you customize how Sensei works for your project.

### Skills browser

Browse all installed skills, commands, agents, and hooks. Filter by kind and scope (global or per-project). Toggle extensions on or off. Create new skills by writing markdown with frontmatter, or import from the marketplace.

### Agent editor

Define autonomous agents that can run multi-step verification and analysis tasks. Each agent has a tool access checklist, an autonomy level (fully autonomous, checkpoint-based, or manual), and a procedure. You can start from templates for common tasks (code review, test generation, doc update) and test agents against historical session replays before enabling them.

### Persona editor

Create project-specific personas that activate based on working directory patterns. Each persona carries trigger conditions, rules for the assistant to follow, files to always include, and patterns to enforce. A preview shows exactly what the assistant will receive when the persona is active. Personas are grounded in real session failures -- you can link evidence sessions that inspired the persona.

### Inference settings

Control which models power Sensei's reasoning. Manage local models (pull, delete, update), configure external providers with API keys, and set routing preferences (auto, always local, always external). The MOE reasoning panel lets you assign roles (proposer, challenger, synthesizer) to different models.

### Benchmark runner

Measure whether a skill, rule, or persona actually improves outcomes. Define a task corpus, run it with and without the change, and compare first-try resolution, corrections, token usage, and duration. The runner produces a plain-language summary of whether the change is effective, with actions to promote, iterate, or discard.

---

## Reference

- [Daemon architecture](../design/02-daemon.md)
- [MCP protocol and tools](../design/04-mcp.md)
- [Marketplace and extensions](../design/06-marketplace.md)
