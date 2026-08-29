# 名 · Preferences

**Segment:** 02 · First run & Preferences
**Route:** `/preferences` with sub-panes
**Source mockup:** [`lib/setup/setup-wizard.jsx`](../../mockups/Sensei/lib/setup/setup-wizard.jsx) → `SetupWizard` with `mode="preferences"` (wizard rehab lives here)
**App file:** `app/src/routes/preferences/+page.svelte`

## Purpose

Preferences is where the **old nine-stage wizard now lives** —
folders, projects, assistants, libraries, instruments, inference,
assignments, profile. The journey map's fix: none of these are
first-run gates any more. They're free-navigation settings the
user visits when they want to tune what defaults got wrong.

Preferences is **searchable** — a text field at the top narrows
across every pane's contents. And every pane carries "review cues"
that surface when sensei has data suggesting a tuning ("your top
assistant sends 40% of sessions to Claude Code but you haven't
picked a default persona for that family").

Kanji is 名 — *name / designation*.

## Panes (8)

Ordered by frequency of visit:

1. **Folders** — scan roots list; add / remove / exclude
   directories. Branch policy (`active-only / pinned / all`)
   configurable here.
2. **Projects** — grouping, multi-repo suggestions, project
   vision editing, dojo membership binding.
3. **Assistants** — installed families (Claude Code, Zed,
   Cursor), integration state, per-family personas / skills.
4. **Libraries** — indexed libraries, wrap-me candidates, add a
   new library (local / GitHub / website).
5. **Instruments** — MCP surface tuning; per-tool
   enable/disable; third-party MCP config visibility.
6. **Inference** — gateway defaults; per-chain model selection
   (embedded gemma4 vs remote); narration-cache on/off with
   fallback templates.
7. **Assignments** — which project → which assistant family
   binding; per-project default persona.
8. **Profile** — user identity, dojo memberships, attribution
   defaults. **User name auto-derives from the home folder on
   first boot** (`/Users/Jerry` → "Jerry"; Linux/Windows
   analogues). The user can override — auto-derivation only
   fills a *blank* field; a user-set name is never overwritten.
   Regression to watch: the field going blank on upgrade instead
   of preserving the derived-or-set value.

Each pane matches a `WizStage` from the mockup's wizard
architecture. **The connection pane** — pair with a company
Sensei server / Dōjō — sits under Profile as a sub-section
(journey map §2.2).

## Data invariants

- `GET /api/preferences` returns the flattened preferences
  document keyed by pane.
- `PUT /api/preferences/{pane}/{key}` updates a single value.
- Every value carries a `default` and a `user_value`; the UI
  shows both when they differ (so the user knows what
  "restore default" would set it to).
- Review cues: `GET /api/preferences/cues` returns a list of
  `{ pane, key, reason, evidence }` — the daemon's suggestions
  for what to tune first.
- Search: client-side across all pane content by label /
  description / current value. Match highlights the pane and
  jumps to the row.

## Signals shown

Top strip:

| Element | Value |
|---|---|
| Header title | `Preferences · 名` |
| Search input | filters every pane |
| Review-cues chip | count of suggested tunings |

Left rail (pane picker):

| Element | Value |
|---|---|
| Pane row | pane kanji + name + optional review-cue count chip |
| Selected pane | left accent border + muted background |

Right pane (contents):

- Standard config-row primitive: label · description · widget ·
  optional `default: X` chip when overridden · optional
  review-cue banner.

## Done gate

- Search across every pane finds every configurable value that
  contains the query term.
- Every review cue links directly to the row it's about.
- Editing a value writes immediately (no Save/Cancel modal) —
  Preferences panes are direct-manipulation.
- Restore-default is available on any overridden value.
- Panes deep-link — `/preferences/folders` opens the Folders
  pane directly.
- The Connection sub-section under Profile can pair with a Dōjō,
  authenticate, and choose which scopes (teams, stacks) to
  follow (per journey map §2.2 opportunities).
- No hidden panes; every setting the wizard used to cover is
  reachable from here.

Optional check:
```
curl -s http://localhost:7744/api/preferences | jq '. | keys'
# expected: [assistants, assignments, folders, inference, instruments,
#            libraries, profile, projects]

curl -s http://localhost:7744/api/preferences/cues | jq 'length'
```

## Wrong gate

- **A setting from the old wizard isn't reachable from any
  pane.** Wizard rehab incomplete.
- **Search misses a value that clearly matches.** Hay-string
  needs to include the current value AND the label AND the
  description.
- **Review cue links to a pane but not the specific row.** Deep-
  link inside pane must scroll and highlight.
- **Saving requires Save/Cancel dance.** Direct manipulation
  regressed.
- **Connection pane forces user through a wizard.** Should be a
  self-contained pane with no forward-back stepping.
- **Restore-default doesn't reset the value.** Or resets to the
  wrong default.
- **Profile user name blank after an upgrade** despite having
  been auto-derived (or manually set) previously. Auto-derive
  fills only when blank at boot, and upgrades must preserve
  existing values. Bug already observed once — 2026-07-07 —
  needs a regression check.

## Related

- [[screen/first-run-scan]] — where the wizard used to be
- [[screen/first-entry-projects]] — link back to Preferences
- [[pipeline/governance]] — Rules panel deep-links here
- [[pipeline/dojo-lifecycle]] — Connection pane
- (memory: project_wizard_rehab_pickup) (memory) — W1-W12 shipped;
  W13 (WKWebView) deferred; W14 not merged
