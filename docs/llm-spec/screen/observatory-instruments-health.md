# 具 · Observatory · Instruments · Health

**Segment:** 03 · Observatory — daily use
**Route:** `/instruments/health`
**Source mockup:** [`lib/instruments-simple.jsx`](../../mockups/Sensei/lib/instruments-simple.jsx) → `InstrumentsHealthSimple`
**Rebuild landed (signal derivation):** 2026-07-07 (this session)
**App file:** `app/src/routes/(observatory)/instruments/+page.svelte` (Health tab)
**Daemon files:** `crates/senseid/src/api/handlers/tool_signals.rs`, `crates/senseid/src/api/handlers/observatory.rs::tool_signals`

## Purpose

The user opened Instruments because something is nagging them about
their tools — either their MCP surface, the built-in sensei tools, or
the assistant's tool-picking behaviour. Health is the **at-a-glance
verdict**: for each tool the daemon has ever seen, is it a win, warn,
opportunity, or dormant? And more importantly: **what should I do about
it?** Kanji is 具 — *instrument*.

The wrong version of this screen is a long list of "no calls in N days"
cards, one per registered tool. That's noise. The right version is a
**selective, actionable list** — every warn and opportunity kept as a
per-tool card because they need action, but all the dormants collapsed
to one summary card ("40 tools dormant") with the first three names and
an "N more" tail.

## Data invariants

- `sensei.tool_usage_stats` has rows for every tool ever called
  (populated by the capture pipeline).
- `sensei.tool_insights` gets a row per tool per aggregation tick
  (written by the `AggregateToolInsights` task, one snapshot per tool).
- **The per-tool cache and the featured-insights list are different
  concerns.** The cache feeds the per-tool detail pane (below the
  Insights strip); the featured list is derived on the fly and curated.
- FTR verdict split (`used / partial / ignored`) comes from
  `get_verdict_split_per_tool(14)` — folded into each `tool_insights`
  row's `metrics`. Zero-verdict tools show 0/0/0, not absent.

## Signals shown

The Insights strip at the top:

| Variant | Kanji | When it fires | Copy pattern |
|---|---|---|---|
| `warn` | 警 | `calls >= 50 && error_rate >= 0.05` | Per-tool. Title: `{short}: {rate}% failure rate`. Detail: `{calls} calls, {errors} errored. High-traffic tool with sharp edges — fix these first.` Action: `Edit tool: {tool}`. |
| `opportunity` | 芽 | `calls >= 10 && error_rate >= 0.05` (but not high-traffic) | Per-tool. Title: `{short}: room to improve`. Detail: `{calls} calls, {rate}% failure. Modest volume — small polish would pay off.` Action: `Edit tool: {tool}`. |
| `unused` | 眠 | `calls == 0` OR `days_since_last_use >= 14`. **Collapsed to one summary when >1.** | Summary title: `{n} tools dormant`. Detail: `{tool1}, {tool2}, {tool3} and {N-3} more haven't been called in the last two weeks. Either wire them into a skill or persona, or archive them.` Action: `Review tool registry`. |
| `win` | 勝 | `calls >= 50 && error_rate <= 0.02`. **Collapsed to one summary when >1.** | Summary title: `{n} workhorse tools`. Detail: `{tool1}, {tool2}, {tool3} are running high-volume with clean error rates.` Action: none. |

**Sort order:** warn → opportunity → unused → win. Actionable first.

The per-tool table below the Insights strip:

| Column | Value | Meaning |
|---|---|---|
| Tool | shortname | Strips `sensei.` or `mcp__…__` prefix |
| Calls | integer | Total in `tool_usage_stats` |
| Errors | integer | Total errors captured |
| Avg ms | integer | Mean duration |
| Last used | relative time | Powers dormancy determination |

## Done gate

- On Jerry's live data (~40 registered sensei tools of which most are
  dormant), the Insights strip renders **at most one dormant card**,
  **at most one win card**, and one card per warn/opportunity — total
  usually 2–4 cards, not 40.
- Every visible card has:
  - the tool's short name in the title (not the raw MCP path)
  - a concrete number in the detail (calls / failure rate / dormancy days)
  - an action hint on warn/opportunity/unused variants
- Dark-mode: the SignalCard text stays readable on all four tinted
  backgrounds. No white-on-cream, no light-green-on-light-green.
- The dormant *count* in the header agrees with the length of the
  dormant list in the per-tool table below.
- Clicking a per-tool row expands it (existing behaviour) and — when
  present — shows the same signal detail we surfaced in the Insights
  strip.

Optional check:
```
curl -s http://localhost:7744/api/observatory/tool-signals | jq '.signals | group_by(.variant) | map({variant: .[0].variant, n: length})'
# expected: at most one row with variant=unused and at most one row with variant=win
```

## Wrong gate

- **40+ signal cards.** Curation regressed — the endpoint is returning
  the raw per-tool list.
- **All dormant cards say "No calls in N days" verbatim.** The
  short-name / kanji / action-hint refactor was reverted.
- **Header dormant count = 0 while the table shows dormant tools.**
  Header derivation is reading the wrong source. Both should derive
  from the same source of truth.
- **Signal action hint missing.** Card renders but there's no next-step
  hint — the "what do I do about it" is what makes this screen useful.
- **"Workhorse tool" title without a tool name.** The generic copy
  ("workhorse tool" appearing multiple times identically) is the
  smoke of a caching-key bug.
- **Every card is `variant=warn`.** Threshold tuning bug — likely the
  error-rate denominator collapsed.

## Related

- [[pipeline/signals]] — the derivation itself, with unit tests
- [[pipeline/capture]] — where `tool_usage_stats` gets its data
- [[screen/observatory-instruments-playground]] — sibling tab
- [[screen/observatory-instruments-replay]] — sibling tab
