# Sensei mockups — screen → source file index

**Read this file before starting UI work.** The mockups directory has multiple
variants of most screens (older/newer, simple/detailed, discarded); picking
the wrong file wastes work.

The authoritative shell is [`../mockups/Sensei/lib/observatory/observatory.jsx`](lib/observatory/observatory.jsx) — its
`section === "…"` switch maps a route id to the component that renders it.
This table is built from that switch (2026-07-07 — rebuilt after the sidebar-
consistency + Health-tab refactors). If a section is added/removed in
`observatory.jsx`, update this table in the same commit.

## Which file is the source of truth for which screen?

| App route | Observatory `section` id | Component | Source file |
|---|---|---|---|
| `/` (Today) | `home` | `ObsHome` | `../mockups/Sensei/lib/observatory/observatory-today.jsx` |
| `/projects` | `projects` | (inline in `observatory.jsx`; card = `ProjectCard` in `navigation.jsx`) | `../mockups/Sensei/lib/observatory/observatory.jsx` + `../mockups/Sensei/lib/shared/navigation.jsx` |
| `/project/[id]/…` | `project` | `ProjectPageSidebar` | `../mockups/Sensei/lib/project/project-pages.jsx` |
| `/sessions` | `sessions` | `SessionsDigestZen` | `../mockups/Sensei/lib/observatory/sessions-zen.jsx` |
| `/logs` | `logs` | `ObsLogs` | `../mockups/Sensei/lib/project/project-logs.jsx` |
| `/libraries` | `libraries` | `LibrariesVariantA` | `../mockups/Sensei/lib/observatory/libraries.jsx` |
| `/insights` | `insights` | `LearningsTriage` (Triage variant) | `../mockups/Sensei/lib/observatory/learnings-v2.jsx` |
| `/memories` | `memories` | `LearningsAnatomyV2` | `../mockups/Sensei/lib/observatory/learnings-anatomy-v2.jsx` |
| `/upgrades` | `upgrades` | `InappDownstream` | `../mockups/Sensei/lib/dojo/dojo-inapp.jsx` |
| `/share-review` | `share-review` | `InappShare` | `../mockups/Sensei/lib/dojo/dojo-inapp.jsx` |
| `/consolidation` | `consolidation` | `ObsConsolidation` | `../mockups/Sensei/lib/observatory/consolidation.jsx` |
| `/traceability` | `traceability` | `ObsTraceability` | `../mockups/Sensei/lib/observatory/traceability.jsx` |
| `/impact` | `impact` | `ObsImpact` | `../mockups/Sensei/lib/observatory/impact.jsx` |
| `/impact` (alert) | `impact-alert` | `ObsNegativeAlert` | `../mockups/Sensei/lib/observatory/impact.jsx` |
| `/dojo/connections` | `dojo-connections` | `InappConnection` | `../mockups/Sensei/lib/dojo/dojo-inapp.jsx` |
| `/dojo/sharing` | `dojo-sharing` | `ObsCollectiveSettings` | `../mockups/Sensei/lib/observatory/collective-settings.jsx` |
| `/instruments` (Playground) | `instruments-playground` | **`InstrumentsPlaygroundSimple`** | **`../mockups/Sensei/lib/observatory/instruments-simple.jsx`** |
| `/instruments/replay` | `instruments-replay` | **`InstrumentsReplaySimple`** → delegates to `InstrumentsReplay` | **`../mockups/Sensei/lib/observatory/instruments-simple.jsx`** + `../mockups/Sensei/lib/observatory/instruments.jsx` |
| `/instruments/health` | `instruments-health` | **`InstrumentsHealthSimple`** → delegates to `InstrumentsHealth` | **`../mockups/Sensei/lib/observatory/instruments-simple.jsx`** + `../mockups/Sensei/lib/observatory/instruments.jsx` |

## Active vs discarded — which file is current

The artboards `<script src>`-load **almost every** file (incl. `discarded/*`), so
imports don't tell you what's live. The real signal is **which component an
artboard renders**:

- **Active** = rendered by `Sensei Observatory.html`.
- **Old** = rendered only in `Sensei Experiments.html`, or anything under `discarded/`.

The pairs that cause confusion (⚠️ two files can define the same component name):

