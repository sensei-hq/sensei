# 観 · First run · Scan

**Segment:** 02 · First run & Preferences
**Route:** `/first-run/scan` (or the same route the Observatory shows before any root is added)
**Source mockup:** [`lib/setup/setup-wizard.jsx`](../../mockups/Sensei/lib/setup/setup-wizard.jsx) → `WizScan` with `context="first-run"`
**App file:** `app/src/routes/first-run/scan/+page.svelte`

## Purpose

The user just watched bootstrap go green. Now sensei needs to know
**where they work** — one or more root folders to scan for
projects. This is the one-and-only first-time gate: drag / browse /
paste a folder path, hit Begin, watch projects appear live.

The mockup is explicit: **the old nine-stage wizard is gone**.
Every other setting has moved into Preferences. The only forward
action on first run is "Begin" — enabled once at least one root is
added.

The scan runs live. As folders are discovered, projects show up as
they get grouped. The results banner appears at the bottom with the
sole forward action ("Enter your workspace" or similar).

Kanji is 観 — *observation*.

## Data invariants

- Empty state on first run: `sensei.scan_roots` has zero rows.
- User actions available:
  - **Drag & drop** a folder onto the drop-zone.
  - **Browse** — Tauri native folder picker.
  - **Paste** a folder path — validated (`must exist`, `must be a
    directory`, `must be readable`).
  - **Remove** an added root before Begin.
- `POST /api/scan/roots` persists each root and starts scanning
  immediately (not batched at Begin time — the user sees
  projects appear before Begin becomes active).
- Begin button state: disabled until at least one root has status
  `scanning` or `watching`; enabled otherwise.
- **Detect a company Dōjō on the network** — per the journey map
  §2.1, first run offers "join your org" if a Dōjō is
  broadcasting. Deferred to a later slice; the first-run screen
  shows a "connect to a Dōjō (optional)" link that opens the
  Preferences Connection pane.

## Signals shown

| Element | Value |
|---|---|
| Header title | "Point sensei at your work." (or the mockup's phrase) |
| Header note | "Pick a folder that contains your projects. Sensei will find them." |
| Drop-zone | large accented area with the browse button + paste field beneath |
| Added roots list | each row: kanji · path · scanning state · remove button |
| Scanning state per root | `discovering` / `indexing` / `watching` with progress |
| Projects preview | live list as projects are grouped — small kanji chips + names |
| Results banner (bottom) | "N projects found · N repos across N folders" + "Enter your workspace" button |
| Optional Dōjō connect link | small, muted |

## Done gate

- On first run, the screen renders empty and Begin is disabled.
- Adding a root immediately kicks off a scan; projects appear
  live in the preview.
- Multi-repo suggestions (see [[pipeline/capture]]) may surface
  during scan; user can accept from this screen.
- The projects preview list is the same primitive as
  [[screen/observatory-projects]] cards, at reduced size.
- Enter-your-workspace transitions to
  [[screen/first-entry-projects]] with the scan continuing in
  the background.
- The screen doesn't require the scan to complete before the
  user proceeds — enter with 5 projects, more come in
  incrementally after.
- No other setup gauntlet steps — the mockup enforces this.

Optional check:
```
# during the scan
curl -s http://localhost:7744/api/scan/roots | jq '.[].status'
curl -s http://localhost:7744/api/projects | jq 'length'
```

## Wrong gate

- **Wizard reappears with 9 stages.** Rehab regressed — this
  screen is the only first-time gate.
- **Paste field accepts a broken path silently.** Must validate
  and give clear error.
- **Begin is disabled after a root is added successfully.**
  Enable-state logic broken.
- **Adding a root shows "scanning… scanning… scanning" forever.**
  Watcher / scanner state not being emitted to the UI.
- **Projects preview doesn't update as the scan progresses.** SSE
  or polling regression.
- **Entering the workspace loses scan state.** Scan must
  continue in the background; the workspace should surface
  incomplete-scan status where appropriate.
- **Dōjō join offer fires without a Dōjō being detected.** Only
  show the option when one is broadcasting.

## Related

- [[pipeline/capture]] — the scanner that this screen triggers
- [[screen/first-entry-projects]] — the next step
- [[screen/preferences]] — where the wizard now lives as
  free-navigation settings
- [[screen/observatory-projects]] — the peer primitive for the
  projects preview
