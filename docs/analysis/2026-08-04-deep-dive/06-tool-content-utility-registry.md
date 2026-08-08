## Tool & Content Utility — the Missing Registry, the Misleading "70% Ignored", and the Un-scanned Grants
_Sensei measures tool utility with two disagreeing subsystems, scores 70% of calls "ignored" via a fragment-overlap heuristic, and has no single registry that unifies tools, skills, agents, MCP servers, and commands with provenance, version, utility, and leak status._

**User's observation.** The metrics doc calls for a "content-type registry (plugins/skills/agents/tools) with leak-scan," and notes agents forget available tooling ("reports it cannot drive e2e tests on tauri despite a working harness") while tools/skills/agents "evolve with libs+models." The observations doc flags that the app surfaces (Libraries, Patterns) show state but "no handoff/intake action." The thread underneath all of it: sensei can list tools, but it can't tell you which are worth loading, where they came from, whether they're stale, or whether they leak.

**What the data shows.**

- The verdict engine is a **fragment-overlap heuristic, not a usefulness judge.** Of 23,638 verdicts, 16,547 (70.0%) are `ignored` — but 16,007 of those (68% of the entire table) carry the reason `no fragment overlap; different target` (14,469), `no citable fragments in response; different target` (1,538), or `reaction is Stop — no downstream tool call` (490). "Ignored" means "the assistant's next message didn't quote this tool's output," which structurally penalizes fire-and-forget tools.

```sql
SELECT verdict, count(*) n, round(100.0*count(*)/sum(count(*)) OVER (),1) pct
FROM sensei.tool_call_verdicts GROUP BY verdict ORDER BY n DESC;
-- ignored 16547 (70.0%) | used 7053 (29.8%) | partial 38 (0.2%)
SELECT reason, count(*) FROM sensei.tool_call_verdicts WHERE verdict='ignored'
GROUP BY reason ORDER BY 2 DESC;  -- top 3 reasons = 16007 rows
```

- **The lowest-"utility" tools are exactly the tools whose value is never a citable fragment.** `used/(used+ignored)` per tool (≥20 verdicts) puts `ScheduleWakeup` and `StructuredOutput` at 0.000, `WebSearch` at 0.015, `TaskUpdate` at 0.016, `ToolSearch` at 0.027, `TaskCreate` at 0.043. These are scheduling/output/bookkeeping/search tools — their job isn't to produce text the model quotes back. The metric is measuring the wrong thing for half the toolbox.

| tool_name | used | ignored | utility | why low |
|---|---:|---:|---:|---|
| ScheduleWakeup | 0 | 56 | 0.000 | schedules, never cited |
| StructuredOutput | 0 | 80 | 0.000 | terminal output, no downstream |
| WebSearch | 8 | 512 | 0.015 | results paraphrased, not quoted |
| TaskUpdate | 8 | 491 | 0.016 | bookkeeping, no fragment |
| ToolSearch | 5 | 179 | 0.027 | discovery, next call cites a different target |
| TaskCreate | 12 | 267 | 0.043 | bookkeeping |
| Bash | 2392 | 8110 | 0.228 | most output is scaffolding |
| Read | 2210 | 4168 | 0.347 | reads that inform, not quote |
| Edit | 1730 | 1024 | 0.628 | genuine workhorse |
| Agent | 237 | 101 | 0.701 | best-scoring tool |

```sql
WITH agg AS (SELECT tool_name,
  sum((verdict='used')::int) used, sum((verdict='ignored')::int) ignored,
  count(*) total FROM sensei.tool_call_verdicts GROUP BY tool_name)
SELECT tool_name, used, ignored,
  round(used::numeric/nullif(used+ignored,0),3) utility
FROM agg WHERE total>=20 ORDER BY utility ASC;
```

- **Half of `tool_insights` is empty rows.** Of 21,783 insight rows, 9,103 (41.8%) are `win`, 1,962 (9.0%) are `unused`, and **10,718 (49.2%) have a NULL variant AND a NULL `signal_title`** — pure filler with no rendered signal. The "insights" surface is half noise.

```sql
SELECT count(*) total,
 count(*) FILTER (WHERE signal_variant='win') win,
 count(*) FILTER (WHERE signal_variant='unused') unused,
 count(*) FILTER (WHERE signal_variant IS NULL) blank
FROM sensei.tool_insights;  -- 21783 | 9103 | 1962 | 10718
SELECT signal_title, count(*) FROM sensei.tool_insights
WHERE signal_variant IS NULL GROUP BY 1;  -- (NULL, 10718)
```

