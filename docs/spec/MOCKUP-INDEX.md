# Sensei mockups — screen → source file index

**Read this file before starting UI work.** The mockups directory has multiple
variants of most screens (older/newer, simple/detailed, discarded); picking
the wrong file wastes work.

The authoritative shell is [`../mockups/Sensei/lib/observatory.jsx`](lib/observatory.jsx) — its
`section === "…"` switch maps a route id to the component that renders it.
This table is built from that switch (2026-07-07 — rebuilt after the sidebar-
consistency + Health-tab refactors). If a section is added/removed in
`observatory.jsx`, update this table in the same commit.

## Which file is the source of truth for which screen?

| App route | Observatory `section` id | Component | Source file |
|---|---|---|---|
| `/` (Today) | `home` | `ObsHome` | `../mockups/Sensei/lib/observatory-today.jsx` |
| `/projects` | `projects` | (inline in `observatory.jsx`; card = `ProjectCard` in `navigation.jsx`) | `../mockups/Sensei/lib/observatory.jsx` + `../mockups/Sensei/lib/navigation.jsx` |
| `/project/[id]/…` | `project` | `ProjectPageSidebar` | `../mockups/Sensei/lib/project-pages.jsx` |
| `/sessions` | `sessions` | `SessionsDigestZen` | `../mockups/Sensei/lib/sessions-zen.jsx` |
| `/logs` | `logs` | `ObsLogs` | `../mockups/Sensei/lib/project-logs.jsx` |
| `/libraries` | `libraries` | `LibrariesVariantA` | `../mockups/Sensei/lib/libraries.jsx` |
| `/insights` | `insights` | `LearningsTriage` (Triage variant) | `../mockups/Sensei/lib/learnings-v2.jsx` |
| `/memories` | `memories` | `LearningsAnatomyV2` | `../mockups/Sensei/lib/learnings-anatomy-v2.jsx` |
| `/upgrades` | `upgrades` | `InappDownstream` | `../mockups/Sensei/lib/dojo-inapp.jsx` |
| `/share-review` | `share-review` | `InappShare` | `../mockups/Sensei/lib/dojo-inapp.jsx` |
| `/consolidation` | `consolidation` | `ObsConsolidation` | `../mockups/Sensei/lib/consolidation.jsx` |
| `/traceability` | `traceability` | `ObsTraceability` | `../mockups/Sensei/lib/traceability.jsx` |
| `/impact` | `impact` | `ObsImpact` | `../mockups/Sensei/lib/impact.jsx` |
| `/impact` (alert) | `impact-alert` | `ObsNegativeAlert` | `../mockups/Sensei/lib/impact.jsx` |
| `/dojo/connections` | `dojo-connections` | `InappConnection` | `../mockups/Sensei/lib/dojo-inapp.jsx` |
| `/dojo/sharing` | `dojo-sharing` | `ObsCollectiveSettings` | `../mockups/Sensei/lib/collective-settings.jsx` |
| `/instruments` (Playground) | `instruments-playground` | **`InstrumentsPlaygroundSimple`** | **`../mockups/Sensei/lib/instruments-simple.jsx`** |
| `/instruments/replay` | `instruments-replay` | **`InstrumentsReplaySimple`** → delegates to `InstrumentsReplay` | **`../mockups/Sensei/lib/instruments-simple.jsx`** + `../mockups/Sensei/lib/instruments.jsx` |
| `/instruments/health` | `instruments-health` | **`InstrumentsHealthSimple`** → delegates to `InstrumentsHealth` | **`../mockups/Sensei/lib/instruments-simple.jsx`** + `../mockups/Sensei/lib/instruments.jsx` |

## Active vs discarded — which file is current

The artboards `<script src>`-load **almost every** file (incl. `discarded/*`), so
imports don't tell you what's live. The real signal is **which component an
artboard renders**:

- **Active** = rendered by `Sensei Observatory.html`.
- **Old** = rendered only in `Sensei Experiments.html`, or anything under `discarded/`.

The pairs that cause confusion (⚠️ two files can define the same component name):

| Screen | ✅ Current file | ❌ Old / discarded |
|---|---|---|
| Bootstrap splash | `lib/bootstrap-splash.jsx` (`Splash`, `SplashOnDesktop`) | `lib/splash-healthcheck.jsx` (`Splash`) · `lib/discarded/bootstrap.jsx` · `lib/discarded/bootstrap-simple.jsx` |
| Insights / Learnings | `lib/learnings-v2.jsx` + `lib/learnings-anatomy-v2.jsx` | `lib/discarded/learnings.jsx` |
| Sessions | `lib/sessions-zen.jsx` | `lib/discarded/sessions.jsx` |
| Project window | `lib/project-pages.jsx` → `ProjectPageSidebar` | same file → `ProjectPageTopTabs` (old variant) |
| Wizard steps | `lib/wiz-assignments.jsx` · `lib/wiz-inference.jsx` | `lib/discarded/wiz-assignments-tabs.jsx` · `lib/discarded/wiz-inference-ladder.jsx` |
| Direction studies | none (direction landed) | `lib/discarded/direction-{enso,ma,shoji,merged}.jsx` |
| Assistant chips | none (experiment) | `lib/assistant-tick-options.jsx` |

## Screens outside the Observatory switch

The table above (`observatory.jsx` switch) covers the Observatory rail. These
screens live elsewhere:

