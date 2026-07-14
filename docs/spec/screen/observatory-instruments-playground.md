# 具 · Observatory · Instruments · Playground

**Segment:** 03 · Observatory — daily use
**Route:** `/instruments`
**Source mockup:** [`lib/observatory/instruments-simple.jsx`](../../mockups/Sensei/lib/observatory/instruments-simple.jsx) → `InstrumentsPlaygroundSimple`
**App file:** `app/src/routes/(observatory)/instruments/+page.svelte` (Playground tab)

## Purpose

Playground is a **room of tools**. The user came here to *see what
each tool does, what it returns, and to try one before trusting the
assistant to reach for it*. Two panes:

- **Left rail** — MCP tree. Every connected MCP, with its
  registered tools underneath, grouped and searchable.
- **Right pane** — tool detail + argument form + execute button +
  response viewer. Try the tool without leaving the app.

The default state is empty — an invitation. The user picks a tool
from the rail, edits argument defaults, hits Execute, sees the
response inline. This is also where **argument defaults come from
the tool's own MCP declaration** — the daemon must offer sensible
starting values, not `""` for every field. (This was a reported UX
bug: `repoId` field starting empty made every default query return
empty results.)

Kanji is 具 — *instrument*.

## Sub-nav placement

Playground follows the Instruments group's placement rule: sub-tab
strip is rendered **inside the screen**, between the hero and the
two-pane body, via the `subNav` JSX prop. See
[[MOCKUP-INDEX]] §"Sub-nav placement".

## Data invariants

- `GET /api/observatory/mcp-servers` returns the connected MCPs
  (same shape as [[screen/observatory-instruments-health]]).
- `GET /api/mcp/tools?mcp={id}` returns tool declarations:
  ```json
  {
    "tools": [
      { "name": "sensei.search", "description": "…",
        "input_schema": { … JSON schema … },
        "defaults": { "query": "sensei.projects", "limit": 10 },
        "category": "codebase" | "session" | "memory" | "library" | "governance" | …,
        "activity": { "calls_14d": N, "last_used_at": iso? } }, …
    ]
  }
  ```
- `POST /api/mcp/call` — the execute endpoint. Body:
  `{ mcp: string, tool: string, args: object }`. Returns the tool's
  raw response, or `{ error, code }`.
- **Argument defaults come from the tool declaration**, not the UI.
  The daemon computes them: for a `repoId` field, it defaults to
  the active project or an example project id (whichever is likely
  useful), not empty. See (memory: feedback_no_command_guessing) — the
  UI can't guess defaults; the tool must declare them.

## Signals shown

### Hero

`InstrHero`: kanji · eyebrow · title · subtitle.

- kanji: (tool-specific in mockup; keep small on real UI)
- title: `"Try any tool before you trust it."`
- subtitle: `"A room of tools. See what each one does, what it returns. Try one."`

Below the hero: the `subNav` strip (`Playground · Replay · Health`).

### Left rail

| Element | Value |
|---|---|
| Search input | filters tool names + categories |
| MCP group header | MCP kanji + name + activity chip (`{tools_invoked_14d} of {tools_registered}`) |
| Group toggle | expand/collapse per MCP |
| Tool row (in group) | `sensei.search` short name + activity dot (green if used-in-14d, muted if not) |
| Focused tool row | left border accent + muted background |

### Right pane — detail + form + response

| Element | Value |
|---|---|
| Tool title | full tool name (mono) + short name (display) |
| Description | tool declaration description |
| Category chip | small `category` label |
| Argument form | one row per property in `input_schema.properties`. Type-appropriate widget: text, number, boolean toggle, JSON textarea for objects/arrays |
| Argument default | pre-filled from `tool.defaults` — never empty when a default exists |
| Execute button | `data-testid="tool-execute"` — mockup convention |
| Response viewer | pretty-printed JSON with copy button, elapsed time chip |
| Error viewer | red-tinted panel with `error.message` + `error.code` when the call fails |

## Done gate

- On Jerry's live data, opening `/instruments` lands in Playground.
- Left rail lists every connected MCP with tool counts matching
  the daemon.
- Picking a tool populates the argument form with defaults from the
  tool declaration — no empty `repoId` on a `search` call.
- Execute round-trip works against real MCPs (sensei is
  a guaranteed presence; postgres/stripe/etc. per install).
- The subNav renders between the hero and the two-pane body, not
  above the hero.
- Elapsed time chip on the response viewer matches the
  `duration_ms` the daemon reports.
- Failed calls show the daemon's error, not a swallowed
  "something went wrong."

Optional check:
```
# Are default args populated for sensei.search?
curl -s 'http://localhost:7744/api/mcp/tools?mcp=sensei' \
  | jq '.tools[] | select(.name=="sensei.search") | .defaults'
# expected: at least { query: "…", limit: N } — never {}
```

## Wrong gate

- **Argument form defaults are all empty.** Tool declaration
  defaults not being surfaced OR the daemon isn't computing them.
- **Execute button clicks but nothing happens.** POST target
  wrong, or the response isn't wired into the viewer.
- **Response viewer shows `{}` for a tool that should return
  data.** `repoId` default was empty (the reported bug); or the
  path variable is being passed with a name the daemon expects as
  a UUID.
- **subNav appears above the hero.** Placement rule violated.
- **Activity dot green on a tool that hasn't been called in 30
  days.** Time window filter mis-applied.
- **MCP tree collapses fully on tool selection.** Group open state
  isn't persisted per session.
- **Third-party MCP tool call returns "MCP not connected" despite
  the L1 Health tab showing it connected.** Two sources of
  connection truth diverged.
- **Executing a tool call is logged neither in
  `activity.tool_usage_stats` nor in the Replay session for the
  developer's own actions.** Playground calls should be
  attributable to the user, not confused with an assistant's.

## Related

- [[pipeline/mcp-surface]] — declaration + defaults contract
- [[pipeline/capture]] — activity attribution
- [[screen/observatory-instruments-health]] — L1 MCP grid + L2 signals
- [[screen/observatory-instruments-replay]] — sibling tab
