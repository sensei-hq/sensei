# 録 · Observatory · Instruments · Replay

**Segment:** 03 · Observatory — daily use
**Route:** `/instruments/replay`
**Source mockup:** [`lib/instruments-simple.jsx`](../../mockups/Sensei/lib/instruments-simple.jsx) → `InstrumentsReplaySimple` → delegates to `InstrumentsReplay` in `lib/instruments.jsx`
**App file:** `app/src/routes/(observatory)/instruments/+page.svelte` (Replay tab)

## Purpose

Replay is the answer to *"what did the assistant actually do?"* Not
a session log — a **tool-call replay**. Step through the tools the
assistant reached for, in order. Pure request + response, per call:
what was asked, what came back, how long it took.

The default flow: pick a session from the left rail → see a summary
strip up top → walk the timeline of tool calls on the left → see
the currently-focused call's request + response on the right.

This screen is the truthful record. When Insights says "the
assistant reached for `sensei.search` three times and ignored the
results", Replay is where the user goes to see it happen and decide
whether the diagnosis holds.

Kanji is 録 — *record*.

## Sub-nav placement

Replay follows the Instruments group's placement rule: sub-tab
strip renders **inside the screen**, between the hero and the
two-pane body, via `subNav` prop.

## Data invariants

- `GET /api/sessions?limit=N` returns the session list for the
  picker (see [[screen/observatory-sessions]] data shape).
- `GET /api/sessions/{id}/replay` returns the tool-call timeline:
  ```json
  {
    "session_id": "…",
    "title": "…",
    "project": "…",
    "totalTurns": N,
    "toolCallCount": N,
    "ftr": bool,
    "corrections": N,
    "duration_ms": N,
    "calls": [
      { "i": 1, "kind": "read|search|edit|call|…",
        "tool_name": "sensei.search",
        "ts_ms": N,
        "duration_ms": N,
        "request":  { … arguments passed to the tool … },
        "response": { … tool response OR error … },
        "verdict": "used|partial|ignored" }, …
    ]
  }
  ```
- **Session-id resolution.** The endpoint accepts either the
  observatory UUID or the client-session id (see
  [[pipeline/capture]] "session-id gotcha"). The bug already fixed
  once here: `activity.assistant_events.session_id` stores the
  client id; the handler must resolve. Any regression here shows
  as "no tool calls" against a session that clearly has some.
- `verdict` per call comes from
  `sensei.tool_call_verdicts.classify_verdict` (#90, shipped
  2026-07). Renders as a chip on the timeline row.
- Every call carries a `ts_ms` so the timeline can render in real
  chronology, not row order.

## Signals shown

### Session-picker rail

| Element | Value |
|---|---|
| Section header | `sessions` (uppercase, 11px) |
| Session row | `s-2891` mono + FTR chip (`ftr` green OR `{N}c` warn) + 2-line title + `{project} · {N} calls · {duration}` |
| Focus state | left border accent + paper background |
| Ordering | by `started_at` desc; ties by session id |

### Summary strip (right pane top)

| Element | Value |
|---|---|
| Session title | display 15px |
| Session id | mono, small |
| Turns stat | integer |
| Tool calls stat | integer |
| FTR stat | `yes` / `no` with success/warn tone |

### Timeline (left of detail split)

| Element | Value |
|---|---|
| Call row | `{i}` mono ordinal + kanji (from `kind`) + tool short-name + duration + verdict chip |
| Focus state | filled paper background + border |
| Ordering | by `i` (which is `ts_ms` order) |
| Verdict chip | `used` (success) / `partial` (mute) / `ignored` (warn) |

### Detail (right of timeline split)

| Element | Value |
|---|---|
| Request panel | pretty-printed JSON of `call.request`, copy button |
| Response panel | pretty-printed JSON of `call.response`, copy button, elapsed time chip |
| Error state | red-tinted panel when the call failed; error message + code |
| Empty state | "Select a call to see request + response." — only when timeline is populated |

## Done gate

- On Jerry's live data, picking a session in the rail populates
  the summary strip, the timeline, and the initial detail
  (call `i=1`).
- Every session that shows a `ftr` badge in the rail has a
  populated timeline in the detail pane. **No "no tool calls in
  this session" against a session that actually has tool calls.**
- Session-id resolution: both observatory-UUID and client-session
  id paths resolve to the same replay data.
- Each timeline row's verdict chip matches the daemon's
  `sensei.tool_call_verdicts` classification.
- Elapsed times on the response panel match `duration_ms` in the
  call row.
- The subNav renders between the hero and the two-pane body.
- No horizontal scrollbars introduced by the timeline / detail
  split at reasonable window widths.
- Dark mode: verdict chip colours stay readable.

Optional check:
```
# Does the replay endpoint resolve BOTH id shapes?
curl -s http://localhost:7744/api/sessions/$OBS_UUID/replay | jq '.calls | length'
curl -s http://localhost:7744/api/sessions/$CLIENT_SID/replay | jq '.calls | length'
# expected: both non-zero, both equal

# Are verdicts populated for calls in the last week?
psql -A -t -c "select verdict, count(*) from sensei.tool_call_verdicts
                where ts_ms > (extract(epoch from now()) - 7*86400)*1000
                group by verdict" -d sensei
# expected: rows for used / partial / ignored, not all NULL
```

## Wrong gate

- **"No tool calls in this session"** on a session whose rail chip
  says `ftr` or `{N}c`. Session-id resolution regressed — the fix
  from 2026-07-07 must survive. This is the exact bug we already
  paid to fix once.
- **Every call carries verdict `used`.** Classifier not populated
  or the endpoint isn't reading `tool_call_verdicts`.
- **Timeline row order doesn't match `ts_ms`.** Sort applied to
  `i` OR to insertion order rather than timestamp.
- **Response panel shows the raw HTTP body of the daemon**
  (`{data: {…}}`), not the tool's response payload. Unwrap layer
  missed.
- **Sensitive strings in the request/response** (API keys, PII).
  Secret-redaction from capture should have caught it before it
  reached the DB.
- **`Select a call to see request + response.`** shows even when
  a call is focused — empty-state gate wrong.
- **Session picker shows only 1 session** despite the sessions
  view showing many. Picker uses a different endpoint than
  `/api/sessions`.
- **subNav appears above the hero.** Placement rule violated.
- **Elapsed time renders in seconds when the daemon returns
  milliseconds.** Unit mismatch on the display.

## Related

- [[pipeline/capture]] — session-id gotcha & hook event stream
- [[pipeline/ftr]] — the FTR badge on session rows
- [[pipeline/signals]] — verdicts feed into signal derivation
- [[screen/observatory-sessions]] — session-list peer
- [[screen/observatory-instruments-playground]] — sibling tab
- [[screen/observatory-instruments-health]] — sibling tab