| Screen spec | Component | Source file |
|---|---|---|
| `bootstrap-green` / `bootstrap-probing` | `Splash` (state prop) | `lib/bootstrap-splash.jsx` |
| `first-run-scan` | `WizScan` | `lib/setup-wizard.jsx` |
| `preferences` | `SetupWizard` | `lib/setup-wizard.jsx` |
| `settings-inference` | `InferenceSettings` | `lib/inference-settings.jsx` |
| `project-overview` / `project-memories` / `project-about` | `ProjOverviewLite` / `ProjMemoriesLite` / `ProjAboutPane` | `lib/project-lite-panes.jsx` |
| `project-patterns` / `project-sessions` | (`ProjectPageSidebar` tabs) | `lib/project-pages.jsx` |
| `dojo-{admin,lead,maintainer}-console` | (panels) | `lib/dojo-console.jsx` |
| `dojo-developer-flow` | `Inapp*` | `lib/dojo-inapp.jsx` |
| `insights-reasoning` | (MOE section) | `lib/mcp-replay-insights.jsx` |
| **Relay** (14 screens, new) | `Relay*` | `lib/relay.jsx` · `lib/relay-planner.jsx` · `lib/relay-desktop.jsx` |

## Sub-nav placement (2026-07-07 refactor)

Two placement rules — the Instruments group is the exception:

- **Default (top-of-main).** For every group except `instruments`, the
  sub-tab strip is rendered by `observatory.jsx` at the top of the main
  container, above the section body. Sections don't need to know sub-nav
  exists.
- **Instruments (below the hero).** Instruments sub-tabs render **inside**
  each Instruments screen, between its hero and its content. The parent
  passes `subNav` as a JSX prop; the screen decides exactly where it goes.
  Reason: Instruments screens each own their own chrome and had a
  duplicate sub-tab strip when the observatory shell added one above.

If you add a new group whose screens own their own chrome, follow the
Instruments pattern: `groupKeyOf(section) !== "instruments"` gate lives in
`observatory.jsx` — extend it.

## Data variants worth knowing

| Data source | Location | Notes |
|---|---|---|
| Signals + Health | `../mockups/Sensei/lib/mcp-signals-data.js` → `window.MCP_SIGNALS` | Health opens at **MCP-level** (see `mcpMeta`). Per-tool detail is `toolUsage` (Sensei tools) + `thirdPartyUsage` (non-Sensei). |
| Today | `../mockups/Sensei/lib/observatory-today.jsx` → `window.OBS_DATA` | Early / mature variants on `dataMaturity`. |
| Projects | `../mockups/Sensei/lib/project-data.js` → `window.PROJECTS_INDEX` | Card supports `p.vision` for the description row. |
| Sessions | `../mockups/Sensei/lib/sessions-zen.jsx` → `window.SESSIONS` | Synthesized history for prior-week windows. |

## Deprecated / discarded — DO NOT use as a target

Everything under `lib/discarded/` is off. Plus two top-level files that are older
experiments (NOT yet moved to `discarded/`). Verified 2026-07-14 against what the
artboards render:

| Deprecated file | Superseded by |
|---|---|
| `lib/discarded/learnings.jsx` | `lib/learnings-v2.jsx` + `lib/learnings-anatomy-v2.jsx` |
| `lib/discarded/sessions.jsx` | `lib/sessions-zen.jsx` |
| `lib/discarded/bootstrap.jsx` · `lib/discarded/bootstrap-simple.jsx` | `lib/bootstrap-splash.jsx` |
| `lib/discarded/direction-{enso,ma,shoji,merged}.jsx` | direction has landed; see `hq/site.jsx` |
| `lib/discarded/wiz-assignments-tabs.jsx` · `lib/discarded/wiz-inference-ladder.jsx` | `lib/wiz-assignments.jsx` · `lib/wiz-inference.jsx` |
| **`lib/splash-healthcheck.jsx`** (top-level, older splash) | `lib/bootstrap-splash.jsx` — retire or move to `discarded/` |
| **`lib/assistant-tick-options.jsx`** (top-level, experiment) | not wired; experiment only |
| `lib/instruments.jsx` (three-tab shell) | `lib/instruments-simple.jsx` — but the `-simple` wrappers delegate INTO `instruments.jsx` for Replay/Health, so it's **not dead**; keep both |

**Unresolved (needs a designer call):** `lib/upgrades.jsx` and `lib/sharing-review.jsx`
are top-level but the specs render `lib/dojo-inapp.jsx` (`InappDownstream` / `InappShare`)
instead — likely superseded, confirm before pruning.

## Working rules

1. **Start from `observatory.jsx` when you don't know which file to open.**
   Grep for the section id or route; the rendered component name → the file
   where it's defined.
2. **Prefer files named `*-simple.jsx` or `*-v2.jsx` over their siblings** —
   they are the newer iterations that observatory.jsx wires. Exception:
   the Instruments `-simple` wrappers delegate INTO `instruments.jsx` for
   Replay/Health, so keep both open when touching those.
3. **When in doubt, verify against the observatory switch**, not the file
   name.
4. **Never target a file in `discarded/`.**
5. **Related spec docs live under `docs/spec/`.** Every screen has a
   spec that names its source file in the front-matter.

## Design conventions

See [`docs/mockups/Sensei/CLAUDE.md`](../mockups/Sensei/CLAUDE.md) for token / type / spacing rules.
Every screen wraps in `.sensei` (or `.zs` / `.artboard-shell`) to activate the
utility classes.