- **The two utility subsystems openly contradict each other on the same tool.** `tool_insights` labels `Read` a `win`/"workhorse" ("15572 calls, 0% failure rate — well-oiled") — but the very same row's `metrics` jsonb records `ignoredPct: 0.735, usedPct: 0.264`, and `tool_call_verdicts` scores Read's utility at 0.347. `win` is computed from call-count + error-rate; `utility` from fragment overlap. Nothing reconciles them, so a tool is simultaneously "well-oiled" and "ignored 73% of the time."

```sql
SELECT tool_name, signal_title, metrics->>'usedPct' usedpct, metrics->>'ignoredPct' ignpct
FROM sensei.tool_insights WHERE tool_name='Read' AND signal_variant='win' LIMIT 1;
```

- **Insight generation duplicates itself.** The same dormant tool gets both a generic `Dormant tool` row ("No calls in the last 31 days — is this tool still needed?") and a tool-specific `browser_navigate: dormant` row ("…wire it into a skill or persona, or archive it"). `win` similarly splits into generic `Workhorse tool` (862) and per-tool `svelte-autofixer: workhorse` (768) etc. Two templates, same event — inflating the table and the UI.

- **The registry is real but shallow, and claude-only.** `assistant_tools` holds 114 rows, all `assistant_family='claude'`: 90 `mcp` tools across 4 servers + 24 `builtin`. It has provenance (`source_type`, `source_key`, `server_id`) and an `invoked_name`, but **no utility, no version, no leak-scan, no last-used** columns. MCP inventory: 6 `mcp_servers`, of which 4 are live (sensei 55 tools, Playwright 24, Semgrep 7, Svelte 4) and **2 are dead entries** (`mcp-server-context7`, `postgres-context-server` — `connection_state=error`, empty `command`, imported from a Zed settings file).

| server | tools | state | note |
|---|---:|---|---|
| sensei | 55 | connected | plugin_sensei_sensei |
| Playwright | 24 | connected | npx @playwright/mcp@latest |
| Semgrep | 7 | connected | semgrep mcp |
| Svelte MCP | 4 | connected | npx @sveltejs/mcp |
| mcp-server-context7 | 0 | **error** | no command configured |
| postgres-context-server | 0 | **error** | no command configured |

```sql
SELECT assistant_family, source_type, count(*), count(DISTINCT server_id) servers
FROM sensei.assistant_tools GROUP BY 1,2;
SELECT server_name, tool_count, error FROM sensei.mcp_tool_manifests ORDER BY tool_count DESC;
```

- **Utility, insights, and registry don't share a key.** Verdicts cover 64 distinct `tool_name`s, insights cover 94, the registry holds 114 — and there is no foreign key between them; utility is keyed on a bare invocation string. Only 54 verdict names join the registry on `invoked_name`; **60 of 114 registered tools (52.6%) have zero verdict/utility data.** The entire Semgrep server (7 tools), most sensei MCP tools (`plan`, `consensus`, `save_memory`, `record_outcome`…), and every Playwright interaction tool have never been scored — so utility-informed selection is impossible for them.

```sql
SELECT count(*) registered, count(*) FILTER (WHERE v.tool_name IS NULL) no_verdict
FROM sensei.assistant_tools a
LEFT JOIN (SELECT DISTINCT tool_name FROM sensei.tool_call_verdicts) v
  ON v.tool_name = a.invoked_name;  -- 114 | 60  (52.6%)
```

- **Tool identity is not stable across plugin/model migrations — and utility is silently split across the aliases.** The 10 verdict names that don't match the registry (149 verdicts) are all *pre-migration* identities of tools that still exist. The verdict log records `mcp__plugin_svelte_svelte__svelte-autofixer` (116) while the registry stores the same tool as `mcp__svelte__svelte-autofixer`; verdicts log `mcp__playwright__browser_navigate` while the registry has `mcp__plugin_playwright_playwright__browser_navigate`. svelte-autofixer's utility is fractured across two names (55 verdicts under one, 116 under the other), so neither reflects the tool's real usage.

```sql
SELECT v.tool_name, count(*) verdicts FROM sensei.tool_call_verdicts v
LEFT JOIN sensei.assistant_tools a ON a.invoked_name=v.tool_name
WHERE a.id IS NULL GROUP BY 1 ORDER BY 2 DESC;  -- 10 stale identities, 149 verdicts
```

- **Content beyond tools is registered in isolation, with a single flat provenance value.** `library_skills` (4) and `library_agents` (2) are 100% `source='manifest'` — there is no "insight-generated" or "sensei-provided" provenance dimension anywhere, even though the system mints 943 patterns / 1,478 recommendations that *could* become skills. `project_commands` (361) is a fourth silo, folder-scoped by ecosystem (npm 211 across 30 folders, cargo 104/13, maven 46/8). Five tables, five vocabularies, no join.

