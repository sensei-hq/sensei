---
name: Skill & Command Mapping
description: Map every existing marketplace skill, command, hook, and plugin against the 6 core concepts and 16 ideas — identify disposition and required actions
date: 2026-04-17
status: analysis-complete
origin: docs/ideas/01-workflow-system.md
---

# Skill & Command Mapping

## Purpose

Map the existing sensei marketplace inventory (19 skills, 13 commands, 3 hooks, 3 plugins) against the workflow system ideas. For each component: what concept does it serve, what idea does it align with, and what's the disposition (keep, revise, absorb, retire).

---

## Mapping: Skills → Core Concepts

### Concept 1: Workflow

Skills that enforce development process discipline.

| Skill | Idea | Disposition | Action |
|-------|------|-------------|--------|
| `working-smarter` | 01 Workflow | **Absorb** into `/sensei:build` | Extract commit-first and zero-errors behaviors into the build command. Framework-native mockup rules move to `/sensei:mockup`. |
| `decomposing-broad-tasks` | 01 Workflow | **Absorb** into `/sensei:plan` | Task decomposition is what the plan phase does. The 5-file threshold and parallel dispatch logic become part of plan command behavior. |
| `zero-errors-policy` | 01 Workflow | **Absorb** into `/sensei:build` and `/sensei:commit` | Zero-errors checkpoints are embedded in the build and commit commands. Not a standalone skill. |
| `design` | 01 Workflow | **Absorb** into `/sensei:blueprint` | Design-first approach with pattern lookup and decision recording becomes the blueprint command. |
| `guiding-doc-creation` | 01 Workflow | **Revise** | The doc naming/numbering convention is useful but the feature/design split needs updating to match the new phase folder structure. Keep as a reference during blueprint, not a standalone skill. |

### Concept 2: Logging & Qualitative Analysis

Skills that support session tracking, recovery, and qualitative feedback.

| Skill | Idea | Disposition | Action |
|-------|------|-------------|--------|
| `session-management` | 11 Session Continuity | **Absorb** into `/sensei:session` + phase commands | Session protocol (get_session_context → recommend_next → snapshot → checkpoint) becomes embedded behavior. Every phase command calls the session MCP on entry. |
| `context-efficiency` | 14 Context Delivery | **Absorb** into phase commands | `recommend_next()` and `context_pack()` are called automatically by phase commands, not as a separate skill the AI must remember. |

### Concept 3: Metrics & Quantitative Analysis

Skills that measure and visualize quality.

| Skill | Idea | Disposition | Action |
|-------|------|-------------|--------|
| `auditing-skill-descriptions` | 07 Metrics | **Retire** | This was a meta-skill for auditing skill trigger quality. With the new command-based system, skill descriptions matter less. Trigger accuracy is replaced by explicit commands. |
| `analyze` | 07 Metrics + 08 Codebase | **Absorb** into `/sensei:analyze` | The codebase health check (complexity, hotspots, interrupted sessions) becomes one mode of the analyze command. |

### Concept 4: Assistive Tooling

Skills that provide precision assistance — indexing, patterns, context, libraries.

| Skill | Idea | Disposition | Action |
|-------|------|-------------|--------|
| `codebase-indexing` | 08 Codebase Intelligence | **Keep** (revise) | Still needed for initial repo setup. Revise to work with the Rust daemon indexer instead of the old JS pipeline. Update to use `senseid` commands. |
| `pattern-based-development` | 15 Pattern Store | **Absorb** into `/sensei:patterns` | Pattern lookup before coding becomes the refocus command's job. |
| `identifying-patterns` | 15 Pattern Store | **Absorb** into `/sensei:pattern-extract` | Pattern discovery (find 2+ implementations → extract recipe) stays as the extract command. |
| `identify-unknown-libs` | 09 Library Intelligence | **Keep** (revise) | Still needed — detects missing library docs and prompts for source. Revise to work with Rust daemon's `add_library` endpoint. |
| `refactor` | 08 Codebase Intelligence | **Keep** (revise) | Refactoring workflow is orthogonal to phases. Keep as utility. Update MCP tool calls to match current daemon API. |
| `test-gen` | 08 Codebase Intelligence | **Keep** (revise) | Test generation is orthogonal to phases. Integrates with `/sensei:build` (TDD cycle). Update MCP calls. |
| `extract-docs` | 13 Doc Traceability | **Keep** (revise) | Doc extraction from code is still useful. Update to use current MCP tools. |

### Concept 5: Knowledge Integrity

Skills that ensure documentation accuracy and freshness.

