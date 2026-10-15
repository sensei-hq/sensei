# Checkpoint — 2026-10-15

**Slice:** #225 (dōjō device plane): live verification next. #238 (Claude Code mods) is built.

**Done in #238** (pushed to develop, ea4bb6e3..ab228173; each step live-verified):
- Step 2: `hooks/usage.ts` → `SessionMeasure` / `TurnComplete` events, plus `sessions.list_cost_usd`.
- Step 3: `hooks/routing.ts` + `POST /api/review/subagent-model`. The `routing.risk_models`
  config maps a risk class to a subagent model; with no map, nothing is overridden.
- Step 4: `hooks/drive.ts`, the opt-in `runDrive` option. A run's dōjō nudges are submitted
  as prompts, each once.
- Step 5: `hooks/gate.ts`, the opt-in `relayGate` option. A `tool.check` gate that only
  tightens and fails open. The shell `hooks/gate` is retired.
- PRD M3 §Usage & cost capture is the per-assistant adapter contract (gap C-25).
  The 2026-10-15 entry in `docs/decisions.md` records the policy calls.

**Remaining:** #225 live verification on the local Supabase test tenant → #225 app
per-checkout status → #268 dōjō diagram screens → #254 JPA 8b.
#238 stays open until `make bump` ships the mods (C-13 install path).

**Next command:** `/sensei:session refocus`, then start the #225 live verification
(the tenant setup is in the #225 plan, `docs/history/plan/2026-10-12-dojo-device-plane-as-person.md`).

**Open questions:**
- Does `--resume` keep the session id and reset /cost? If so, `GREATEST` under-counts.
- Should `check_hooks` WARN when the mods module is missing?

**Known-broken / pending:**
- The mods and `list_cost_usd` reach installs only at the next release.
- The committed `database/design.dbml` is stale against `dbd dbml` (~1.7k-line diff, pre-existing).
