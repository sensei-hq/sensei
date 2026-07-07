# 具 · Observatory · Instruments · Health

**Segment:** 03 · Observatory — daily use
**Route:** `/instruments/health`
**Source mockup:** [`lib/instruments-simple.jsx`](../../mockups/Sensei/lib/instruments-simple.jsx) → `InstrumentsHealthSimple` → delegates to `InstrumentsHealth` in `lib/instruments.jsx`
**Data:** `lib/mcp-signals-data.js` → `window.MCP_SIGNALS.mcpMeta` + `toolUsage` + `thirdPartyUsage`
**App file:** `app/src/routes/(observatory)/instruments/+page.svelte` (Health tab)
**Daemon files:** `crates/senseid/src/api/handlers/tool_signals.rs`, `crates/senseid/src/api/handlers/observatory.rs::tool_signals`
**Refactored:** 2026-07-07 (per-MCP entry + per-tool L2 drill; signal curation moved to L2)

## Purpose

The user came here worried about tools — MCP surface, sensei
tool-picking, third-party assistants. Health is the **at-a-glance
verdict**, but at two zoom levels:

**L1 — MCP grid.** One card per connected MCP server (Sensei,
Postgres, Stripe, GitHub, Sentry). Each card shows *what share of
this MCP's registered tools are actually being invoked*, whether
it's connected at all, and a one-line note. This is the "which
MCPs are earning their keep?" pane. It's the entry.

**L2 — per-tool.** Click a card. The signal strip (curated
warns/opportunities/dormant-summary/win-summary) and the per-tool
usage table appear scoped to that MCP. This is where the signal
curation we shipped on 2026-07-07 lives — visible when a user cares
about one MCP's per-tool detail, not when they're glancing at the
overall surface.

Kanji is 具 — *instrument*.

## Sub-nav placement

Instruments follows a placement rule that differs from other groups:
the sub-tab strip (`Playground / Replay / Health`) renders **inside
each screen**, between the screen's hero and its body — not at the
top of the observatory main container. The parent passes `subNav`
as a JSX prop; this screen renders it under its own `InstrHero`.
See `docs/llm-spec/MOCKUP-INDEX.md` §"Sub-nav placement".

## Data invariants

### MCP-level (L1)

- `GET /api/observatory/mcp-servers` returns:
  ```json
  {
    "mcps": [
      { "id": "sensei",   "name": "Sensei", "kanji": "先",
        "publisher": "local", "connected": true, "note": "Your codebase's private expert.",
        "tools_registered": 44, "tools_invoked_14d": 16, "share_invoked": 0.36 },
      { "id": "postgres", … }, …
    ]
  }
  ```
- `tools_registered` = distinct tools this MCP declares.
- `tools_invoked_14d` = distinct tools with `call_count > 0` in the
  last 14 days.
- `share_invoked = tools_invoked_14d / tools_registered`, or `0`
  when registered is 0. Rendered as a share bar.
- `connected: false` mcps still render as cards with a "not
  connected — recommended for this stack" note; they never render
  with active signals.
- **Sensei itself is an MCP.** It is the first card. It is
  connected.

### Per-tool (L2, scoped to a selected MCP)

- `GET /api/observatory/tool-signals?mcp={id}` returns the curated
  signal list scoped to that MCP (see [[pipeline/signals]]). If
  omitted, the endpoint returns all-MCPs curated.
- `GET /api/observatory/tool-insights?mcp={id}` returns the flat
  per-tool table (used by the drill pane).
- Signals: at most one dormant summary, at most one win summary,
  all warns/opportunities per tool.

## Signals shown

### L1 — MCP grid

