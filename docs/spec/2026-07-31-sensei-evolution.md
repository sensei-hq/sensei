# Spec — Sensei evolution: constitution injection, surface simplification, library agents, review rigor, maintenance automation

> Grounded in a comparison against `~/Work/example-corp/agent-context` (a leaner
> "layered constitution for coding agents" system) plus a full inventory of
> sensei's own command / skill / MCP / library / review-agent surface. Analysis
> first (Part 0), then workstream specs A–F, then a prioritized backlog. G (the
> dōjō coverage CI failure) is already fixed — commit `fb73b41f`.

**Status:** spec / not started (except G = done).
**Sources:** three grounding passes on 2026-07-31 — agent-context trio map, sensei
surface inventory, dōjō coverage repro. Key existing spec this extends:
[[spec/pipeline/library-intelligence]].

---

## Part 0 — Findings (the analysis)

### 0.1 agent-context vs sensei — sensei is closer than it looks

agent-context (`agentcontext-cli` + `agentcontext-mcp` + `agentcontext-handler`, all
TS, Postgres-backed) is a 3-service layered-constitution system. Its MCP tools map
**almost 1:1 onto tools sensei already has**:

| agent-context tool | sensei equivalent | Gap |
|---|---|---|
| `get_layered_context` (constitution+org+client+project+repo+learnings, one call) | `get_layered_context` (blended memory) + `get_rules` (governance) | sensei splits memory/rules across two tools; no single "conflicts" array |
| `get_constitution` | `get_rules` (mandatory tier) | — |
| `save_learning` (scope split, evidence-gated, secret-block/PII-flag) | `save_memory` / `propose_memory` | **no evidence gate, no secret/PII guard, no local-JSONL scope** |
| `propose_promotion` → admin approve → materialize as rule | `propose_memory` → `accept_proposal` → `promote_memory` | sensei has the verbs; **no evidence requirement, no cache-invalidation contract, weaker "materialize as first-class rule" story** |
| `resolve_risk_class` (path/tag → auto\|review\|approve) | — | **missing — no risk gate that decides review depth** |
| `record_outcome` | `record_outcome` / `report_run_outcome` | — |
| audit_log (tamper-evident, PII-redacted, append-only) | `log_event` | **no canonical-JSON hash, no PII redaction, no UPDATE/DELETE block** |

**Where agent-context is cleaner:**
- **One load call, behavior in injected memory.** The whole governance payload arrives
  via a single `get_layered_context()` at session start; standing behavior (reuse-first,
  idempotency, no-secrets, capture-learnings) lives as always-on prose in
  `CLAUDE.md`/`AGENTS.md`/Cursor/Copilot, not as commands. Only 5 thin auto-invoked
  skills exist (`save`/`validate`/`recall`/`promote`/`promotions`).
- **Idempotent managed-file injection.** A `<!-- agentcontext:managed -->` marker makes
  the CLAUDE.md block re-write-safe; the injected directive is *"BEFORE any other
  action, call `get_layered_context()`, treat all layers as authoritative, list any
  `conflicts` and ask which rule wins, halt if the server is unreachable."*
- **Evidence-gated capture with a security guard.** Non-local `save_learning` *requires*
  evidence; a secret scan hard-blocks the write, PII is flagged. `local` scope writes to
  `~/.agentcontext-mem/<user>/learnings.jsonl` (private, never federated).
- **Promotion materializes.** `propose_promotion` opens a review record; admin approval
  in the console inserts a first-class `instructions` rule at the target layer and fires a
  cache-invalidation webhook so it's live immediately.
- **Tamper-evident audit** on *every* tool call (SHA-256 of canonical params+result, PII
  redaction, DB trigger blocks UPDATE/DELETE).

**Where sensei is already ahead:**
- **Compaction re-injection.** agent-context has **no** PreCompact/SessionStart hook — its
  only re-load path is a manual `/recall`. Sensei already re-pushes rules at SessionStart
  + PreCompact (P3/D-INJECT via `marketplace/plugins/sensei/hooks/session-start`).
- **Code-graph tools.** `search`/`get_callers`/`get_callees`/`get_duplicates`/
  `get_communities`/`get_patterns` have no agent-context analogue.
- **Far richer runtime** (relay, planner, checkers, analyzer, libraries).

