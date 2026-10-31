# 計 · Workspace window · Metrics · SDLC settings

**Segment:** 04 · The workspace window
**Route:** `/workspace/[id]/metrics`, as a panel on the existing Metrics screen.
**Source mockup:** none. The panel follows the settings sections' input + save pattern.
**App files:**
- `app/src/routes/(workspace)/workspace/[id]/metrics/MetricSettingsPanel.svelte` (template)
- `metric-settings-state.svelte.ts` (state, words, actions), with its state spec and
  component spec
- `app/src/lib/api.ts`: `tryGetMetricSettings`, `tryPutMetricSettings`

**Daemon contract:** [`spec/daemon/tracker-sdlc-metrics.md`](../daemon/tracker-sdlc-metrics.md)
S1 (`GET`/`PUT /api/workspaces/{id}/metric-settings`).

## Purpose

These settings turn a ticket's measured time into cost and calibration:
- an hourly rate, for effort and rework cost;
- the minutes one story point is expected to take, for expected time and variance;
- the idle gap that ends a work slice.

They are project-level, never per person (D3).

## States

**Loading.** "Reading settings…".

**Error.** The read failed: the shared tracker error text and **Retry**. The panel never
falls back to the defaults on a failed read.

**Ready.** A `section[data-section="metric-settings"]` headed **SDLC settings**, with three
labelled number inputs:

| Label | Key | Help | Empty means |
|---|---|---|---|
| Hourly rate | `hourly_rate` | "For effort and rework cost. Project-level, never per person." | not set (cost figures stay null) |
| Minutes per story point | `minutes_per_point` | "For expected time and its variance." | not set |
| Idle gap that ends a work slice (minutes) | `idle_threshold_minutes` | "Default 15." | required |

- **Save** sends only the keys that changed. A cleared rate is sent as `null`.
- A `400 invalid_setting` names its key in the body. The panel shows "Check this value."
  under that input (`data-setting-error=<key>`) and keeps what was typed.
- Any other failure shows the shared error text as the panel alert.
- A successful save adopts the daemon's echo and says "Saved.".
- With nothing changed, Save is disabled.

## Done gate

1. The Metrics screen shows the panel with the daemon's three values. An unset rate shows
   empty, never `0`.
2. Changing one value sends `PUT` with exactly that key. Clearing the rate sends
   `{"hourly_rate": null}`.
3. A 400 naming `idle_threshold_minutes` marks that input, and the others stay unmarked.
4. A failed read shows the error with Retry and no inputs.
5. a11y: every input has a programmatic label, and the error is tied to its input
   (`aria-describedby`).
6. Tests: state spec and component spec.

## Wrong gate

- The defaults shown because the read failed.
- A rate stored or shown per person.
- `0` sent for a cleared rate.
- A save that sends keys the user did not change.