| Element | Value | Meaning |
|---|---|---|
| MCP card kanji | `p.kanji` (large) | Domain glyph — Sensei is 先, Postgres 庫, Stripe 銀, GitHub 貢, Sentry 哨 |
| MCP name + publisher | text + small mono | Publisher is scope tag |
| Connected chip | on / off | Muted card when not connected |
| Share-invoked bar | 0..1 fraction | `tools_invoked_14d / tools_registered` |
| Bar detail | `{invoked} of {registered} tools invoked · 14d` | Under the bar |
| Note line | `p.note` | Editorial one-liner from `mcpMeta` (frozen; not model-generated at this layer) |
| Card click | opens L2 for that MCP | Passes `mcp={id}` to the sub-view |

### L2 — per-tool (scoped)

Insights strip:

| Variant | Kanji | When it fires | Copy owner |
|---|---|---|---|
| `warn` | 警 | `calls >= 50 && error_rate >= 0.05` | [[pipeline/insight-copy]] (fallback: `{short}: {rate}% failure rate`) |
| `opportunity` | 芽 | `calls >= 10 && error_rate >= 0.05` (not high-traffic) | insight-copy (fallback: `{short}: room to improve`) |
| `unused` | 眠 | `calls == 0` OR `days_since_last_use >= 14`; collapsed to one summary when >1 | insight-copy (fallback: `{n} tools dormant`) |
| `win` | 勝 | `calls >= 50 && error_rate <= 0.02`; collapsed to one summary when >1 | insight-copy (fallback: `{n} workhorse tools`) |

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

- L1 renders one card per MCP in `mcpMeta` on Jerry's live data.
  Sensei is first; connected MCPs precede disconnected ones.
- The share bar on the Sensei card matches
  `tools_invoked_14d / tools_registered` from the daemon (spot-
  checked against the live tool_usage_stats).
- Clicking any MCP card scopes the L2 view. Deep-linking with
  `?mcp={id}` opens L2 directly.
- L2 Insights strip renders **at most one dormant summary** and
  **at most one win summary**; per-MCP totals never exceed a
  handful of visible cards.
- Every card's title + detail comes through insight-copy when the
  model is available; fallback strings are labelled fallback in
  the wire response for debuggability.
- Dark-mode: SignalCard text stays readable on all four tinted
  backgrounds. Disconnected-MCP cards remain distinguishable.
- Instruments sub-nav renders under the hero (via `subNav` prop),
  never at the top of the observatory main container.

Optional check:
```
curl -s http://localhost:7744/api/observatory/mcp-servers \
  | jq '.mcps | map({id, share: (.share_invoked * 100 | floor)})'

curl -s "http://localhost:7744/api/observatory/tool-signals?mcp=sensei" \
  | jq '.signals | group_by(.variant) | map({variant: .[0].variant, n: length})'
# expected: at most one unused row, at most one win row
```

## Wrong gate

- **L1 shows only Sensei.** Other MCPs from `mcpMeta` aren't being
  populated OR the daemon lacks the mcp_servers table content.
- **Share bar reads 100% on every MCP.** `tools_invoked_14d` is
  being read as `tools_registered`.
- **Disconnected MCP still lists signals.** Signal derivation is
  running on a non-connected surface.
- **L2 shows the same signals regardless of the selected MCP.**
  `?mcp=` isn't filtering server-side; the endpoint returns the
  global curated list.
- **40+ signal cards in L2.** Curation regressed (see
  [[pipeline/signals]] wrong-gate).
- **Instruments sub-nav appears twice** (top-of-main AND
  below-hero). The observatory shell didn't gate on
  `groupKeyOf(section) === "instruments"` correctly.
- **L1 card kanji is the wrong glyph.** `mcpMeta` unified with
  another datasource; use only `mcpMeta` for identity here.
- **Note lines say "coming soon" on connected MCPs.** Editorial
  note has decayed; the wire response should match the current
  state.

## Related

- [[pipeline/signals]] — curation logic scoped by MCP
- [[pipeline/insight-copy]] — mentor-voice text for signal cards
- [[pipeline/capture]] — where `tool_usage_stats` gets its data
- [[pipeline/mcp-surface]] — the connected-MCP list + registration
- [[screen/observatory-instruments-playground]] — sibling tab
- [[screen/observatory-instruments-replay]] — sibling tab