| Skill | Idea | Disposition | Action |
|-------|------|-------------|--------|
| `detecting-doc-drift` | 13 Doc Traceability | **Absorb** into `/sensei:validate` | Drift detection becomes part of the validate phase. Also triggered by `/sensei:review`. |
| `reformatting-docs` | 13 Doc Traceability | **Retire** | Doc reformatting to canonical templates is a one-time cleanup, not an ongoing skill. Handle during the docs disposition work (idea 06). |

### Concept 6: Platform & Adoption

Skills for reverse-engineering and multi-coordinator support.

| Skill | Idea | Disposition | Action |
|-------|------|-------------|--------|
| `reverse-engineering` | 12 Multi-Coordinator | **Absorb** into `/sensei:analyze` | The three modes (product, feature, audit) become modes of analyze. Product and feature analysis feed into blueprint. Audit feeds into review. |
| `building-app-mockups` | N/A (UI tooling) | **Absorb** into `/sensei:mockup` | Mockup workflow (framework-native, two alternatives, promote winner) moves into the mockup utility command. |

---

## Mapping: Commands → New Commands

| Current command | New command | Notes |
|----------------|-------------|-------|
| `/sensei:session` | `/sensei:session` | **Keep** — entry point for sessions |
| `/sensei:checkpoint` | `/sensei:checkpoint` | **Keep** — mid-session snapshots |
| `/sensei:commit` | `/sensei:commit` | **Keep** — zero-errors commit gate |
| `/sensei:help` | `/sensei:help` | **Keep** — update to show new command surface |
| `/sensei:enable` | `/sensei:enable` | **Keep** — may evolve into config model |
| `/sensei:disable` | `/sensei:disable` | **Keep** — may evolve into config model |
| `/sensei:get-api-docs` | `/sensei:get-api-docs` | **Keep** — library doc fetching |
| `/sensei:mockup` | `/sensei:mockup` | **Keep** — absorbs `building-app-mockups` and `working-smarter` mockup rules |
| `/sensei:pattern-extract` | `/sensei:pattern-extract` | **Keep** — absorbs `identifying-patterns` |
| `/sensei:pattern-use` | `/sensei:patterns` | **Rename** — becomes refocus command |
| `/sensei:product` | `/sensei:analyze` | **Absorb** — product mode of analyze |
| `/sensei:feature` | `/sensei:analyze` | **Absorb** — feature mode of analyze |
| `/sensei:audit` | `/sensei:review` | **Absorb** — audit mode of review |
| `/sensei:backlog` | `/sensei:refocus` | **Absorb** — part of re-anchoring |

---

## Mapping: Hooks → New Hooks

| Current hook | Disposition | Action |
|--------------|-------------|--------|
| `session-start` | **Revise** | Update to inject new command awareness, phase context, and tool preference hierarchy. Add lightweight guardrails loading. |
| `pre-tool` | **Keep** | Tool usage analytics. Wire into hooks.json (currently defined but not registered). |
| `post-tool` | **Keep** | Tool result analytics. Wire into hooks.json (currently defined but not registered). |
| (new) `PreCompact` | **Create** | Auto-fire lightweight refocus before context compaction. Save active phase, current task, and guardrails summary. |

---

## Mapping: Plugins → New Plugins

| Current plugin | Disposition | Action |
|----------------|-------------|--------|
| `sensei-mcp` | **Revise** | Config points to `senseid --mcp` but current MCP is the Rust binary `sensei-mcp`. Verify all tool contracts match what skills/commands expect. |
| `playwright-mcp` | **Keep** | Used for E2E testing in `/sensei:validate`. |
| `firebase-mcp` | **Keep** | External service integration, orthogonal to workflow. |

---

## Gap Analysis

### Missing components (need to be created)

| Component | Type | Concept | Idea | Priority |
|-----------|------|---------|------|----------|
| `/sensei:idea` | Command | 1 Workflow | 01 | High — first phase command |
| `/sensei:analyze` | Command | 1 Workflow | 01 | High — replaces product/feature/audit + analyze skill |
| `/sensei:blueprint` | Command | 1 Workflow | 01 | High — replaces design skill |
| `/sensei:experiment` | Command | 1 Workflow | 01 | Medium — new capability |
| `/sensei:plan` | Command | 1 Workflow | 01 | High — replaces decomposing-broad-tasks |
| `/sensei:build` | Command | 1 Workflow | 01 | High — absorbs working-smarter + zero-errors |
| `/sensei:validate` | Command | 1 Workflow | 01 | Medium — absorbs drift detection |
| `/sensei:brainstorm` | Command | 1 Workflow | 01 | Medium — new capability |
| `/sensei:review` | Command | 1 Workflow | 01 | High — absorbs audit |
| `/sensei:guardrails` | Command | 1 Workflow | 01 | High — context decay mitigation |
| `/sensei:patterns` | Command | 1 Workflow | 01 | Medium — replaces pattern-use |
| `/sensei:refocus` | Command | 1 Workflow | 01 | High — context decay mitigation |
| `/sensei:tools` | Command | 1 Workflow | 01 | Medium — tool amnesia mitigation |
| PreCompact hook | Hook | 1 Workflow | 01 | High — auto-refocus on compaction |
| Guardrails file template | Template | 1 Workflow | 01 | High — living document for project rules |
| Phase doc templates | Templates | 1 Workflow | 01 | High — idea, analysis, blueprint, experiment, plan |