- **Leak scan: clean today, but entirely unmonitored.** A regex sweep for `sk-…`, `ghp_`, `AKIA…`, `-----BEGIN`, `xox[baprs]-`, `Bearer …`, and inline `password:"…"` over the manifest `tools` jsonb returns **0 hits**; **0 of 6 `mcp_servers` have a populated `env`**; the only 2 description matches (`resolve_risk_class`, `get-documentation`) are prose that mention the words "token"/"api key," not secrets. Good — but there is **no `leak_scan_status` column, no scan cadence, and no over-broad-grant check** on any of these tables, so this is luck, not a control. The one server field that *would* carry secrets (`mcp_servers.env`) is exactly the field with no scan attached to it.

```sql
SELECT count(*) FILTER (WHERE tools::text ~* '(sk-[a-z0-9]{20}|ghp_|AKIA[0-9A-Z]{16}|-----BEGIN|xox[baprs]-)')
FROM sensei.mcp_tool_manifests;                                   -- 0
SELECT count(*) FROM sensei.mcp_servers WHERE env <> '{}'::jsonb; -- 0
```

**Root cause / interpretation.**

Sensei grew *two* utility subsystems bottom-up and never unified them. `tool_call_verdicts` answers "did the model quote this tool's output in its next turn?" — a cheap fragment-overlap heuristic computed from `assistant_events` reactions. `tool_insights` answers "is this tool high-volume and low-error, or dormant?" — a call-count/error-rate roll-up. Both are reasonable signals in isolation, but they were bolted to a bare `tool_name` string with no shared identity, no shared window, and no arbitration. The result is a table that calls Read a "workhorse" and "70% ignored" in the same row, and a headline "70% of tool calls ignored" that is mostly an artifact of scheduling, bookkeeping, and terminal-output tools whose value is *never* a quoted fragment. Before any of these numbers can drive product decisions, "utility" needs a definition that credits a tool for the *state it changes or the decision it unblocks*, not for showing up as a substring in the next assistant message.

The deeper gap is that there is no registry — there are five registries. `assistant_tools`, `mcp_servers`/`mcp_tool_manifests`, `library_skills`, `library_agents`, and `project_commands` each model a slice of "things the assistant can invoke," each with its own primary key, its own provenance word, and no cross-links. Utility (`tool_call_verdicts`/`tool_insights`) is a sixth silo keyed on yet another string. Nothing ties `mcp__plugin_svelte_svelte__svelte-autofixer` the runtime invocation to `svelte-autofixer` the registered tool to `Svelte MCP` the server to `@sveltejs/mcp@0.0.1` the version. So when a plugin namespace changes (`mcp__svelte__*` → `mcp__plugin_svelte_svelte__*`) or a model's tool set shifts, the accumulated utility history is orphaned and the tool looks brand-new again. This is precisely the "tools/skills/agents evolve with libs+models but there's no registry" failure in the metrics doc, and it's why an agent can "forget" a working harness: the tool is present in `assistant_tools` but carries no salience signal (no recent-use, no win-rate the selector can rank on), so nothing surfaces it at the moment of need.

Provenance and safety were never modeled at all. `library_skills.source` and `library_agents.source` are hardcoded to `'manifest'`; there is no way to express "this skill came from a detected pattern," "this agent is sensei-provided," or "this tool is a third-party MCP." Without a provenance dimension, sensei can't reason about trust, can't prioritize sensei-owned content over library-derived, and can't route a "handoff/intake action" (the observations-doc complaint) because it doesn't know what kind of thing it's handing off. And because no table carries a leak-scan status or a grant scope, the fact that today's manifests are secret-free is invisible to the product — there's nothing to display as "verified clean," nothing to re-check when a manifest's `probed_at` refreshes (TTL 900s), and no gate that would catch a future MCP server that ships an `env` full of API keys.

The two dead `mcp_servers` (context7, postgres) are the small tell that closes the loop: the registry ingests config from external files (Zed settings) and never reconciles it. It accumulates entries but has no lifecycle — no "connected → error → archive," no staleness eviction, no dedup across plugin renames. It's an append log wearing a registry's schema.

**Recommendations.**

1. **(P0) Redefine tool utility as an outcome signal, then reconcile the two subsystems into one score.** In the daemon module that writes `tool_call_verdicts`, stop treating "no fragment overlap" as `ignored` for tools whose contract is non-textual (scheduling, task bookkeeping, structured output, discovery). Classify by *effect*: did a downstream `PostToolUse`/edit/state-change follow (`assistant_events`), did it unblock the plan phase (`activity.run_events`)? Fold the `tool_insights` win/error signal and the verdict signal into a single `utility_score ∈ [0,1]` stored once per (tool_identity, window). Expected effect: the "70% ignored" headline stops being a heuristic artifact and starts predicting which tools to load.

