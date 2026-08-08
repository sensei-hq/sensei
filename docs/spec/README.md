# Sensei — LLM-executable spec

**Purpose.** This directory translates the mockups + journey maps into
per-screen and per-pipeline specs an LLM (or a new engineer) can use to
build to a defined "done." Every spec answers five questions:

1. **Purpose.** What should the user feel here, in one paragraph.
2. **Data invariants.** What must be true in the DB before this screen
   can look right. Names the tables, the joins, the "sensei project has
   ≥ 1 enriched session" style preconditions.
3. **Signals shown.** What each visible number / card / chip means, with
   worked examples. If a number is meaningless without context, we say
   what context makes it meaningful.
4. **Done gate.** The English claim we make when the screen is finished.
   Optionally a curl-shaped check for the non-obvious ones.
5. **Wrong gate.** What would make this screen embarrassing to demo.
   The list of failure modes we specifically watch for.

Screens without all five answers are stubs. Fill them before coding.

---

## The vision — one paragraph

Sensei is a **helpful, observer and mentor of how a developer works with AI
coding assistants. mentoring surfaces as insights land**. It captures the sessions (hook events, tool calls, prompts, outcomes), builds a code + activity graph over the developer's
real repos, and turns the resulting signal into things a human can act
on: memories that stick, patterns worth reusing, guards worth adopting,
and a clear picture of when the pairing worked and when it didn't. It is
not a productivity dashboard — it is a **retrospective loop for a pair
(you + your assistant) that otherwise never gets one**. The Observatory
shows *today's one thing*; the project window shows *what this project
learned*; the Dōjō (SaaS) extends the same loop across a team without
leaking client work.

**North-star metric:** FTR — first-turn resolution — the fraction of
sessions where the assistant's first attempt landed without a
correction. Every screen is judged by whether it makes FTR go up or
exposes the reason it went down.