### Broken or stale components (need fixing)

| Component | Issue | Action |
|-----------|-------|--------|
| `pre-tool` hook | Defined but not wired in hooks.json | Wire into hooks.json |
| `post-tool` hook | Defined but not wired in hooks.json | Wire into hooks.json |
| `sensei-mcp` plugin config | Points to `senseid --mcp`, actual binary is `sensei-mcp` | Update plugin config |
| Multiple skills reference old MCP tools | Tool names/signatures may not match Rust daemon API | Audit all MCP tool calls against current `sensei-mcp` tool contracts |
| `session-start` hook | Injects old skill list and context format | Update to reflect new command surface |

### Skills with stale MCP tool references

These skills call MCP tools that may have changed in the Rust rewrite. Each needs verification against the current `sensei-mcp` tool list.

| Skill | MCP tools referenced | Status |
|-------|---------------------|--------|
| `session-management` | `get_session_context`, `recommend_next`, `take_snapshot`, `checkpoint`, `record_memory`, `close_memory` | Verify |
| `context-efficiency` | `get_session_context`, `recommend_next`, `context_pack`, `search`, `checkpoint` | Verify |
| `design` | `context_pack`, `search`, `get_bearings`, `get_symbol`, `record_memory`, `take_snapshot` | Verify |
| `analyze` | `get_session_context`, `get_complexity`, `get_bearings`, `record_memory` | Verify |
| `codebase-indexing` | `get_llmspec` | Verify |
| `test-gen` | `search`, `get_symbol` | Verify |
| `refactor` | `get_complexity`, `get_symbol`, `context_pack`, `record_memory`, `take_snapshot` | Verify |
| `extract-docs` | `get_bearings`, `get_symbol`, `record_memory` | Verify |
| `detecting-doc-drift` | `check_drift` | Verify |
| `reverse-engineering` | Various — complex multi-tool workflow | Verify |

---

## Disposition Summary

| Disposition | Count | Components |
|-------------|-------|------------|
| **Keep** (revise) | 7 skills, 8 commands, 2 hooks, 3 plugins | Core utilities that are orthogonal to phases |
| **Absorb** into new commands | 11 skills, 5 commands | Behaviors embedded into phase/cross-cutting/refocus commands |
| **Retire** | 2 skills | `auditing-skill-descriptions`, `reformatting-docs` — no longer needed |
| **Create** | 13 commands, 1 hook, ~7 templates | New workflow system components |

---

## Recommended Actions Before Blueprint

### Priority 1: Foundations (do first)

1. **Verify MCP tool contracts** — Audit the Rust `sensei-mcp` binary's actual tool list against what skills reference. Create a tool contract document mapping old JS tool names to current Rust tool names. This unblocks all skill revisions.

2. **Wire pre-tool and post-tool hooks** — They exist but aren't in hooks.json. Wiring them enables interaction tracking (idea 07) without new code.

3. **Fix sensei-mcp plugin config** — Update to point to the correct binary path. Already fixed in Claude settings but marketplace plugin config needs updating too.

4. **Create phase doc templates** — Templates for idea, analysis, blueprint, experiment, plan. These define the contract for what each phase produces. Needed before any command can be implemented.

5. **Create guardrails file template** — Define the structure of `.sensei/guardrails.md`. This is the living document that grows from feedback.

### Priority 2: Reorganize (do during blueprint)

6. **Archive superpowers/** — Move `docs/superpowers/plans/` and `docs/superpowers/specs/` to `docs/_archive/superpowers/`.

7. **Update session-start hook** — Revise to inject new command surface, phase awareness, and tool preference hierarchy.

8. **Consolidate skill behaviors into command specs** — For each new command, write a spec that explicitly lists which skill behaviors it absorbs and how.

### Priority 3: Build incrementally (do during build phase)

9. **Implement commands one at a time** — Start with `/sensei:refocus` and `/sensei:guardrails` (highest immediate value — solve context decay). Then `/sensei:build` (absorbs most skills). Then remaining phase commands.

10. **Retire absorbed skills** — Only after their replacement commands are tested and working.

---

## Next phase

This analysis feeds into `/sensei:blueprint` — the architecture of how commands, phases, templates, guardrails, and the MCP server interact as a system.
