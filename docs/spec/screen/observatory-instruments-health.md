# 健 · Observatory · Instruments · Health

**Segment:** 03 · Observatory — daily use
**Route:** `/instruments/health`
**Source mockup:** [`lib/observatory/instruments-simple.jsx`](../../mockups/Sensei/lib/observatory/instruments-simple.jsx) → `InstrumentsHealthSimple` → delegates to `InstrumentsHealth` in `lib/observatory/instruments.jsx`
**Data:** `GET /api/instruments/tools-health` (L1 grid) · `GET /api/observatory/tool-signals` + `tool-insights` (L2 per-tool)
**App file:** `app/src/routes/(observatory)/instruments/+page.svelte` (Health tab)
**Daemon files:** `crates/senseid/src/api/handlers/tool_signals.rs`, `crates/senseid/src/api/handlers/observatory.rs::tool_signals`
**Refactored:** 2026-07-07 (per-MCP entry + per-tool L2 drill; signal curation moved to L2)

## Purpose

The user came here worried about tools — MCP surface, sensei
tool-picking, third-party assistants. Health is the **at-a-glance
verdict**, but at two zoom levels:

**L1 — source grid.** One card per registered source (a builtin
assistant tool set, or a probed MCP server). Each card shows
*what share of this source's registered tools are actually being
invoked* in the last 14 days, the total call count, and whether
the source is connected. When a server cannot be probed,
`tools_registered` is null and the card shows
`invoked N · registered —` with no share bar. This is the
"which sources are earning their keep?" pane.

**L2 — per-tool.** Click a card. The signal strip (curated
warns/opportunities/dormant-summary/win-summary) and the per-tool
usage table appear scoped to that MCP. This is where the signal
curation we shipped on 2026-07-07 lives — visible when a user cares
about one MCP's per-tool detail, not when they're glancing at the
overall surface.

Kanji is 健 — *health*.

## Sub-nav placement

Instruments follows a placement rule that differs from other groups:
the sub-tab strip (`Playground / Replay / Health`) renders **inside
each screen**, between the screen's hero and its body — not at the
top of the observatory main container. The parent passes `subNav`
as a JSX prop; this screen renders it under its own `InstrHero`.
See `docs/spec/MOCKUP-INDEX.md` §"Sub-nav placement".

## Data invariants

### Tool inventory and discovery (L1 source)

- **`sensei.assistant_tools`** is the unified tool registry — one row per
  registered tool: `assistant_family, source_type ('mcp'|'builtin'),
  source_key, tool_name, invoked_name` (joins
  `sensei.tool_usage_stats.tool_name`), `server_id → mcp_servers`.
  Config (command/args/env/connection_state) stays on the typed source
  table `sensei.mcp_servers`.
- Discovery is daemon-owned: runs at startup and on
  `POST /api/instruments/tools/refresh`. For Claude Code, the
  `ToolDiscovery` trait scans `~/.claude/mcp.json`, project `.mcp.json`,
  `~/.claude.json`, and `~/.claude/plugins/**/.mcp.json`, then probes
  each server to enumerate its tools.

### L1 grid endpoint

`GET /api/instruments/tools-health` → `{ sources: [ … ] }`. One card per
source (one row per `source_type + source_key` pair):

```json
{
  "sources": [
    {
      "assistant_family": "claude_code",
      "source_type": "builtin",
      "source_key": "claude_code",
      "name": "Claude Code built-ins",
      "connected": true,
      "connection_state": "connected",
      "server_id": null,
      "tools_registered": 21,
      "tools_invoked_14d": 14,
      "calls_14d": 3847,
      "share_invoked": 0.67
    },
    {
      "assistant_family": "claude_code",
      "source_type": "mcp",
      "source_key": "sensei",
      "name": "sensei",
      "connected": true,
      "connection_state": "connected",
      "server_id": "…uuid…",
      "tools_registered": 33,
      "tools_invoked_14d": 6,
      "calls_14d": 412,
      "share_invoked": 0.18
    }
  ]
}
```

Field notes:

- `connection_state` is the raw string enum from `sensei.mcp_servers`
  (`unknown | connected | error | disabled`); `connected` (bool) is
  the normalised form — both are present in the wire so the UI can
  use either.
- `tools_registered` (int | null). **Null when the server was never
  probed** — e.g., a server whose process could not be started.
  The card must then show `invoked N · registered —` and must **not**
  render a share bar. `share_invoked` is also null in this case.
- `share_invoked = tools_invoked_14d / tools_registered` for probed
  sources; null for un-probeable servers.
- `tools_invoked_14d` = distinct tools with at least one call in the
  last 14 days (via `invoked_name → tool_usage_stats`).
- `calls_14d` = total call events across all tools on this source in
  the last 14 days.
- **Worked example:** sensei MCP — 33 registered · 6 invoked ·
  412 calls · 18% share.

### Per-tool (L2)

- `GET /api/observatory/tool-signals` returns the curated featured
  signal list (see [[pipeline/signals]]). The endpoint is param-free
  today and returns all sources combined; scoping to one source is a
  follow-up.
- `GET /api/observatory/tool-insights` returns the flat per-tool
  table used by the drill pane. Also param-free today.
- Clicking an L1 card drills to L2 for that source.
- Signals: at most one dormant summary, at most one win summary,
  all warns/opportunities per tool.

## Signals shown

### L1 — source grid

Header KPIs (derived from the same `sources` grid rows):

