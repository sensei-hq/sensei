# 験 · Pipeline · Testability & TDD

**Owner files:**
- Function-shape analysis: `crates/senseid/src/patterns/shape.rs`
- TDD guardrails (workflow enforcement): `crates/senseid/src/workflow/tdd_gate.rs`
- Test-approval surface: MCP `propose_tests` / `approve_tests`

**Companion design doc:** [`docs/archive/ideas/18-testability-tdd.md`](../../archive/ideas/18-testability-tdd.md).

## Purpose

AI-generated code tends toward monolithic functions — faster to
write one giant function than to decompose. That's cheap in the
first turn, expensive in every turn after (hard to test, hard to
change, hard to compose). Testability catches this early with
two mechanisms:

1. **Function shape analysis** — every function has a shape:
   `params → purpose → returns → uses[]`. Sensei detects when
   the shape crosses a complexity threshold and proposes
   decomposition BEFORE the code is written.
2. **TDD gate** — the assistant proposes tests FIRST; the user
   approves; then the assistant writes code to pass those
   tests. Without the human approval step, TDD degenerates into
   "AI writes tests it knows will pass."

Kanji is 験 — *test / verify*.

## Data invariants

### Function-shape record

- `inference.function_shapes` — one row per detected function:
  - `id`, `node_id`, `params_count`, `has_side_effects` bool,
    `pure` bool, `uses_count`, `complexity_estimate`,
    `testable_verdict` enum `unit | integration | boundary | untestable`,
    `detected_at`.
- The verdict is derived:
  - `unit` — pure, few params, no side effects → simple
    assertions test it.
  - `integration` — orchestrator: calls pure functions +
    handles side effects → integration tests.
  - `boundary` — adapter to external system (DB / filesystem /
    HTTP) → mock at the boundary.
  - `untestable` — too many params, hidden side effects,
    unclear purpose → decompose before writing tests.

### TDD gate — the workflow contract

Enforced through the workflow phases (see
[[pipeline/analyzer]] enrichment step 6):

    Step 1: assistant proposes tests (no implementation)
    Step 2: assistant surfaces tests via `propose_tests(session, tests)`
    Step 3: user reviews on [[screen/observatory-insights]] or
            inline in their assistant → `approve_tests`
    Step 4: assistant implements to make tests pass
    Step 5: assistant runs tests, reports results via
            `record_outcome`

`sensei.tdd_proposals` — persists step 1–2 outputs so the user
sees what was proposed before implementation.

### Decompose-before-code

When function-shape analysis detects an about-to-be-written
function with `testable_verdict = untestable`, the pipeline
raises a warning to the assistant via
[[pipeline/patterns]] anti-pattern surfacing:

- Anti-pattern id: `anti.god_function`.
- Message: "This function has 8 params, writes to 3 sources,
  and calls 15 helpers. Consider decomposing before writing
  tests."
- Suggestion: extract N pure helpers + 1 orchestrator.

## Signals produced

| Signal | Consumer |
|---|---|
| Function shape record | [[pipeline/patterns]] anti-pattern detection |
| TDD proposal | Insights + Playground |
| Approved tests → implemented code coverage delta | Impact (verdict when TDD gates fire) |
| Untestable-shape warnings | Insights Now column violation card |

## Done gate

- Every new function detected by the analyzer has a
  `function_shapes` row within the incremental window.
- TDD proposals persist and are approvable through the standard
  Apply-family verb.
- Approved tests can be executed and reported through
  `record_outcome`.
- Function shape verdicts match a manual reading on the
  documented examples in the archive.
- Assistant sessions that follow the TDD flow record a distinct
  outcome tag; effectiveness measured over 30d.

## Wrong gate

- **A test-approval step is skipped** and the assistant writes
  tests inline with the implementation. Gate not enforced.
- **Function-shape verdict wrong** — a clearly pure function
  labelled `boundary`.
- **`propose_tests` accepts empty test lists.** Should reject
  and prompt.
- **Approved test suite executes but no coverage delta
  measured.** Impact link broken.
- **Untestable-shape warnings never surface** despite obviously
  monolithic functions being added.

## Related

- [[pipeline/patterns]] — anti-pattern surface for untestable
  shapes
- [[pipeline/analyzer]] — enrichment scheduling
- [[pipeline/impact]] — TDD effectiveness verdict
- [[pipeline/governance]] — TDD as a governance rule (P1
  recommended, promotable to P0)
- [[screen/observatory-insights]] — where TDD proposals surface
- (archive: ideas/18-testability-tdd.md) — source design