2. **(P0) Introduce a stable `tool_identity` and back-key verdicts/insights to it.** Add a canonical id on `assistant_tools` (e.g. `server_name + '/' + tool_name`, migration-stable) and an alias table mapping historical invocation strings (`mcp__svelte__*`, `mcp__plugin_svelte_svelte__*`) to it. Rewrite `tool_call_verdicts.tool_name` / `tool_insights.tool_name` to reference it. Expected effect: svelte-autofixer's 171 split verdicts merge; the 60 unscored registered tools become addressable; utility survives plugin/model renames.

3. **(P1) Build one `content_registry` view/table unifying tools, skills, agents, commands, MCP servers with a shared `provenance` and `lifecycle`.** Columns: `kind` (tool|skill|agent|command|mcp_server), `provenance` (sensei_provided|library_derived|insight_generated|user_config|third_party_mcp), `version`, `utility_score`, `last_used_at`, `lifecycle` (active|dormant|error|archived), `leak_scan_status`. Populate provenance properly (replace the hardcoded `library_skills.source='manifest'`; mark sensei's 55 MCP tools `sensei_provided`; mark the 2 dead servers `error`→`archived`). This is the "content-type registry with provenance" the metrics doc asks for and the join the Libraries/Patterns app screens need to render a handoff/intake action.

4. **(P1) Add a leak-scan + grant-scope gate on `mcp_servers.env`, `mcp_tool_manifests.tools`, and `library_skills/agents.body`.** Run the secret-pattern sweep (already clean: 0 hits) on every manifest re-probe (TTL 900s) and every library sync, write `leak_scan_status` + `scanned_at`, and flag any MCP server whose tool grants exceed a policy (e.g. `browser_run_code_unsafe`, `execute_sql`). Expected effect: "verified clean" becomes a displayable, re-checked control instead of luck.

5. **(P1) Feed `utility_score` + `last_used_at` into tool selection / context packing.** The MCP `context_pack` / `get_layered_context` path should rank and surface high-utility, recently-successful tools for the task at hand, so agents stop "forgetting" a working harness (Playwright's 24 tools score 0.0–0.18 today largely because they're never resurfaced). Expected effect: fewer "I cannot drive e2e on tauri" false negatives.

6. **(P2) Collapse duplicate insight rows and drop the 10,718 NULL-variant filler.** In the insight generator, emit one row per (tool_identity, signal) — not both a generic and a tool-specific template — and don't persist rows with NULL variant+title. Expected effect: the insights surface goes from 49% noise to signal-only, and the app's Patterns/Insights screens become readable.

7. **(P2) Add registry lifecycle reconciliation tied to library/model changes (staleness detection).** When a library version or a model's tool set changes, mark dependent registry rows `stale` and re-probe; evict `error` servers after N failed probes. Expected effect: dead entries (context7, postgres) don't linger; renamed tools reconcile instead of forking history.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|---|---|---|---|---|
| Tool utility score (v2) | `used/(used+ignored)` re-scored so non-textual tools credit downstream effect, merged with win/error | `sensei.tool_call_verdicts.verdict` + `tool_insights.metrics` | session + 14-day window | today two disagreeing scores; heuristic penalizes fire-and-forget tools |
| Registry utility coverage | `1 − no_verdict/registered` | `assistant_tools.invoked_name` ⋈ `tool_call_verdicts.tool_name` | daily | **52.6%** (60/114) tools unscored; no shared key |
| Unused-tool count | tools `unused`/dormant in window | `tool_insights` WHERE `signal_variant='unused'` | daily | 1,962 signals, duplicated (generic+specific rows) |
| Insight signal density | `1 − blank/total` | `tool_insights.signal_variant IS NULL` | daily | **49.2%** blank NULL-variant/NULL-title rows |
| Identity stability | verdict names resolving to a canonical tool id | `tool_call_verdicts.tool_name` ⋈ alias map | on plugin/model change | 10 stale identities (149 verdicts) orphaned; no alias table |
| Registry provenance coverage | rows with non-default `provenance` / total | proposed `content_registry.provenance` | daily | provenance is hardcoded `'manifest'` / absent |
| Leak-scan pass rate | scanned rows with `leak_scan_status='clean'` / scanned | proposed col on `mcp_servers`,`mcp_tool_manifests`,`library_*` | per manifest re-probe (TTL 900s) + library sync | **no column exists**; 0 secrets today but unmonitored |
| MCP server health | `connected/total`, dead-entry age | `mcp_servers.connection_state`, `last_seen_at` | daily | 2/6 in `error` with no command; never reconciled |
| Content staleness | rows whose lib/model version drifted from `version_used` | `library_skills.version_range`, `referenced_libraries.version_used` | on library/model change | no drift check wired to registry |
