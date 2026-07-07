# 信 · Pipeline · Health signals

**Owner file:** `crates/senseid/src/api/handlers/tool_signals.rs`
**Called by:** `AggregateToolInsights` task (persistence) + `GET /api/observatory/tool-signals` (featured list)
**Rebuild landed:** 2026-07-07 (this session)

## Purpose

Turn the flat per-tool aggregate (`sensei.tool_usage_stats`) into two
distinct products:

1. **Per-tool signals** — one signal per tool that deserves one,
   persisted to `sensei.tool_insights`. Used by the per-tool detail
   pane. Every row that would trip a threshold becomes a row.
2. **Curated featured insights** — a small, selective list returned to
   the Instruments Health Insights strip. Same source, different
   audience: the Insights strip is glanceable, so warns and
   opportunities pass through per-tool (they need action per-tool),
   but dormants and wins **collapse to a single summary card each**
   when there are more than one.

The old shape returned every dormant tool as its own card. On a
40-tool registry with 35 dormants that was 35 cards of noise. The
curation step is the fix.

## Data invariants

- `sensei.tool_usage_stats` (view) has one row per tool ever called,
  with `call_count`, `error_count`, `avg_duration_ms`,
  `last_used_at`, `error_rate` (derived).
- `sensei.tool_insights` (table) is append-only. `AggregateToolInsights`
  writes one row per tool per tick; readers pick the latest via
  `DISTINCT ON (tool_name) ORDER BY tool_name, computed_at DESC`.
- Verdict split (`used_count`, `partial_count`, `ignored_count`) folds
  into each row's `metrics` from `get_verdict_split_per_tool(14)`.
- The wire shape from `GET /api/observatory/tool-signals` is
  `{ signals: [{ tool_name, variant, title, detail, action? }], source: "derived" }`.
  Tool_name is `""` for summary cards.

## Signals derived

### Variants

| Variant | Trigger | Per-tool or summary |
|---|---|---|
| `warn` | `call_count >= 50 && error_rate >= 0.05` | per-tool |
| `opportunity` | `call_count >= 10 && error_rate >= 0.05` (moderate-traffic) | per-tool |
| `unused` | `call_count == 0` OR `days_since_last_use >= 14` | summarised when >1 |
| `win` | `call_count >= 50 && error_rate <= 0.02` | summarised when >1 |

Thresholds live in `SignalThresholds` — one struct, one edit to tune.

### Copy patterns (single tool)

| Variant | Title | Detail | Action |
|---|---|---|---|
| warn | `{short}: {rate}% failure rate` | `{calls} calls, {errors} errored. High-traffic tool with sharp edges — fix these first.` | `Edit tool: {tool}` |
| opportunity | `{short}: room to improve` | `{calls} calls, {rate}% failure. Modest volume — small polish would pay off.` | `Edit tool: {tool}` |
| unused | `{short}: dormant` | `No calls in the last {n} days ({calls} total). Either wire it into a skill or persona, or archive it.` OR `Registered but never called. …` | `Trace: why is {short} unused?` |
| win | `{short}: workhorse` | `{calls} calls, {rate}% failure rate — well-oiled.` | none |

### Copy patterns (summary — 2+)

| Variant | Title | Detail | Action |
|---|---|---|---|
| unused | `{n} tools dormant` | `{tool1}, {tool2}, {tool3} and {n-3} more haven't been called in the last two weeks. Either wire them into a skill or persona, or archive them.` | `Review tool registry` |
| win | `{n} workhorse tools` | `{tool1}, {tool2}, {tool3} are running high-volume with clean error rates.` | none |

### Short-name rule

`short_name(tool)` strips known prefixes: `sensei.` or `mcp__…__`.
Falls back to the raw string. Enforced in unit tests.

### Sort order

Curated output is sorted **warn → opportunity → unused → win**.
Actionable first. Raw output from `derive_signals` is unsorted; the
sort is inside `curate_insights`.

## Done gate

- `cargo test -p senseid tool_signals` passes all 13 unit tests
  (2026-07-07 baseline). New tests added when thresholds change.
- `GET /api/observatory/tool-signals` on a live daemon with a
  registry of ≥ 20 tools where most are dormant returns a
  `signals` array of at most 4–5 entries, containing at most one
  `variant=unused` and at most one `variant=win`.
- No signal card has a title that reads `"Dormant tool"` verbatim —
  the short name must be present.
- `Signal.action` serialises as an omittable field:
  ```
  Signal { action: None } → JSON without an "action" key
  Signal { action: Some("...") } → JSON with "action": "..."
  ```
- Persisted `sensei.tool_insights` rows still carry raw per-tool
  signals (the DB doesn't know about curation) so the per-tool
  detail pane keeps working.

Optional check:
```
curl -s http://localhost:7744/api/observatory/tool-signals \
  | jq '.signals | group_by(.variant) | map({variant: .[0].variant, n: length})'
# expected: [{variant: "warn", n: X}, {variant: "opportunity", n: Y}, {variant: "unused", n: 0..1}, {variant: "win", n: 0..1}]
```

## Wrong gate

- **N dormant cards, one per tool.** Curation regressed. Endpoint is
  returning `derive_signals` output raw.
- **`Signal.action` is a required field** in the JSON — should be
  `#[serde(skip_serializing_if = "Option::is_none")]`.
- **Summary card lists tool names but tool_name field is populated
  with one of them** — should be `""` so consumers can distinguish
  summary rows.
- **Threshold coupling.** The per-tool insights writer picks a
  different threshold than the endpoint. Both must reach for
  `SignalThresholds::default()`.
- **`short_name` mishandles a namespaced tool** (e.g. `plugin.foo.bar`).
  Not caught by current tests — worth adding one when we see it in
  the wild.
- **Curated list contains identical titles for two tools** ("workhorse
  tool"). Copy-key collision — should never happen if `{short}` is
  in the title.

## Related

- [[pipeline/capture]] — populates `sensei.tool_usage_stats`
- [[screen/observatory-instruments-health]] — the primary consumer
- [[pipeline/analyzer]] — schedules `AggregateToolInsights` per tick
