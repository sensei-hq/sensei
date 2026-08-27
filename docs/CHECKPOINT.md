# Checkpoint

**Slice:** thematic retrospectives — repo and cross-repo (`docs/spec/2026-08-26-thematic-retrospectives.md`)

## Done

- **Spec written and twice corrected by real data.** Grounded in the live DB: the 29-metric catalogue, `metric_facts` (15,636), LLM-authored `insight_copy` cached by `facts_hash`, and insight→rule/skill/agent materialization all already exist. This is new *copy over existing facts*, not a second stack.
- **P1 stage attribution — DONE and deployed.** `sensei.work_stage` + `stage_source` on `activity.session_facets`; inference is a fifth key in the call the process analyzer already makes. Validated on the **release** daemon: **86 sessions staged from real transcripts**.
- **Leak guard** (`.githooks/check-no-leaks.sh`) wired into pre-commit, 11 self-tests.
- **History redacted** across all three public repos; single identity `Sensei HQ <hi@sensei-hq.com>`.

## Measured (86 sessions, release binary)

| stage | n | deviated | shallow | corrections | depth |
|---|---:|---:|---:|---:|---:|
| build | 26 | 38% | 31% | 5 | 3.2 |
| analyze | 26 | 38% | 23% | 8 | 3.3 |
| plan | 19 | 26% | 21% | 3 | 3.7 |
| verify | 7 | 29% | 29% | 3 | 3.7 |
| fix | 6 | 33% | 17% | 0 | 3.0 |
| operate | 2 | — | — | 0 | 3.5 |
| explore | 1 | — | — | 0 | 2.0 |

## Next

1. **`#125` — Zed ingest is broken and self-sealing.** 176 Zed sessions, 2 analyzable. 174 watermarks claim turns that do not exist, so ingest never retries; 46 of 48 turn sets are orphaned from any session. **Largest single lever on every transcript-derived metric** — fixing it roughly triples the analyzable pool (109 of 287 today).
2. **Collapse `explore` into `analyze`** — 1 session in 86; open question 1 is answered by data.
3. **P2 — repo retrospective**: T1/T3 cross-repo first. Repo × stage is still too sparse (7 of 31 cells reach n≥5).

## Known-broken / caveats

- 23 sessions still queued for the analyzer; the daily pass will drain them.
- The process pass is **daily-only** by design — use `POST /api/projects/{id}/process/analyze` to drain on demand.
- Stage rollups must state `stage_source` (all `inferred` today) and the grain measured (§6a) — a repo-level pattern is not a session-level mechanism.
