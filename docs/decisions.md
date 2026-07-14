# Decisions log — considered · adopted · discarded · deferred

> When a doc is superseded and archived, the *ideas* worth keeping land here so
> nothing useful is lost and nothing already-rejected gets re-proposed. This is
> the memory of **why the shape is the shape**. Additive — append, don't rewrite
> history. Pairs with [`requirements/open-issues.md`](requirements/open-issues.md)
> (what's left to build) and [`backlog.md`](backlog.md) (the issue index).

## Adopted — concepts carried forward (from `archive/ideas/`, `design/`)

| Concept | Where it lives now |
|---|---|
| The **four-segment journey** (Bootstrap · First-run+Prefs · Observatory · Project) | [requirements/vision.md](requirements/vision.md), [objectives.md](requirements/objectives.md) |
| **Value before setup** — projects first, not a wizard | vision theme 1; realised as first-run-scan + Preferences |
| The **module lifecycles** (the loops inside the daily app) | objectives O5 |
| **FTR** as the single north-star | vision.md |
| **Adapter-IR** + language-adapter split; task hierarchy + barriers; compression L0–L3 | [architecture/daemon.md](architecture/daemon.md) (+ `architecture/reference/`) |
| **Single binary, single DB, port 7744** | architecture/data.md + daemon.md |
| The **retrospective-loop** framing (capture→graph→analyze→learn→deliver→measure) | vision.md core loop |
| Dōjō **priority-ladders / specificity-wins / pull-never-push / preview-always** | [architecture/dojo.md](architecture/dojo.md), objectives DJ* |

## Discarded — considered and rejected (do not re-propose)

| Idea | Why rejected | Superseded by |
|---|---|---|
| **10-step linear setup wizard** (old `ideas/02-setup.md`) | A first-run gauntlet violates *value before setup*; users abandoned it | first-run-scan (value immediately) + a searchable **Preferences** surface |
| **dev/prod split** (two binaries, port 7745) | Needless complexity; one machine, one dataset | **single mode** — one binary, one DB, :7744 |
| **Gateway as an in-tree crate** (`crates/gateway/`) | Couldn't release to crates.io independently | **`gateway-embedded`** git dep (sibling repo `sensei-hq/gateway`) |
| **`hive-mind` / `sensei-hive` naming** | Ambiguous; drifted from the product concept | **Dōjō / `sensei-dojo`** (`crates/dojo-mind`) — terminology migration in progress |
| **Push-based federation** | Leaks control + confidentiality | **pull, never push** + preview-always (theme 5) |
| **Global collective as the default share lane** | Org/client work must not hit the public commons | the **Dōjō membership** boundary is the default; Collective is explicit opt-in |
| **`client-lead` role name** | Longer than needed; the engagement (not the person) is the "client" | role renamed to **`lead`**; it still guards *client engagements* |

## Deferred — good, not now (parked with a trigger)

| Idea | Why deferred | Revisit when |
|---|---|---|
| **User-facing learnings** ("you tend to give sparse instructions for schema changes…") | The pair-both-ways signal needs the loop closed first | after Phase 1 (FTR loop closes) |
| **Assistant proactive clarification** ("I need the migration policy before I can answer") | v2 behaviour; needs the signal + a prompting contract | [pipeline/clarification-prompting](spec/pipeline/clarification-prompting.md) — post-Phase 2 |
| **Benchmarks** (runner + corpus) | No runner/TaskKind yet; not on the FTR path | when model-effectiveness needs a controlled corpus |
| **Testability** (test-runner adapter → `test_pass_rate`) | No test-runner integration; quality signal optional | when a quality dimension is prioritised |
| **Diagnostic sessions/traces + issue export** (#39) | Larger new-schema + cross-cutting capture effort; only flat `public.logs` today | when support/debug UX is prioritised |
| **Image-gen as seed** (#77) · **embedded in CI release binaries** (#78) | Need `model_capability=image` / cross-platform native llama.cpp sign-off | gateway/seed hardening pass |
| **Dōjō live activation** | External-blocked (needs a remote server + SaaS-infra decision) | Phase 4 |
| ~~`llm-spec/` → `spec/` rename~~ **DONE 2026-07-14** | was high-churn (run-state/cron driver + gate agents + memory) | completed: dir renamed + all referrers fixed (docs, `.claude/agents`, code comments) |

## How to use this

- Rejecting an idea? Add a **Discarded** row with the reason — future-you (and the
  next assistant) won't waste a cycle re-proposing it.
- Parking an idea? Add a **Deferred** row with an explicit *revisit-when* trigger.
- Promoting a deferred idea to active work? Move it into
  [open-issues.md](requirements/open-issues.md) and note the date here.
