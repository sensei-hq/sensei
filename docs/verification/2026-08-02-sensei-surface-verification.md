# Sensei surface verification — 2026-08-02

> Live verification of every sensei-exposed MCP tool + skill + agent + command against the running
> daemon (`localhost:7744`) + `psql sensei`. Bar: a 200 is NOT a pass — reads must return real,
> shape-correct data (verified vs psql); writes must persist (read-back); hardcoded/fabricated/noop = FAIL.
> Ran as a 12-agent verification workflow. **111 checks · 79 pass · 25 degraded · 6 FAIL · 1 smoke-only.**

## Ranked defects (fix list)

| # | Surface | Severity | Layer | Symptom | Root cause (file:line) |
|---|---|---|---|---|---|
| 1 | `log_event` + 14 commands + 2 skills | HIGH | mcp | Returns canned `{"ok":true,"noop":"events sink retired (#68)"}`; no sink table; MANDATORY step persists nothing | `mcp/src/main.rs:394`; desc `lib.rs:638` |
| 2 | `run_checkers`, `promote_memory` (FAIL) + `get_workflow_state` default (degraded) — #109 | HIGH | mcp | MCP forwards cwd as `folder`; daemon prefers folder over `project` → 404/400 even with `project:sensei` | `lib.rs:278,463,687-693`; daemon `knowledge.rs:1068,495` |
| 3 | `accept_playbook_rule` | HIGH | daemon | Nonexistent UUID returns `{accepted:id}` while psql shows 0 rows changed — fabricated success | `pg_store.rs:10007`; `playbook.rs:314` (no rows_affected check) |
| 4 | `get_duplicates` | MED | daemon | `count:0` at every threshold over 4520 fns — always-empty mask | `find_duplicates_scoped` `a.folder_id != b.folder_id` + monorepo rolls all fns to repo-root folder |
| 5 | `get_pattern_for` | MED | daemon | `{pattern:null}` for ANY symbol — matches a `members` array never SELECTed | `codebase.rs:266-271` vs `pg_store.rs:3524` |
| 6 | `list_library_skills/agents`, `get_library_skill` | MED | daemon | `[]`/404 system-wide; rokkit's `sensei.library.json` (4 skills/2 agents) never ingested | `libraries.rs:257-268` (manifest ingest only on LocalDir; rokkit indexed w/o local_path) |
| 7 | `update_session` | MED | daemon | `{ok:true}` but summary/tokensIn/tokensOut/cost silently dropped (columns stay NULL) | `mcp.rs:136-155`, `sessions.rs:220`; schema `lib.rs:556` |
| 8 | `session.md` status | LOW | command | "Patterns: 0" always — `get_patterns(pattern='')` filters by tag → `[]` | authoring bug; real count in `get_project_conventions.patterns_total` |
| 9 | `generate_image` | LOW | config | 502 "openai missing API key" — reachable, routes right, fails honestly (no fabricated path) | `OPENAI_API_KEY` unset in daemon env (config, not code) |
| 10 | REST `GET /api/libs/*/docs` | LOW | daemon | dead routes: ignore `component` / return libraries not pages (MCP tools bypass them) | `libraries.rs:82,94` |
| 11 | `sensei-persona-reviewer` agent | LOW | agent | step 4 says `git diff` but no Bash grant | `sensei-persona-reviewer.md:44` vs `:23` |
| 12 | `analyzer` skill + `help.md` | LOW | skill | stale disambiguation vs a removed 'analyze' health-check skill | `analyzer SKILL.md:8`; help.md analyzer row |

## Passing (79)

Codebase/graph reads (search, get_callers/callees, communities, commands, project_summary/conventions,
patterns, match_pattern, user_for_project — all counts verified vs psql). Governance/memory (get_rules,
resolve_risk_class, set_stance persisted, playbook proposals list, pending_nudges, intake_guide,
recommend_playbook, save/propose/accept/reject_memory full round-trip persisted, get_layered_context,
context_pack). Libs (get_lib_docs, search_lib_docs, add_library write+ingest). Session/run (create_session,
pause_run, run_status, report/record_outcome, update_task_status — persisted). Projects (list/find/use). Gateway
(gateway_status, infer, consensus 3-model MOE, embed 384-dim real vector). Plan (plan, register_plan, update_phase).
Plus 21 skills, 9 agents, 7 commands with valid frontmatter + all refs resolving.

## Fix approach (functional tests, not call-success — per standing rule)

Every fix: extract the real logic to a **pure function** where possible, RED→GREEN with a test that asserts
**real behavior/persistence** (a regression must fail it), zero-errors, then reinstall + re-verify live.
Batched by crate to minimize rebuilds: **daemon (senseid)** fixes → **MCP (sensei-mcp)** fixes → **log_event
revive** (the one DDL/approve-class item — revive per user directive, not remove) → **markdown** (skills/agents/
commands) → **config** (generate_image key). Re-run the verification workflow at the end.
