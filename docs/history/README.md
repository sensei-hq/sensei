# History — the small archive live docs still cite

This tree keeps **only** superseded documents that a live doc or source comment
still sends readers to. Everything else that used to live here was deleted on
2026-10-21 at the user's request; it lives in git history:

```bash
git log --diff-filter=D --name-only -- docs/history   # what was deleted, and when
git show <commit>^:docs/history/<path>                # read a deleted file
```

The reorg that created this tree: [`reorg.md`](../reorg.md) (2026-10-13).

## What is kept, and why

| File | Why it is kept |
|---|---|
| [`analysis/2026-07-24-design-journey.md`](analysis/2026-07-24-design-journey.md) | cited by `../decisions.md` as the narrative behind the early decisions |
| [`analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md`](analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md) | the measured regression evidence cited by the code-graph spec and the M4 PRD |
| [`analysis/2026-08-05-indexer-capability-coverage.md`](analysis/2026-08-05-indexer-capability-coverage.md) | the indexer capability roadmap cited by the code-graph, patterns, semantic-search and traceability specs |
| [`analysis/graph-coverage-gaps.md`](analysis/graph-coverage-gaps.md) | cited by `../design/indexer.md` as background it does not repeat |
| [`analysis/graph-recovery-plan.md`](analysis/graph-recovery-plan.md) | cited by `../design/indexer.md` as background it does not repeat |
| [`analysis/metric-explainability-generation.md`](analysis/metric-explainability-generation.md) | the decision record for the metrics explainer (cited from `explainer.rs`) |
| [`backlog/writeups.md`](backlog/writeups.md) | full write-ups of past findings, linked from `../backlog.md` |
| [`design/2026-07-27-dojo-relay-rls-membership-function.md`](design/2026-07-27-dojo-relay-rls-membership-function.md) | the relay RLS membership-function decision, cited by `owns_membership.ddl` and the dōjō specs |
| [`plan/2026-07-27-data-model-fix-impact-register.md`](plan/2026-07-27-data-model-fix-impact-register.md) | the impact register cited by the dōjō screen specs |
| [`plan/2026-07-27-dojo-inbox-mockup-fidelity.md`](plan/2026-07-27-dojo-inbox-mockup-fidelity.md) | the element→data map cited by the dōjō inbox spec |
| [`plan/2026-10-13-indexer-full-call-placement.md`](plan/2026-10-13-indexer-full-call-placement.md) | the plan the indexer's compiler-session and placement code cites |
| [`plan/decisions.md`](plan/decisions.md) | parked and deferred decisions cited from daemon, dōjō and DDL comments |
| [`plan/operating-model.md`](plan/operating-model.md) | the operating-model spec the design docs build on |
| [`plan/relay-engine.md`](plan/relay-engine.md) | the end-to-end relay spec cited by the relay routes, protocol crate and design docs |
| [`plan/relay-p4-prod-runbook.md`](plan/relay-p4-prod-runbook.md) | the relay production runbook; kept while it is confirmed whether its production steps (the VAPID Web Push secret) are live |
| [`reports/2026-10-13-placement.md`](reports/2026-10-13-placement.md) | the placement numbers cited by `../design/indexer.md` |
| [`requirements/repository-sharing.md`](requirements/repository-sharing.md) | the repository-sharing model, still the mandatory first read for sharing/visibility (cited by the sync code and dōjō specs) |

## The rule

A superseded doc that no live doc cites is deleted, not archived — git history
holds it. A doc moves here only while something live still points at it; when
the last citation goes, the file can go too.