**Net:** don't "port their commands." Port the five sharp mechanisms sensei lacks —
(1) the managed first-message directive, (2) evidence+secret/PII gate on save,
(3) the promote→materialize contract, (4) the tamper-evident audit wrapper,
(5) `resolve_risk_class`.

### 0.2 Sensei surface — the simplification facts

- **21 commands** (`marketplace/plugins/sensei/commands/`): 5 thin-wrapper
  (`checkpoint`, `docs`, `mockup`, `patterns`, `spec`), 8 orchestrator, 8 utility.
  `idea`/`analyze`/`blueprint`/`experiment`/`plan`(human-mode) are **five near-identical
  "author a frontmatter-templated doc + `update_phase` + `log_event`" flows** differing
  only by target folder/template.
- **23 skills** with real clusters: verification (`verify-outcome` + `data-reality-check`
  + `verify-deploy`), grounding (`ground-before-scope` + `recall-canon` +
  `data-reality-check`), UI-build (`ui-state-pattern` ⊂ `tauri-screen-dev` ⊂
  `building-app-mockups`), styles (`semantic-styles` vs `semantic-styles-rokkit`), memory
  (`knowledge-capture` vs `recall-canon`). `plan-depth-review` skill **duplicates** the
  `sensei-plan-depth-reviewer` agent. `help.md`'s skill table is **stale** (lists a
  nonexistent `analyze` skill; omits 9 real ones).
- **51 MCP tools.** `get_layered_context` returns **memory, not rules** — `get_rules`
  already owns rules. So the proposed `get_layered_rules` rename is the wrong direction.
- **Libraries are pull-only.** The session-start hook injects governance rules but for
  libraries only prints a static tool-name reference — it never injects `library_pages`.
  Docs are reached solely via explicit `get_lib_docs`/`search_lib_docs`. **No
  library→skill/agent/tool association exists** — though the schema *reserves*
  `libraries.props.skill_path` and [[spec/pipeline/library-intelligence]] already
  designs `sensei.library_skills` + `library_versions` + `get_library_skill` /
  `list_library_skills`.
- **9 review agents, only 1 runs tests.** All 9 grant the sensei MCP tools, but only
  `sensei-acceptance-tester` is instructed to execute the test suite. The 4 no-Bash agents
  (`analyst`, `persona-reviewer`, `ux-designer`, `plan-depth-reviewer`) structurally
  cannot verify against live state; the 4 other Bash agents use Bash for static scans, not
  test runs. **None** query `psql`, curl `:7744`, or drive the app/Playwright. This is the
  mechanical root of "reviews gloss over."

---

## Part A — Constitution injection at first message + compaction

**Problem.** Sensei *pushes* rules via the session-start hook (Claude only, event-driven),
but plants no *pull-first* directive telling the agent to fetch governance before acting —
so non-Claude ACPs (Cursor/Codex/Copilot) and any session where the hook doesn't fire get
nothing. agent-context's managed CLAUDE.md directive is the missing belt-and-suspenders.

**Proposal.**
1. `sensei init` writes an **idempotent managed block** (marker-delimited, e.g.
   `<!-- sensei:managed -->…<!-- /sensei:managed -->`) into `CLAUDE.md`, `AGENTS.md`,
   `.cursor/rules/sensei.mdc`, `.github/copilot-instructions.md`. Content: *"On session
   start AND after any context compaction, call `get_rules()` + `get_layered_context()`
   first; treat mandatory rules as non-negotiable; if rules conflict, list them and ask
   which wins; then proceed."*
2. Re-run safe via the marker (skip if present, replace block on `--force`). Mirror
   agent-context's `writeWithStrategy` marker check.
3. Reconcile with the existing hook: hook = push for Claude; directive = pull fallback for
   all ACPs + the explicit compaction re-fetch instruction (sensei already re-pushes at
   PreCompact for Claude; the directive generalizes it).
4. Add a `conflicts[]` array to `get_rules` output (mandatory rule vs a more-specific
   override that illegally tries to weaken it) so the directive's "halt on conflict" has
   real data — mirrors agent-context's `non_overridable`-misuse detection.

**Owner files:** `crates/cli` (init/scaffold), `marketplace/plugins/sensei/hooks/`,
`crates/mcp` (`get_rules` conflicts), `crates/senseid` `/api/knowledge/rules`.
**Effort:** M. **Risk:** low (additive, idempotent).

---

## Part B — MCP tool naming

