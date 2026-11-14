# Setup & Preferences · M2

**Plane:** foundation · **Status:** [roadmap/01-foundation.md](../../../../roadmap/01-foundation.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)), team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Everything sensei can be told — and nothing more than first-run needs — lives in a
free-navigation Preferences surface, reachable at any time, never gated. Getting
started stays minimal: folders, workspaces, assistants, libraries, instruments,
inference, assignments, profile are panes, not steps. The user searches or browses,
edits, and the change applies live — no Save/Cancel dance, no restart. The one-and-only
first-run gate is *point sensei at your work*: add a scan root and press Begin, watching
workspaces appear live.

## In scope / out of scope

- In: the 8 Preferences panes (Folders, Workspaces, Assistants, Libraries, Instruments,
  Inference, Assignments, Profile); pane deep-links (`/preferences/folders`); global
  search across every configurable value; review cues that surface tuning suggestions;
  restore-default on overridden values; the first-run scan screen (single gate: add root + Begin);
  the kanji ⇄ solar icon-style preference and the wider personal-preference surface
  (decision 10, `prd/03-personas.md` §Preference surface).
- Out: a mandatory setup flow (the panes are free navigation, reachable at any time);
  dojo pairing/auth (T1 — the Connection sub-section wires to it, architecture here);
  capture, scanning, and watcher implementation (M3) — Folders pane *configures* roots.

## Surfaces

- **App** — `app/src/routes/(preferences)/settings/` (the pane; its panels live beside
  it); the setup wizard's `(config)/setup/scan` stage.
- **API** — `GET/PUT /api/config` (a flat key→value map), `GET/DELETE
  /api/config/{key}` for one key (writes go through `PUT /api/config`),
  `GET/PUT /api/preferences/collective`,
  `POST /api/scan/roots` (persist + start scanning immediately), `GET /api/assistants/health`.
  The designed `GET/PUT /api/preferences` (one object per pane) and
  `GET /api/preferences/cues` were **never built** — there is no `/api/preferences`
  route, only the `collective` and `commands` children.
- **Daemon** — assisted preferences bindings (`crates/senseid` collective_preferences
  surface for team defaults).

## Data invariants

- `GET /api/config` returns a flat key→value map; one key is read or deleted
  at `/api/config/{key}`, and writes go through `PUT /api/config`. The eight-section wire shape
  (`[assistants, assignments, folders, inference, instruments, libraries, profile,
  projects]`) was designed and dropped, so no client may expect it.
- Settings apply live: editing a value writes immediately; no value requires a restart to
  take effect.
- Root entry is validated (`must exist`, `must be a directory`, `must be readable`) before it
  can be kept; `sensei.scan_roots` persists each root and scanning starts immediately, not at
  "Begin".
- Begin is disabled until at least one root is `scanning`/`watching`.
- Every configurable setting is reachable from at least one pane (no hidden settings).
- Restore-default resets an overridden value to its shipped default — and only that value.
- Review cues deep-link to the specific row they're about, not just the pane.
- Profile user name auto-derives from the home folder on first boot, fills only a blank field,
  and is never overwritten on upgrade (observed regression 2026-07-07 — covered by a check).

## Signals shown

| Element | Value | Meaning | Decision it drives / next step |
|---|---|---|---|
| Panes (8) | Folders · Workspaces · Assistants · Libraries · Instruments · Inference · Assignments · Profile | ordered by frequency of visit | open the pane that holds the setting |
| Search field | text filter across every pane's values + labels + descriptions | type-ahead narrows live | jump to the setting that matches |
| Review cues | "your top assistant sends 40% of sessions to Claude Code but you haven't picked a default persona" | a nudge with a direct link | follow the link and fix the gap the cue names (pick the default persona) |
| Canvas root list | roots + branch policy (`active-only` / `pinned` / `all`) | where code comes from | add or remove a root, or change its branch policy |
| Inference pane | chains, installed models, hardware tier chip, cost running total | gateway tuning per chain (M5) | change a chain's model, or move a chain to a local model when the cost total climbs |
| First-run banner | "Point sensei at your work." + live-arriving workspaces | one gate, one forward action | add a scan root and press Begin |

## Done gate

- A first-run user adds a root and reaches "Begin" in one action; workspaces appear live as the
  scan discovers them.
- Every configurable value is reachable by search from `/preferences` — nothing is
  stranded.
- Editing any value applies live and survives restart; an override shows a one-click
  restore-default.
- The silhouette toggle (kanji ⇄ solar) and the personal-preference surface from decision 10
  render and persist.
- Team defaults merge sanely: assisted preferences round-trip and reject invalid destinations.

```bash
curl -s http://localhost:7744/api/config | jq '. | keys'   # the keys the settings panes write
curl -s http://localhost:7744/api/assistants/health | jq 'length' # one row per detected assistant
```

## Wrong gate

- **A configurable setting isn't reachable from any pane** — the pane coverage is incomplete.
- **Search misses a value that clearly matches** — hay-stack must index value *and* label *and* description.
- **Editing requires a Save/Cancel modal** — direct manipulation regressed.
- **Review cue links to the pane but not the specific row** — must scroll + highlight inside the pane.
- **Restore-default doesn't reset, or resets the wrong default.**
- **Profile user name blank after an upgrade** despite auto-derivation or a prior user-set value.
- **First-run forces a full setup flow** — the only forward gate is "point sensei at your work".

## Scenarios

- [`scenarios/setup.feature`](scenarios/setup.feature) — app-shell nav, live edit, first-run
  gate, team defaults.

## Open gaps

- [A-1](../../../../roadmap/06-gaps.md) — the kanji ⇄ solar toggle: **resolved**. The app
  stores it as `ui.glyphs` (Settings → General); the dōjō's account menu has Theme and Glyphs.
- [A-2](../../../../roadmap/06-gaps.md) — the personal-preference surface is narrower than the
  configuration the app already supports (P3).
- No registered gap for profile-name preservation (regression check lives in the Done gate above).

## Related

- [spec/screen/preferences](../../../../spec/screen/preferences.md) — 8 panes, cues,
  pane coverage (legacy depth, folds here).
- [spec/screen/first-run-scan](../../../../spec/screen/first-run-scan.md) ·
  [spec/screen/settings-inference](../../../../spec/screen/settings-inference.md) — the gate + Inference pane.
- [features/02-config.md](../../../../features/02-config.md) — the Configuration story
  (stamped superseded; this spec is the replacement).
- [prd/02-objectives.md §02 First run & Preferences](../../../02-objectives.md) ·
  [prd/03-personas.md §Preference surface](../../../03-personas.md#preference-surface-decision-10).
- MCP: `collective_preferences` tools; regression: project_wizard_rehab memory (W1–W12 shipped, W13/W14 deferred).