| Screen | ✅ Current file | ❌ Old / discarded |
|---|---|---|
| Bootstrap splash | `lib/setup/bootstrap-splash.jsx` (`Splash`, `SplashOnDesktop`) | `lib/discarded/splash-healthcheck.jsx` (`Splash`) · `lib/discarded/bootstrap.jsx` · `lib/discarded/bootstrap-simple.jsx` |
| Insights / Learnings | `lib/observatory/learnings-v2.jsx` + `lib/observatory/learnings-anatomy-v2.jsx` | `lib/discarded/learnings.jsx` |
| Sessions | `lib/observatory/sessions-zen.jsx` | `lib/discarded/sessions.jsx` |
| Project window | `lib/project/project-pages.jsx` → `ProjectPageSidebar` | same file → `ProjectPageTopTabs` (old variant) |
| Wizard steps | `lib/setup/wiz-assignments.jsx` · `lib/setup/wiz-inference.jsx` | `lib/discarded/wiz-assignments-tabs.jsx` · `lib/discarded/wiz-inference-ladder.jsx` |
| Direction studies | none (direction landed) | `lib/discarded/direction-{enso,ma,shoji,merged}.jsx` |
| Assistant chips | none (experiment) | `lib/discarded/assistant-tick-options.jsx` |

## Screens outside the Observatory switch

The table above (`observatory.jsx` switch) covers the Observatory rail. These
screens live elsewhere:

| Screen spec | Component | Source file |
|---|---|---|
| `bootstrap-green` / `bootstrap-probing` | `Splash` (state prop) | `lib/setup/bootstrap-splash.jsx` |
| `first-run-scan` | `WizScan` | `lib/setup/setup-wizard.jsx` |
| `preferences` | `SetupWizard` | `lib/setup/setup-wizard.jsx` |
| `settings-inference` | `InferenceSettings` | `lib/setup/inference-settings.jsx` |
| `project-overview` / `project-memories` / `project-about` | `ProjOverviewLite` / `ProjMemoriesLite` / `ProjAboutPane` | `lib/project/project-lite-panes.jsx` |
| `project-patterns` / `project-sessions` | (`ProjectPageSidebar` tabs) | `lib/project/project-pages.jsx` |
| `dojo-admin-console` | `DojoAdminConsole` (Overview · Monitor · Members · Scopes) | `lib/dojo/dojo-admin.jsx` |
| `dojo-maintainer-console` | `DojoMaintainerConsole` (Triage · Candidate · Knowledge) | `lib/dojo/dojo-maintainer.jsx` |
| `dojo-lead-console` | `DojoLeadConsole` (Clients · Audit) | `lib/dojo/dojo-lead.jsx` |
| `dojo-developer-console` | `DojoDeveloperConsole` (My teams · My contributions · For me) | `lib/dojo/dojo-developer.jsx` |
| Ecosystem architecture board | `EcosystemArchitecture` | `lib/shared/ecosystem-arch.jsx` (Observatory section ⑧) |
| `dojo-developer-flow` | `Inapp*` | `lib/dojo/dojo-inapp.jsx` |
| `insights-reasoning` | (MOE section) | `lib/observatory/mcp-replay-insights.jsx` |
| **Relay** (13 screens, specced `relay-*.md`) | `Relay*` (skip `RelayArchitecture` — concept diagram) | `lib/relay/relay.jsx` · `lib/relay/relay-planner.jsx` · `lib/relay/relay-desktop.jsx` |
| `project-atlas` (code graph) | `ProjectAtlasWindow` | `lib/project/project-atlas.jsx` |
| `agent-editor` / `persona-editor` | `AgentEditor` / `PersonaEditor` | `lib/observatory/agent-persona-editors.jsx` |
| `benchmark-runner` | `BenchmarkRunnerDashboard` / `…Notebook` | `lib/observatory/benchmark.jsx` |
| `solution-{architecture,dashboard,sessions}` | `Solution*` | `lib/observatory/solution-track.jsx` |