**Finding.** `get_layered_rules` (the requested name) is the wrong direction:
`get_layered_context` returns **memory**, and `get_rules` already owns **rules** — so
`get_layered_rules` would collide with `get_rules` and mislabel the memory plane. The real
issue is that "context" is vague.

**Recommendation.** **Do not** rename to `get_layered_rules`. Options, in order:
- (Preferred, cheap) Keep the name; sharpen its catalog description to "blended *memory*
  (decisions/conventions/learnings), not governance rules — see `get_rules`."
- (If a rename is still wanted) `get_layered_memory` is the accurate counterpart to
  `get_rules`. Blast radius is **contained**: 1 dispatch site (`crates/mcp/src/main.rs:402`;
  the daemon endpoint `/api/knowledge/context` is decoupled), catalog + ~15 test refs, and
  ~13 doc refs (commands/skills/agents). No `app/`/`dojo/` refs.

**Effort:** S (docs-only) or M (rename). **Risk:** low. **Recommendation:** docs-only now;
defer the rename unless the memory/rules split is being reworked anyway.

---

## Part C — Command/skill simplification + port promote/save

**C1 — Collapse the 5 doc-authoring commands.** `idea`/`analyze`(author-mode)/`blueprint`/
`experiment`/`plan`(human-mode) → one parametrized `/sensei:doc <phase>` flow (or an
auto-invoked `author-phase-doc` skill) that takes `{phase, folder, template}`. Keep
`analyze`/`plan` as thin dispatchers only for their *other* (skill-delegating) modes.

