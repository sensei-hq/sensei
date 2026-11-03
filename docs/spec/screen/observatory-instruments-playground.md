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

- `GET /api/instruments/mcp-servers` returns every MCP server discovered in
  an assistant's config (same rows as
  [[screen/observatory-instruments-health]]). Sensei's own server is one of
  them — the rail lists it first and opens it.
- `GET /api/instruments/mcp-servers/{id}/tools` returns the tools **as the
  server itself lists them** (`tools/list`, probed over stdio and cached):
  `name`, `description`, `inputSchema`. A probe that failed carries its
  `error`, shown on the group.
- `POST /api/instruments/mcp-servers/{id}/call` runs one tool **through the
  server**, over stdio `tools/call`, exactly as an assistant would. Body:
  `{ tool, arguments }`. Answers `{ result, isError, duration_ms }`; a tool's
  own failure is `isError: true` (its message is the result), and a server
  that cannot be started or answered is a 502 naming why.
- **The form is the tool's `inputSchema`.** One field per property, typed
  (string, number, boolean, enum, JSON); a declared `default` prefills it,
  and an otherwise-open `repoId` / `workspaceId` takes the active workspace.
  Values are sent in their declared types; an empty optional value is left
  out so the tool's own default applies.

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
| Argument form | one row per property in `inputSchema.properties`. Type-appropriate widget: text, number, boolean, enum select, JSON textarea for objects/arrays |
| Argument default | pre-filled from the schema's `default`; a repo/workspace id from the active workspace |
| Execute button | `data-testid="tool-execute"` — mockup convention |
| Response viewer | the result's text parts (pretty-printed when JSON), elapsed time chip from `duration_ms` |
| Error viewer | the tool's own `isError`, or the daemon's reason the server could not answer |

## Done gate

- On Dev's live data, opening `/instruments` lands in Playground.
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
# sensei's server lists its tools, with schemas, as an assistant sees them
id=$(curl -s localhost:7744/api/instruments/mcp-servers | jq -r '.servers[] | select(.mcp_key|test("(^|_)sensei$")) | .id')
curl -s "localhost:7744/api/instruments/mcp-servers/$id/tools" | jq '.tools | length'
curl -s -X POST "localhost:7744/api/instruments/mcp-servers/$id/call" \
  -H 'content-type: application/json' -d '{"tool":"list_workspaces","arguments":{}}' | jq '.isError, .duration_ms'
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