| KPI | Kanji | Value |
|---|---|---|
| Servers connected | 接 | `count(connected=true)` of total sources |
| Tool coverage | 具 | `sum(tools_invoked_14d) / sum(tools_registered)` across probed sources only |
| Total tool calls | 計 | `sum(calls_14d)` across all sources |
| First-try rate | 一 | from `tool_usage_stats` verdict split for the same window |

Per-source card:

| Element | Value | Meaning |
|---|---|---|
| Source name | `s.name` | e.g. `sensei`, `Claude Code built-ins` |
| Connected chip | on / off | From `s.connected` (normalised bool); muted card when off |
| Share-invoked bar | 0..1 fraction | `s.share_invoked`; **omitted entirely** when `tools_registered` is null |
| Bar detail | `invoked {invoked_14d} · registered {registered}` | Shows `registered —` when `tools_registered` is null (honest degrade) |
| Calls chip | `{calls_14d} calls · 14d` | Total call events across all tools on this source |
| Card click | opens L2 | Drills to per-tool view for this source |

### L2 — per-tool (scoped)

Insights strip:

| Variant | Kanji | When it fires | Copy owner |
|---|---|---|---|
| `warn` | 警 | `calls >= 50 && error_rate >= 0.05` | [[pipeline/narration-cache]] (fallback: `{short}: {rate}% failure rate`) |
| `opportunity` | 芽 | `calls >= 10 && error_rate >= 0.05` (not high-traffic) | narration-cache (fallback: `{short}: room to improve`) |
| `unused` | 眠 | `calls == 0` OR `days_since_last_use >= 30` (a month — 14 flagged weekly tools as noise, #98); collapsed to one summary when >1 | narration-cache (fallback: `{n} tools dormant`) |
| `win` | 勝 | `calls >= 50 && error_rate <= 0.02`; collapsed to one summary when >1 | narration-cache (fallback: `{n} workhorse tools`) |

Per-tool table (drill):

| Column | Value |
|---|---|
| Tool | shortname (strips `sensei.` or `mcp__…__`) |
| Calls | integer |
| Errors | integer |
| Avg ms | integer |
| Last used | relative time |
| Verdict chip | `healthy` / `ok` / `warn` / `unused` (matches the mockup vocabulary) |

## Done gate

- L1 renders one card per source row from `GET /api/instruments/tools-health`
  on Jerry's live data. Connected sources precede disconnected ones.
- The `builtin` card shows `tools_registered` ≈ 21 and a real
  `share_invoked` (not null, not fabricated).
- The sensei MCP card shows real `tools_registered` and a real
  `share_invoked`; the worked example is 33 registered · 6 invoked ·
  18% share.
- Any source whose server could not be probed shows
  `registered —` and no share bar (not a zero bar, not a fabricated
  number).
- Clicking any source card opens L2. The drill pane makes clear
  that signals are currently all-source (label: `all sources`);
  it does not fake per-source scoping.
- L2 Insights strip renders **at most one dormant summary** and
  **at most one win summary**; per-source totals never exceed a
  handful of visible cards.
- Every card's title + detail comes through narration-cache when the
  model is available; fallback strings are labelled fallback in
  the wire response for debuggability.
- Dark-mode: SignalCard text stays readable on all four tinted
  backgrounds. Disconnected-source cards remain distinguishable.
- Instruments sub-nav renders under the hero (via `subNav` prop),
  never at the top of the observatory main container.

Optional check:
```
curl -s http://localhost:7744/api/instruments/tools-health \
  | jq '.sources | map({source_key, source_type, tools_registered, share: (.share_invoked // "null")})'
# expected: builtin row tools_registered≈21 with real share;
#           sensei row tools_registered≈33 with real share;
#           any un-probed server shows share: "null"

curl -s http://localhost:7744/api/observatory/tool-signals \
  | jq '.signals | group_by(.variant) | map({variant: .[0].variant, n: length})'
# expected: at most one unused row, at most one win row
```

## Wrong gate

- **L1 shows only the sensei MCP card.** Discovery hasn't captured
  builtin tools or other MCP servers. Check that `sensei.assistant_tools`
  has rows for `source_type = 'builtin'` and every installed MCP.
- **Share bar reads 100% on every source.** `tools_invoked_14d` is
  being read as `tools_registered`.
- **A card shows a share bar while its server was never probed.**
  When `tools_registered` is null, `share_invoked` is also null —
  the card must show `registered —` and must not render a share bar.
  A zero bar or a fabricated percentage is equally wrong.
- **Duplicate source cards for the same server.** `source_type +
  source_key` normalization didn't deduplicate; a known follow-up
  when discovery runs more than once before reconcile.
- **Disconnected source still shows signals.** Signal derivation is
  running on a non-connected surface.
- **L2 shows the same signals regardless of which card was clicked.**
  The drill pane must not silently pass global all-source signals as
  if they were scoped. Until per-source scoping is built, label the
  pane clearly (`all sources · scoping per source coming`).
- **40+ signal cards in L2.** Curation regressed (see
  [[pipeline/signals]] wrong-gate).
- **Instruments sub-nav appears twice** (top-of-main AND
  below-hero). The observatory shell didn't gate on
  `groupKeyOf(section) === "instruments"` correctly.

## Related

- [[pipeline/signals]] — curation logic scoped by MCP
- [[pipeline/narration-cache]] — mentor-voice text for signal cards
- [[pipeline/capture]] — where `tool_usage_stats` gets its data
- [[pipeline/mcp-surface]] — the connected-MCP list + registration
- [[screen/observatory-instruments-playground]] — sibling tab
- [[screen/observatory-instruments-replay]] — sibling tab