> **Not app screens:** `lib/dojo/dojo-saas.jsx` (`DojoOrgs`, `DojoSignIn`) is the
> Dōjō **SaaS website** surface, not a desktop-app screen — no `spec/screen/`.
> `lib/observatory/perspective-split.jsx` is a **WIP exploration** (collective vs
> Dōjō); not specced yet.
>
> **Dōjō touchpoints (Observatory ⑦):** the in-app Dōjō flows (`lib/dojo/dojo-inapp.jsx`
> — `InappConnection` / `InappShare` / `InappDownstream`) are wired into the
> **Observatory** as a section, not the console. The specs `observatory-dojo-connections`,
> `observatory-share-review`, `observatory-upgrades` cover them and correctly point
> at `dojo-inapp.jsx`.

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
| Signals + Health | `../mockups/Sensei/lib/data/mcp-signals-data.js` → `window.MCP_SIGNALS` | Health opens at **MCP-level** (see `mcpMeta`). Per-tool detail is `toolUsage` (Sensei tools) + `thirdPartyUsage` (non-Sensei). |
| Today | `../mockups/Sensei/lib/observatory/observatory-today.jsx` → `window.OBS_DATA` | Early / mature variants on `dataMaturity`. |
| Projects | `../mockups/Sensei/lib/data/project-data.js` → `window.PROJECTS_INDEX` | Card supports `p.vision` for the description row. |
| Sessions | `../mockups/Sensei/lib/observatory/sessions-zen.jsx` → `window.SESSIONS` | Synthesized history for prior-week windows. |

## Deprecated / discarded — DO NOT use as a target

Everything under `lib/discarded/` is off. `lib/` was reorganized into folders on
2026-07-14 (`shared/ setup/ observatory/ project/ dojo/ relay/ data/ discarded/`);
each artboard now imports only what it renders. Superseded files:

| Deprecated file | Superseded by |
|---|---|
| `lib/discarded/learnings.jsx` | `lib/observatory/learnings-v2.jsx` + `lib/observatory/learnings-anatomy-v2.jsx` |
| `lib/discarded/sessions.jsx` | `lib/observatory/sessions-zen.jsx` |
| `lib/discarded/bootstrap.jsx` · `lib/discarded/bootstrap-simple.jsx` | `lib/setup/bootstrap-splash.jsx` |
| `lib/discarded/direction-{enso,ma,shoji,merged}.jsx` | direction has landed; see `hq/site.jsx` |
| `lib/discarded/wiz-assignments-tabs.jsx` · `lib/discarded/wiz-inference-ladder.jsx` | `lib/setup/wiz-assignments.jsx` · `lib/setup/wiz-inference.jsx` |
| `lib/discarded/splash-healthcheck.jsx` (`SplashHealthCheck`) | `lib/setup/bootstrap-splash.jsx` (`Splash`) |
| `lib/discarded/assistant-tick-options.jsx` | experiment only; not wired |
| `lib/discarded/extensions-browser.jsx` · `lib/discarded/skill-editor.jsx` | dead (rendered nowhere) |
| `lib/discarded/dojo-console.jsx` (one console, all roles) | split into `lib/dojo/dojo-admin.jsx` · `dojo-maintainer.jsx` · `dojo-lead.jsx` (+ shared frame `lib/dojo/dojo-shared.jsx`) |
| `lib/observatory/instruments.jsx` (three-tab shell) | `lib/observatory/instruments-simple.jsx` — but the `-simple` wrappers delegate INTO `instruments.jsx` for Replay/Health, so it's **not dead**; keep both |

**Global vs per-Dōjō — NOT a duplicate.** `lib/observatory/upgrades.jsx`
(`ObsUpgrades`) and `lib/observatory/sharing-review.jsx` (`ObsSharingReview`) are
the **global Collective** upgrades / share screens. `lib/dojo/dojo-inapp.jsx`
(`InappDownstream` / `InappShare`) is the **per-Dōjō** version the observatory
routes wire today. Both are kept — they are different scopes.

## Screen states (empty · loading · error)

Screens take a `state` prop and early-return a scaffold from
[`lib/shared/screen-states.jsx`](../mockups/Sensei/lib/shared/screen-states.jsx)
(`ScreenState`): **loading** (spinner + skeleton), **empty** (kanji + what's
missing + how to get data), **error** (message + Retry). The happy path renders
when `state="ready"`. Usage: `<ObsTraceability state="loading" />`.

Screens with all three states today: solution-track (+ demo artboards
`sol-loading`/`sol-empty`/`sol-error`), Libraries, Traceability, Impact,
Consolidation, Collective-settings, Logs, the project panes (Memories/Impact),
and the project window. Per-state *artboards* exist for solution-track only so far;
the rest expose the states via the prop. `screen-states.jsx` is loaded on
Observatory / Components / Experiments / _capture — add the script line if you
mount these screens on another page.

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
