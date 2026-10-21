# Instruments · M18

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Instruments are the workbench: try a tool by hand (Playground), read what actually
happened in a session (Replay), classify whether each tool call was **used**,
**partial** or **ignored** (Verdicts), and watch the tool fleet's health (tools-health
grid) and per-tool signals. The job is to turn raw tool telemetry into something a
developer can reason about — *did sensei's advice land, or was it ignored?* — and to
give a hands-on way to poke a tool without editing code. The user feels this as *I can
see exactly what sensei did and whether it worked.*

> **Status honesty.** Replay, the verdict classifier, the tools-health grid, the MCP
> server/tool listing and tool-signals are built. **The benchmark runner is not
> built** — there is no runner code and no test (gap [C-5](../../../../reorg.md) →
> M18). The tools-health grid (`tools_health.rs`) and the MCP server tool listing
> (`mcp_servers.rs`) have **no dedicated test** (marked `NO TEST` in the features);
> the grid is exercised only through the app. The screen spec
> `spec/screen/instruments-*` names a dedicated Benchmarks tab — it is a placeholder
> until C-5 lands.

## In scope / out of scope

- In: Playground (call an MCP tool and read the raw response); Replay (a session's
  tool/response timeline with per-call verdicts); the verdict classifier and its
  list/summary; the tools-health grid + refresh; per-tool signals (usage → unused /
  warn / opportunity / win, curated); and the MCP server/tool listing.
- Out: the code graph and the Code map (M17); per-workspace Impact (M19); the planner (M20);
  **the benchmark runner (C-5, not built)**; the mobile relay surfaces (M21).

## Surfaces

- **Routes** — `POST /api/instruments/verdicts/classify` · `GET /api/instruments/verdicts`
  · `/verdicts/summary` · `GET /api/instruments/mcp-servers/{id}/tools` ·
  `GET /api/instruments/tools-health` · `POST /api/instruments/tools/refresh` ·
  `GET /api/sessions/{id}/replay` · `/tool-timeline` · `GET /api/mcp/tools` ·
  `POST /api/mcp/call` · `GET /api/observatory/tool-{usage,signals,insights}`.
- **Daemon** — `api/handlers/{verdicts.rs, tool_signals.rs, tools_health.rs,
  mcp_servers.rs, sessions.rs, mcp.rs, observatory.rs}`;
  `tasks/verdict_classifier.rs`; `db/pg_store/tool_calls_tests.rs` (test helper —
  the real store code is in `db/pg_store/session.rs`).
- **App** — `(project)/project/[id]/instruments/{playground,replay,health}/+page.svelte`.
- **Data** — `sensei.tool_call_verdicts`, `activity.assistant_events`,
  **`session_tool_calls` (view)**, `sensei.mcp_servers` / `mcp_tool_manifests`.
  **No `activity.tool_calls` table** — the telemetry lands in a view.

## Data invariants

- **Lead.** A verdict is exactly one of `used` / `partial` / `ignored`; the wire
  serialises the variant lowercase.
- **Lead.** A verdict shows on its own call. `cat a_verdict_shows_on_its_own_call_in_the_replay_timeline`.
- **Lead.** A call is paired with its *own* response and the engine's duration — a
  duplicated capture is one call, a missing id is unpaired, never guessed.
- **Lead.** Signals are curated, not raw. Multiple dormants collapse into one summary;
  ordering is warn → opportunity → unused → win.
- **Lead.** The verdict classifier reads the reaction to a call: a citation of a
  fragment → `used`; same target, no citation → `partial`; next event is a user prompt,
  differing targets, or no reaction → `ignored`.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Playground call | request / response pair | tool name + raw JSON |
| Replay timeline | tool call and its response | `Read · 12ms · used` |
| Verdict | used / partial / ignored | `partial` |
| Unused signal | no activity in two weeks | `sensei__search — unused 14d` |
| Warn signal | high failure rate | `sensei__index — 38% failure` |
| Opportunity | moderate traffic, high errors | `sensei__impact — try first` |
| Win | high traffic, clean | `sensei__search — clean, 412 calls` |
| Health grid | per-tool status | servers × tools grid |

## Done gate

- Playground issues a tool call and renders the response (including errors).
- Replay shows a session's calls, responses, durations and verdicts in order.
- `POST /verdicts/classify` writes `used` / `partial` / `ignored`; the list and summary
  read them back.
- The tools-health grid renders from real data; refresh re-reads.
- Tool signals render curated (a dormant fleet collapses to one summary, not N rows).

```bash
curl -s "http://localhost:7744/api/sessions/$SID/replay" | jq '.calls[0] | {tool, verdict}'
curl -s http://localhost:7744/api/instruments/verdicts/summary | jq
```

## Wrong gate

- **A verdict is not one of the three** — the enum leaked a raw string.
- **A verdict appears on the wrong call's row** — pairing broken.
- **A duplicated capture counts twice, or a missing id is guessed** — pairing broken.
- **Raw dormants instead of one curated summary** — curation regressed.
- **Playground swallows the error and shows an empty 200.**
- **The health grid renders from a static list, not live data.**
- **A Benchmarks tab claims to run benchmarks** — C-5 is unbuilt; it must read as a
  placeholder.

## Scenarios

- [`scenarios/instruments.feature`](scenarios/instruments.feature) — Playground,
  Replay, the verdict classifier and the signal curation (pinned).
- [`scenarios/instruments-health.feature`](scenarios/instruments-health.feature) — the
  health grid, MCP listing and the unbuilt benchmark runner (`NO TEST` / `# gap: C-5`).

## Open gaps

- [C-5](../../../../reorg.md) — **benchmark runner unbuilt** (no code, no test) → M18.
- [M-7 / M-8 / M-9](../../../../reorg.md) (M8) — no registered metrics read tool-usage
  / verdict / session data; Instruments is the input.

## Related

- [spec/screen/instruments-*](../../../../spec/screen/) — playground, replay, health,
  benchmarks (legacy depth, folds here).
- [features/03-observatory.md](../../../../features/03-observatory.md) — tool-signal
  lane (legacy depth).
- [prd/02-objectives.md §05](../../../02-objectives.md) · [prd/03-personas.md §1](../../../03-personas.md#1--solo-developer--the-wedge).
- Crates: `crates/senseid/src/{api/handlers/verdicts.rs, api/handlers/tool_signals.rs,
  api/handlers/tools_health.rs, api/handlers/mcp_servers.rs, api/handlers/sessions.rs,
  tasks/verdict_classifier.rs, db/pg_store/session.rs}`.