**The pair goes both ways.** Sensei is not "watching the assistant
make mistakes" — it's watching a *pair* (human + assistant) and
noticing patterns from both sides. Sometimes the correction was the
assistant's fault; sometimes it was the human giving underspecified
instructions, incomplete context, or wrong assumptions. Both are
learning signal. Deferred but not forgotten: user-facing learnings
("you tend to give sparse instructions when asking for schema
changes — the assistant will do better if you include X") and an
assistant behaviour where the LLM proactively asks for clarification
with a reason ("I need to know the migration policy before I can
answer confidently; without it, half my choices are guesses"). The
ultimate vision is human and LLM working in sync — mutual
improvement, not one-sided teaching.

---

## The journey — the four segments we build against

Source of truth: [`docs/mockups/Sensei/Sensei Journey Map.html`](../mockups/Sensei/Sensei%20Journey%20Map.html).

| # | Segment | Kanji | Purpose | Screens |
|---|---|---|---|---|
| 01 | Bootstrap | 支 | Reach a working, trustworthy environment without thinking about toolchains. Homebrew · Postgres · Ollama · daemon come up green. | probing, green |
| 02 | First run & Preferences | 名 | Point Sensei at real folders, watch projects appear, then tune what defaults got wrong. **The wizard has moved into Preferences — it's not a first-run gauntlet.** | scan, first projects, preferences |
| 03 | Observatory — daily use | 家 | Walk in, learn the one thing that needs me today, act on it, stay in control of what leaves my machine. | today, projects, project, sessions, insights, memory, libraries, instruments, upgrades, impact, collective |
| 04 | The project window | 雲 | Work inside one project end-to-end and trust what Sensei learned here before any of it travels. | overview, sessions, memories, traceability, libraries, instruments, patterns, impact, about |

Dōjō (SaaS) is a companion track — [`Sensei Dōjō Journey Map.html`](../mockups/Sensei/Sensei%20D%C5%8Djo%20Journey%20Map.html):

| Role | Surface | Stages |
|---|---|---|
| Developer | in-app | discover, authenticate, bind project→org, a finding forms, share upstream, watch it travel, receive downstream |
| Maintainer | console | queue, evaluate, decide, set distribution, publish & measure |
| Org admin | console | stand up, connect identity, provision members, scopes & policies, monitor |
| Client / engagement lead | console | define engagement, anonymize always, no per-item review, audit trail, incident handling |

---

## The themes — non-negotiable design principles

Every spec must honour these. If a spec pushes against one, call it out.

1. **Value before setup.** The first thing the user does is see their
   own projects. Not a wizard.
2. **One decision, one default.** Across insights, traceability,
   libraries, upgrades — the same verb set: **Apply · Review · Dismiss.**
   The recommended one is highlighted; the others are one keystroke
   away.
3. **Discoverability of depth.** Nothing important is hidden behind
   one-liner descriptions. Preferences is searchable. The sidebar
   clusters with a Focus mode. Nav entries name the safety screens.
4. **Trust through proof.** No claim without a receipt. Confidence
   scores, regression notes, before/after FTR — the user verifies,
   they don't take our word for it.
5. **Org boundary is Dōjō.** Anything that should stay inside a company
   or a client engagement goes through the Dōjō lane, not the global
   Collective. Personal Sensei can operate perfectly without a Dōjō —
   but when one exists, the boundary is exact.
6. **Insight copy comes from the model, not the template.** Every
   human-readable string on an insight card, koan, adopted-blurb,
   drift note, or FTR sentence goes through
   [[pipeline/insight-copy]] — embedded gemma4 first, static template
   as fallback. Templated copy hits its ceiling immediately (the same
   "N tools dormant" reads as noise the second time). The mentor
   voice needs a mentor writer. Actions and route labels stay
   deterministic; the code owns those.

---

## The doc template — every spec looks like this

    # {kanji} · {name}

    **Segment:** 03 · Observatory | Route: `/…` | Source mockup: `lib/….jsx`

    ## Purpose

    One paragraph. What the user feels. Not what the screen contains.

    ## Data invariants

    - `activity.sessions` has at least one row for {project} in the last 14d
    - `analyzer.session_metrics` join returns non-null ftr for at least 50%
    - No orphaned tool_calls (client_session_id must resolve to a session)

    ## Signals shown

    Prefer the 4-column form when the screen has ≥ 4 rows and
    signal density matters. For thin surfaces (one card, one
    stat strip), a 2-column `Element | Value` table is
    acceptable — but every ambiguous value must still carry a
    worked example somewhere in the doc.

    | Element | Value shape | Meaning | Example |
    |---|---|---|---|
    | FTR chip | 0.00–1.00 pct | 14d rolling first-turn resolution | 0.63 → "63%, up 8 pts this window" |
    | Dormant summary | integer | Tools with no calls in 14d, aggregated | "40 tools dormant" — not 40 cards |

    ## Done gate

    - When Jerry loads this screen on the sensei project he sees an FTR
      chip with a real number, at least one non-noise signal, and can
      click through to at least one memory that was promoted this week.
    - Zero repeated cards ("no calls in N days" appears at most once).

    Optional check:
    ```
    curl -s http://localhost:7744/api/projects/sensei/ftr | jq '.ftr14d'
    # expected: > 0.4
    ```

    ## Wrong gate

    - Numbers that don't correlate (e.g. dormant count = 0 while list has 40)
    - Silent empty state where data exists but wasn't fetched
    - Names shown as UUIDs (name-or-UUID resolution missing)
    - Dark-mode text on light-tint backgrounds

    ## Related

    - [[pipeline/analyzer]] · [[pipeline/ftr]] · [[screen/instruments-health]]

---

## The index — every doc, current status

Status legend: **draft** = written and reviewed, **stub** = frontmatter
only, **todo** = not started, **roadmap** = describes an intended system
not yet built (verify against the code before relying on it — see
[`analysis/2026-08-05-indexer-capability-coverage.md`](../analysis/2026-08-05-indexer-capability-coverage.md)).

### Screen specs — personal Sensei

| Segment | Doc | Status | Notes |
|---|---|---|---|
| 01 Bootstrap | [screen/bootstrap-probing.md](screen/bootstrap-probing.md) | **draft** | 6 gates, why-and-if-missing per row |
| 01 Bootstrap | [screen/bootstrap-green.md](screen/bootstrap-green.md) | **draft** | Calm all-green, auto-transition |
| 02 First-run | [screen/first-run-scan.md](screen/first-run-scan.md) | **draft** | The only first-time gate: pick roots |
| 02 First-run | [screen/first-entry-projects.md](screen/first-entry-projects.md) | **draft** | Value-before-setup landing with banner |
| 02 Preferences | [screen/preferences.md](screen/preferences.md) | **draft** | 8 panes, searchable, review cues |
| 03 Observatory | [screen/observatory-today.md](screen/observatory-today.md) | **draft** | Today — the koan surface |
| 03 Observatory | [screen/observatory-projects.md](screen/observatory-projects.md) | **draft** | The projects index rebuilt today |
| 03 Observatory | [screen/observatory-sessions.md](screen/observatory-sessions.md) | **draft** | SessionsDigestZen — trend/stream/etc. variants |
| 03 Observatory | [screen/observatory-insights.md](screen/observatory-insights.md) | **draft** | Learnings Triage — Now/Soon/Settled |
| 03 Observatory | [screen/observatory-memories.md](screen/observatory-memories.md) | **draft** | Learnings Anatomy v2 — LLM-primary + human promotion ladder |
| 03 Observatory | [screen/observatory-libraries.md](screen/observatory-libraries.md) | **draft** | LibrariesVariantA — ecosystem/tier/wrap-me filters |
| 03 Observatory | [screen/observatory-instruments-playground.md](screen/observatory-instruments-playground.md) | **draft** | MCP tree + tool detail + execute round-trip |
| 03 Observatory | [screen/observatory-instruments-replay.md](screen/observatory-instruments-replay.md) | **draft** | Session picker + tool-call timeline + verdict chips |
| 03 Observatory | [screen/observatory-instruments-health.md](screen/observatory-instruments-health.md) | **draft** | Signal derivation shipped today |
| 03 Observatory | [screen/observatory-upgrades.md](screen/observatory-upgrades.md) | **draft** | Downstream lane (Apply/Mute/Pin) |
| 03 Observatory | [screen/observatory-impact.md](screen/observatory-impact.md) | **draft** | Verdicts + Regressions nav entry always visible |
| 03 Observatory | [screen/observatory-collective.md](screen/observatory-collective.md) | **draft** | Global↔Dōjō toggle with independent controls |
| 03 Observatory | [screen/observatory-traceability.md](screen/observatory-traceability.md) | **draft** | Doc-drift list + Expected-vs-Actual diff |
| 03 Observatory | [screen/observatory-consolidation.md](screen/observatory-consolidation.md) | **draft** | Merge similar memories/patterns/rules |
| 03 Observatory | [screen/observatory-logs.md](screen/observatory-logs.md) | **draft** | Operator surface + scheduled-task strip |
| 03 Observatory | [screen/observatory-share-review.md](screen/observatory-share-review.md) | **draft** | Batch review upstream — client credit locked to anonymous |
| 03 Observatory | [screen/observatory-dojo-connections.md](screen/observatory-dojo-connections.md) | **draft** | Memberships + SSO/OAuth/device-code |
| 03 Observatory | [screen/observatory-dojo-sharing.md](screen/observatory-dojo-sharing.md) | **draft** | Per-membership sharing overrides |
| 04 Project window | [screen/project-overview.md](screen/project-overview.md) | **draft** | |
| 04 Project window | [screen/project-sessions.md](screen/project-sessions.md) | **draft** | Zen digest with multi-repo folder-role chip |
| 04 Project window | [screen/project-memories.md](screen/project-memories.md) | **draft** | Ready-to-share lane + generalise action |
| 04 Project window | [screen/project-traceability.md](screen/project-traceability.md) | **draft** | Doc-coverage summary + Expected/Actual drawer |
| 04 Project window | [screen/project-libraries.md](screen/project-libraries.md) | **draft** | One-click wrap + version-conflict warnings |
| 04 Project window | [screen/project-instruments.md](screen/project-instruments.md) | **draft** | 3-tab shell scoped to project |
| 04 Project window | [screen/project-patterns.md](screen/project-patterns.md) | **draft** | 5-source patterns; promotion to rule via ladder |
| 04 Project window | [screen/project-impact.md](screen/project-impact.md) | **draft** | Project FTR trend with apply-event annotations |
| 04 Project window | [screen/project-about.md](screen/project-about.md) | **draft** | Vision + multi-repo membership + Dōjō binding |

### Screen specs — Solution scope

| Doc | Status | Notes |
|---|---|---|
| [screen/solution-dashboard.md](screen/solution-dashboard.md) | **draft** | Cross-repo aggregate FTR + members + connections |
| [screen/solution-sessions.md](screen/solution-sessions.md) | **draft** | Sessions across member projects with role facet |
| [screen/solution-architecture.md](screen/solution-architecture.md) | **draft** | Merged cross-repo graph + focus mode + doc-link edges |

### Additional Observatory / Preferences

| Doc | Status | Notes |
|---|---|---|
| [screen/settings-inference.md](screen/settings-inference.md) | **draft** | Inference pane — chains + models + budget + circuit state |
| [screen/insights-reasoning.md](screen/insights-reasoning.md) | **draft** | MOE consensus panel — propose → challenge → synthesize |

### Screen specs — Dōjō (SaaS)

| Role | Doc | Status |
|---|---|---|
| Developer | [screen/dojo-developer-flow.md](screen/dojo-developer-flow.md) | **draft** | Journey narrative wiring across in-app screens |
| Maintainer | [screen/dojo-maintainer-console.md](screen/dojo-maintainer-console.md) | **draft** | Queue / evaluate / decide / distribute / measure |
| Org admin | [screen/dojo-admin-console.md](screen/dojo-admin-console.md) | **draft** | Stand up / identity / members / policies / monitor |
| Client / engagement lead | [screen/dojo-lead-console.md](screen/dojo-lead-console.md) | **draft** | Engagement + universal strip + audit trail |

### Pipeline specs — the data behind the screens

| Doc | Status | Notes |
|---|---|---|
| [pipeline/capture.md](pipeline/capture.md) | **draft** | Hook events + scanner + multi-repo project detection |
| [pipeline/analyzer.md](pipeline/analyzer.md) | **draft** | Scheduler + L0 enrichment + L1 signal derivation |
| [pipeline/code-graph.md](pipeline/code-graph.md) | **draft** | Idempotent node/edge/community indexing + retrieval contract; fixes the Atlas regression (dedup, incremental re-runs, restored kinds) |
| [pipeline/ftr.md](pipeline/ftr.md) | **draft** | Correction-signal detection, 14d roll-up, north-star metric |
| [pipeline/metrics.md](pipeline/metrics.md) | **draft** | Registry-driven metric computation + project/session/date value store + roll-up views + derived health score |
| [pipeline/memory.md](pipeline/memory.md) | **draft** | LLM-primary consumer + human promotion ladder |
| [pipeline/traceability.md](pipeline/traceability.md) | **roadmap** | Doc-drift scanner, confidence scoring, auto-fix policy — **only deletion-of-symbol drift is built; `traces_to`=0, drift is file-level, no signature diff** |
| [pipeline/impact.md](pipeline/impact.md) | **draft** | MeasureVerdicts + regression alerts |
| [pipeline/libraries.md](pipeline/libraries.md) | **draft** | Detect → wrap → query → watch + lib-docs ingestion |
| [pipeline/insights.md](pipeline/insights.md) | **draft** | Generator + apply/review/dismiss + follow-up to impact |
| [pipeline/signals.md](pipeline/signals.md) | **draft** | Health-tab signal derivation (rewritten today) |
| [pipeline/insight-copy.md](pipeline/insight-copy.md) | **draft** | LLM-generated human-readable insight text — gemma4 primary, static fallback. **All insight copy reaches for this.** |
| [pipeline/project-icon.md](pipeline/project-icon.md) | **draft** | README/logo/favicon → project icon inference chain with kanji + letter fallback. |
| [pipeline/memory.md](pipeline/memory.md) | **draft** | Memory pipeline — LLM-primary consumer via MCP `get_memories`; formation, scope, promotion ladder, feedback. |
| [pipeline/mcp-surface.md](pipeline/mcp-surface.md) | **draft** | Tool declarations + defaults contract + third-party discovery |
| [pipeline/governance.md](pipeline/governance.md) | **draft** | Priority ladders + scope precedence + rule promotion path |
| [pipeline/dojo-lifecycle.md](pipeline/dojo-lifecycle.md) | **draft** | Memberships + routing + attribution + upstream/downstream loop; SaaS+self-hosted modes |
| [pipeline/patterns.md](pipeline/patterns.md) | **roadmap** | 5 sources + anti-patterns + optimization — **not built: `crates/senseid/src/patterns/` absent; `detected_patterns` is behavioral churn, `family` NULL** |
| [pipeline/inferencing.md](pipeline/inferencing.md) | **draft** | Gateway + adapters + MOE consensus + fallback / budget / circuit |
| [pipeline/context-delivery.md](pipeline/context-delivery.md) | **draft** | Resolution levels L0-L3 + token budget + BFS ranking + session dedup |
| [pipeline/semantic-search.md](pipeline/semantic-search.md) | **roadmap** | Hybrid FTS+semantic+structural — **partial: semantic+structural+RRF real, but no FTS, `search` is a keyword router, `node_fts`/`hybrid.rs` absent** |
| [pipeline/bootstrap-resolution.md](pipeline/bootstrap-resolution.md) | **draft** | 5-phase startup + Tauri sidecar + hardware tiers + ollama-as-soft-dep |
| [pipeline/agent-execution.md](pipeline/agent-execution.md) | **draft** | Focused specialist agents + isolation + tool restriction + reporting |
| [pipeline/library-intelligence.md](pipeline/library-intelligence.md) | **draft** | Doc ingestion + version pinning + custom-lib + skill gen + drift |
| [pipeline/collective-intelligence.md](pipeline/collective-intelligence.md) | **draft** | Federated insights via `global-dojo` — anonymisation + community promotion |
| [pipeline/testability.md](pipeline/testability.md) | **draft** | Function-shape analysis + TDD gate + decompose-before-code |
| [pipeline/benchmarks.md](pipeline/benchmarks.md) | **draft** | Reproducible benchmark runs + competitive matrix + regression detection |

---

## Reading order

Newcomer to the codebase reads: this README → [EXECUTION-PLAN.md](EXECUTION-PLAN.md) (the 5-day plan; picks up cold from a phone) → [MOCKUP-INDEX.md](MOCKUP-INDEX.md) (which mockup file to open for which screen — **never guess**) → [agents/README.md](agents/README.md) (the gate playbook) → [pipeline/analyzer.md](pipeline/analyzer.md) → [pipeline/ftr.md](pipeline/ftr.md) → any screen doc they own.
A developer picking up an autonomous task reads: [screen/{their-target}.md](screen/) → the pipeline docs it references → the mockup file listed in the front-matter → then runs the gates from the playbook.

The tie-breaker rule: **if this spec disagrees with the mockup, the
mockup wins; if the mockup disagrees with the wire API, the wire wins.**
File an update to whichever is behind.

---

## Gates — how a spec goes from draft to shipped

Every spec is executed through four gates. See
[agents/README.md](agents/README.md) for the full playbook. Short
version:

| Gate | Agent | When |
|---|---|---|
| 1. Spec review | `spec-doc-reviewer` | BEFORE coding. Confirms the doc is usable and agrees with the mockup. |
| 2. Done-gate verification | `done-gate-verifier` | AFTER coding. Runs each "Done gate" check against the live daemon. |
| 3. Wrong-gate hunt | `wrong-gate-hunter` | AFTER coding, in parallel with #2. Actively probes for each "Wrong gate" anti-pattern. |
| 4. Persona review | `sensei-persona-reviewer` | AFTER #2 + #3 pass. Independent check that the work serves each persona. |

Segment-level end-to-end runs `sensei-acceptance-tester` when a whole
segment (Bootstrap, Observatory, Project window) is claimed done.

**Autonomous execution rule:** if any gate fails, park the doc with a
note and continue to the next. Don't try to muscle a doc past a
failing gate.