**C2 — Demote thin wrappers to auto-invoked skills.** `checkpoint`, `docs`, `patterns`,
`spec`, `mockup` are ~10-line forwarders. Where Claude Code auto-invokes skills reliably,
drop the command and rely on the skill (agent-context's model). Keep a command only where
an explicit typed invocation is the UX (e.g. `commit`, `session`).

**C3 — Merge/relate skill clusters** (don't hard-merge distinct procedures; add an
umbrella + cross-links):
- Verification family: keep `verify-outcome` (read real output), `data-reality-check`
  (query live rows/artifact), `verify-deploy` (rebuild+smoke) as distinct *procedures* but
  under one "verify" index skill that routes to the right one. They are genuinely different
  checks; the overlap is the shared premise, not the procedure.
- `plan-depth-review` **skill** → delete; keep the `sensei-plan-depth-reviewer` **agent**
  (dispatch form) and point the skill's triggers at it.
- Fix `help.md`'s stale skill table (doc-drift; regenerate from the skills dir).

**C4 — Port promote/save semantics** (sensei has the verbs; add the rigor):
- **Evidence gate:** `save_memory`/`propose_memory` for non-local scope require an
  `evidence[]` (file:line, test name, run id). Reject without it.
- **Security guard:** a secret scan **hard-blocks** the write; PII is flagged (reuse
  `crates/senseid/src/resolution.rs` fail-closed idioms). Never persist a secret into
  memory.
- **Local scope:** a `local` memory scope that writes to a per-user file (not the shared
  DB), never federated — the private-scratchpad tier agent-context has.
- **Materialize contract:** document + test that `promote_memory`/`accept_proposal`
  inserts a first-class rule row AND invalidates the rules cache so it's live immediately
  (agent-context's approve→materialize→invalidate).
- **`promote` UX:** a guided `/sensei:promote` (or auto-skill) that walks scope selection +
  evidence, backed by the existing verbs.

**Owner files:** `marketplace/plugins/sensei/{commands,skills}`, `crates/mcp` (save/promote
tools), `crates/senseid` memory handlers + DDL (`memories` evidence column, local scope).
**Effort:** M–L. **Risk:** medium (touches governance write path — must not weaken it).

---

## Part D — Library ↔ skill/agent association (extends library-intelligence.md)

**Problem.** Libraries are indexed llms.txt that agents never automatically reach, and a
library cannot advertise the skills/agents/tools it provides. The user's model: **sensei
owns generalized skills/agents; each library repo owns its specialized skills/agents/tools**,
and sensei *surfaces* them when a project uses that library.

**Most of the ingestion/skill-generation is already specced** in
[[spec/pipeline/library-intelligence]] (`sensei.library_skills`, `library_versions`,
`get_library_skill`, `list_library_skills`, version pinning, drift). This workstream **adds
the association + provisioning layer** on top:

**D1 — Library manifest (provided capabilities).** Extend the library model so a library
declares, via a manifest in its own repo (`sensei.library.json` or a section in its
`llms.txt`), the **skills, agents, and MCP tools it ships** + the version they apply to.
Sensei ingests this into `sensei.library_skills` (+ a new `library_agents`) instead of only
*generating* skills. This makes rokkit/gateway/dbd/kavach the owners of their own
specialized knowledge.

**D2 — Recommender hook.** Detection already populates `referenced_libraries` /
`project_libraries` from manifests (`crates/senseid/src/adapters/manifest.rs`). Wire
`recommend_playbook` / `/sensei:intake` / `get_intake_guide` to offer a library's
skill+agent when the project depends on it ("this repo uses rokkit → load rokkit's
component-review agent"). This is the designed-but-unbuilt `enable_skill` insight in
[[spec/pipeline/analyzer]].

**D3 — Per-library review agents.** A library ships a review agent (rokkit → "config +
palette + component-pattern reviewer"; gateway/dbd/kavach → their equivalents) that reviews
*before* coding (is the config right? are the patterns idiomatic?) and *verifies after*.
These live in the **library's** repo, register with sensei via D1's manifest, and are
dispatched by D2's recommender. Sensei provides the framework; the library provides the
content.

**D4 — Relocate misplaced skills.** `semantic-styles-rokkit` (entirely rokkit-internal:
`rokkit.config.js` palettes/skins/`-z` tokens) → **move to the rokkit repo** and consume it
via D1. Raise a tracking issue on rokkit to adopt it. (`tauri-*` skills are framework-general
— lower priority; keep for now.)

**D5 — Auto-inject a library's skill at session start** when the project uses it heavily
(usage threshold from library-intelligence.md) — closes the "pull-only" gap so library
knowledge reaches the agent without an explicit call.

**Owner files:** `crates/senseid/src/libraries/` (new `skills.rs`, `manifest`),
`database/ddl/table/sensei/{library_skills,library_versions,library_agents}.ddl`,
`crates/mcp` (`get_library_skill`/`list_library_skills`/`list_library_agents`),
`marketplace` (move rokkit skill out). **Effort:** L. **Risk:** medium.

---

## Part E — Review diligence (PRIORITY)

**Problem (mechanical root).** Reviews gloss over because the review agents don't *do* the
verification: 1/9 runs tests, 0/9 query live DB / curl the daemon / drive Playwright, and
none audit whether the *tests themselves* assert the functional requirement (vs a mock or
fallback). TDD holes (asserting dummy fallback data, `assert(true)`-shaped checks) pass
review because nothing checks the assertions against intent.

**Proposals.**

**E1 — Make review agents verify, not just read.** Amend the Bash-capable review agents
(`developer`, `acceptance-tester`, `security-reviewer`, `performance-engineer`,
`devops-sre`) to REQUIRE, per their domain, concrete verification evidence in their output:
the actual test-run tail, a `psql` count, a `curl :7744` status, a Playwright run for UI
diffs — never "looks correct." An agent that cannot produce the evidence must say so, not
pass. Encode this as a shared "Verification evidence (required)" block, mirroring the
`verify-outcome`/`data-reality-check`/`verify-deploy` skills' bar.

**E2 — Test-intent audit (new dimension/agent).** Add a `sensei-test-reviewer` (or a
`/sensei:review` dimension) that reviews **the tests against the functional need**:
- Does each test assert the *requirement*, not a fallback/dummy/mock value? (Flag
  assertions that match a `unwrap_or_default`/fixture path.)
- Are assertions meaningful (not vacuous, not asserting the mock they set up)?
- Is every acceptance criterion covered by a test that would *fail* if the feature
  regressed? (adversarial: "what input makes this test pass while the feature is broken?")
This is the `pr-test-analyzer` pattern, wired to sensei's tools.

**E3 — Tool-backed duplicate/existence check (the user's example).** A `dry-check` skill
that **mandates** calling `search` / `get_duplicates` / `get_callers` *before* writing a new
function/type — and the review verifies it happened. It's tool-supported, so make it a hard
step, not a guideline. Ties the existing DRY rule in CLAUDE.md to the actual index tool.

**E4 — Risk-class gate (port `resolve_risk_class`).** A `resolve_risk_class(paths, task)` →
`auto | review | approve` decision (path patterns: `auth/`/`payments/`/identity/DDL →
approve; `src/`/`lib/` → review; docs/tests → auto). This decides **how hard to review**:
`approve` triggers the full multi-agent adversarial pass + human sign-off; `auto` skips the
heavy pass. Directly fixes "reviews gloss over" by escalating the changes that matter and not
wasting rigor on the trivial. Feeds `/sensei:review` and the autonomous run's gate.

**E5 — Playwright not optional for UI diffs.** `/sensei:validate` and the acceptance-tester
must run the Tauri/Playwright suite (`tauri-playwright-testing` skill) whenever the diff
touches UI, and assert it ran (evidence). No "verified by reading the component."

**E6 — Adversarial framing.** Review agent prompts shift from "confirm it works" to "find
the input that breaks it / the assertion that's vacuous / the criterion with no test." Add a
verify-then-refute step.

**Owner files:** `marketplace/plugins/sensei/agents/*` (evidence blocks, new test-reviewer),
`marketplace/plugins/sensei/skills/` (`dry-check`, risk-class), `crates/mcp` +
`crates/senseid` (`resolve_risk_class` tool + handler), `commands/{review,validate}.md`.
**Effort:** M (agents/skills) + M (risk-class tool). **Risk:** low-medium.

---

## Part F — Scheduled maintenance automation

**Problem.** Registered libraries drift (new versions, new/updated docs, new skills/agents,
security patches) with no automated upkeep. The user wants a recurring job that models
versions and applies updates by severity.

**Most of the version model is specced** in [[spec/pipeline/library-intelligence]]
(`library_versions`, lockfile pinning, re-ingest on bump, drift). This workstream **adds the
scheduler + apply policy**:

**F1 — Library-update scheduler.** A long-lived tick (mirror
`crates/senseid/src/tasks/reconcile_scheduler.rs` / the analyzer scheduler) that, per
registered library, checks the source (llms.txt ETag, registry version, git tag) for a newer
version and records `current` / `lowest-used` / `available` per library.

**F2 — Severity-based apply policy.**
- **Patch** → auto-re-ingest docs + regenerate skills; no gate.
- **Minor** → compatibility check (drift detection from library-intelligence.md) →
  apply if clean, else surface a review item.
- **Major** → never auto; generate a migration skill (diff the two doc snapshots) + a
  review item.
- **Security patch** → scan a vuln source (e.g. advisory DB / `cargo audit` / `osv`),
  surface high-severity with an expedited apply recommendation. (Do NOT auto-apply code
  changes; auto-apply docs/skills refresh + flag.)

**F3 — Provisioned-capability refresh.** When a library updates, re-pull its D1 manifest so
new/changed skills+agents flow in, and mark superseded ones.

**Owner files:** `crates/senseid/src/tasks/` (new `library_update_scheduler.rs`),
`crates/senseid/src/libraries/version.rs`, a vuln-source adapter.
**Effort:** L (phased: v0 poll+notify → v1 auto-patch → v2 security scan). **Risk:** medium
(auto-apply must be conservative; never auto-change code).

---

## G — Dōjō coverage CI (DONE)

Fixed in `fb73b41f`. Root cause: `src/lib/paraglide/` (generated by
`paraglideVitePlugin`, gitignored) is absent on a fresh CI checkout, and
`dojo/vitest.config.ts` doesn't load that plugin, so 3 relay suites couldn't resolve
`$lib/paraglide/messages`. Fix: `dojo/package.json` `test`/`test:watch` now run
`paraglide-js compile --project ./project.inlang --outdir ./src/lib/paraglide` before
vitest. Verified from a clean state (dir removed): `bun run test` and the CI's
`bun run test --coverage` both pass 1175/1175, exit 0.

---

## Prioritized backlog (recommended sequencing)

The user selected **all** of E, D, A+B+C, F as priorities. Recommended order — highest
leverage / lowest risk first, front-loading the review-rigor concern:

| # | Item | Part | Effort | Risk | Why here |
|---|---|---|---|---|---|
| 1 | ✅ Risk-class gate (`resolve_risk_class`) | E4 | M | L | **DONE `0ec738d2`** — pure classifier + endpoint + MCP tool + `/sensei:review` Step 0; installed + smoked live |
| 2 | ✅ Review agents verify + test-intent audit + dry-check | E1b/E2/E3 | M | L | **DONE `5c58b899`** — 5 agents evidence-mandatory; `sensei-test-reviewer` (mutation spot-check) wired into review Check 4; `dry-check` skill wired into build Step 3 + review Check 2 |
| 3 | ✅ Playwright-mandatory + adversarial framing | E5/E6 | S | L | **DONE `933bd15d`** — `/sensei:validate` hard-requires the Playwright/component suite on a UI diff; adversarial refutation in review Step 0 approve-depth + agent evidence blocks |
| 4 | ✅ Managed first-message + compaction directive | A | M | L | **DONE `933bd15d`** — `sensei init` writes the idempotent "load governance first" block into CLAUDE.md/AGENTS.md (managed.rs, 6 tests). Remaining sub-item: `get_rules` `conflicts[]` array |
| 5 | Command collapse + skill cluster cleanup + help fix | C1/C2/C3 | M | L | **C3 DONE `3d7adc69`**. C1/C2 **grounded-out**: the commands aren't redundant wrappers (patterns/docs/spec are substantive; the 5 doc commands are distinct phase flows; `plan-depth-review` is planner-wired, not a dup) — collapsing destroys value. Real "fewer commands" = a deliberate workflow-UX redesign, not a dedup. Not doing it. |
| 6 | Evidence + secret/PII gate on save/promote | C4 | M | M | **secret guard DONE `ccb7d356`** + **evidence DONE `90935401`** (reused `memory_evidence`, session-nullable; save-time source note). Remaining (deliberate, not quick): a hard evidence *gate* (needs the internal-caller carve-out) + PII flag (no consumer yet). **C is substantively complete.** |
| 7 | Library manifest + recommender + relocate rokkit skill | D1/D2/D4 | L | M | **D1+D2 DONE `eeb7b312`** (design-workflow blueprint, skeptic-verified): `sensei.library.json` manifest, `library_skills`/`library_agents` tables, `index_library` ingestion (LocalDir v1), `list/get_library_skill(s)`+`list_library_agents` MCP tools, recommender `suggested_skills/agents` (fail-closed). **D4 pending user** (outward): file the rokkit issue (below) + delete sensei's stale `semantic-styles-rokkit` fork once rokkit ships its manifest. |
| 8 | Per-library review agents + session-start inject | D3/D5 | L | M | **D3 ingestion side DONE** (`library_agents` + `list_library_agents` receive a library's declared agent). Remaining: rokkit authors `rokkit-styles-reviewer` (via the issue); **D5 auto-inject-on-heavy-use deferred** (recommender surfaces at intake now). |

### D4 — rokkit issue (FILED: jerrythomas/rokkit#142)

Remaining D4: once rokkit acts on #142 (ships its `sensei.library.json` + agent), sensei
ingests rokkit's skill/agent → **then** delete sensei's stale `semantic-styles-rokkit` fork
(`marketplace/plugins/sensei/skills/semantic-styles-rokkit/`) + `make marketplace-push`.

**Title:** Own `semantic-styles-rokkit` as the sole source of truth + ship a config/pattern review agent + declare `sensei.library.json`

**Body:** sensei is retiring its stale in-marketplace fork of `semantic-styles-rokkit` (teaches the old `-z`-only vocabulary); rokkit already ships the canonical newer copy at `packages/cli/skills/semantic-styles-rokkit/SKILL.md`. Going forward rokkit is the single owner and sensei *ingests* rokkit's skills/agents. Asks: (1) keep the skill canonical in rokkit; (2) add a per-library review agent `packages/cli/agents/rokkit-styles-reviewer.md` (sensei agent format: reviews a consuming app's `rokkit.config.js`/token usage before coding, verifies with `bun run build` + Playwright snapshots after) + a `rokkit agents add` command mirroring `rokkit skills add`; (3) commit a root `sensei.library.json`: `{"library":"rokkit","version":">=1.3","skills":[{"name":"semantic-styles-rokkit","focus":"styling","path":"packages/cli/skills/semantic-styles-rokkit/SKILL.md"}],"agents":[{"name":"rokkit-styles-reviewer","focus":"styling-review","path":"packages/cli/agents/rokkit-styles-reviewer.md"}]}` so sensei's D1 ingestion associates them to any rokkit-using project and auto-recommends them (D2). Non-goal: no change to the skill's teaching content.
| 9 | Library-update scheduler + apply policy | F1/F2/F3 | L | M | Automation; depends on version model |
| 10 | MCP naming (docs-only; rename deferred) | B | S | L | Low value; do the doc clarity, hold the rename |

---

## Part E follow-up — review findings (approve-class adversarial pass)

Whole-repo adversarial review of the E workstream at `5c58b899`, run via the risk-class gate.
Verified live: `make test-fast` green (165+10 Rust bootstrap · 1279 app unit); MCP 51+7 green;
full senseid suite green with `--test-threads=1` (1817 passed). Findings and the plan to close
them:

| # | Finding | Severity | Plan |
|---|---|---|---|
| F1 | **Risk-class gate under-escalates identity files** — `dojo/client.rs` (device tokens), `gateway_config_loader.rs` (Bearer/api_key), `jwt.rs`/`tokens.rs`/`saml.rs`/`mfa.rs`/`signing.rs`, `relay/auth.rs` all classify `review` (the `/auth/` needle only matches a directory, so `auth.rs` slips). Latent: `is_test_or_doc` matches any segment ending `test/` → `latest/`/`contest/` dirs classify `auto`. | HIGH | Add high-signal needles to `crates/senseid/src/review.rs:55-85` (`auth.rs`, `jwt`, `saml`, `mfa`, `session_store`, `device_token`, `signing`) + tighten the `test/` substring to exact segments. Add regression cases. |
| F2 | **Full suite red under parallel DB tests** — 4 senseid tests fail on the shared `sensei_test` DB when run in parallel (all pass alone; 1817 green serial). `test-fast` doesn't cover them, so `make test` can stay red silently. | MED | Serialize the DB-backed tests (marker or `--test-threads=1` profile) so `make test` is reliable. |
| F3 | **Wrong-target / vacuous tests** — `languages/common.rs:207-211` zero assertions; `tests/e2e_index.rs:15-66` never touches the real indexer, lines 88-121 compute-but-never-assert signals. | MED | Rewrite against the real indexer API or delete; add real assertions on computed signals. |
| F4 | **Dojo fixture remnants on identity-adjacent screens** — `org/[slug]/+page.ts` (needsYou from kit fixtures), `you/[section]/+page.ts:5` (stance/ladder/rulePacks fixtures), `org/[slug]/[section]/+page.ts:143` (`candidateDetailFor` fixture). | MED | Replace with daemon reads (honest-empty) per the no-fixtures rule. |
| F5 | **Marketplace/doc drift** — `help.md` stale (lists nonexistent `analyze`, omits ~9 skills); catalog `pre/post-tool` hook paths resolve to nothing (catalog.json:457-476); sensei-mcp base-path wrong at :160; 4 near-duplicate skill clusters + 5 near-identical doc-authoring commands. | LOW | Fold into C1/C3 backlog rows (surface simplification). |
| F6 | `transcript/mod.rs:163` `unwrap_or(true)` masks a DB error as "has events" (self-correcting). | LOW | Prefer propagating the error; note as accepted if kept. |

**Acceptance criteria for closing:** `review.rs` classifies every credential/identity/session file
`approve` (unit test enumerates the list above); `make test` green in default parallel mode; the
two vacuous/wrong-target tests gone or asserting real behavior; no kit-fixture imports on
identity-adjacent dojo screens; catalog/help in sync.

---

## Open decisions (need a call before building the affected item)

1. **Command removal vs. keep-as-alias.** For C2, do we hard-remove demoted commands, or
   keep them as thin aliases for muscle-memory? (Recommend: keep as one-line aliases that
   just note "auto-invoked now" for a release, then remove.)
2. **Where library agents live + how they register.** D1/D3: a `sensei.library.json`
   manifest in each library repo vs. a central registry. (Recommend: manifest in the
   library repo — keeps ownership with the library, matches "repos own specialized".)
3. **Security-patch autonomy.** F2: how far does auto-apply go for security patches — docs/
   skills refresh + flag only (recommended), or also open a PR bumping the dependency?
4. **Local memory scope storage.** C4: per-user file location + whether it ever syncs.
5. **`get_layered_context` rename.** B: docs-only (recommended) or commit to
   `get_layered_memory` now.

## Related
- [[spec/pipeline/library-intelligence]] — ingestion/skill-gen/version model (D/F build on it)
- [[spec/pipeline/analyzer]] — the `enable_skill` insight (D2 hook)
- `marketplace/plugins/sensei/{commands,skills,agents}` — the surface being simplified
- Governance rules P2/P5 + the 6 FTR skills — the review-rigor culture E enforces
