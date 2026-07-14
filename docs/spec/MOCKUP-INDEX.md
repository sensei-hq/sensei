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

Anything in `../mockups/Sensei/lib/discarded/` is off. These `../mockups/Sensei/lib/*.jsx` files are older
variants superseded by the "-simple" or `-v2` files above:

| Deprecated file | Superseded by |
|---|---|
| `../mockups/Sensei/lib/instruments.jsx` (three-tab shell) | `../mockups/Sensei/lib/instruments-simple.jsx` (three independent sidebar destinations; the `-simple` wrappers delegate INTO `instruments.jsx` for the actual Replay / Health rendering, so the file isn't dead — it's just no longer the wire target) |
| `../mockups/Sensei/lib/learnings.jsx` | `../mockups/Sensei/lib/learnings-v2.jsx` + `../mockups/Sensei/lib/learnings-anatomy-v2.jsx` |
| `../mockups/Sensei/lib/sessions.jsx` | `../mockups/Sensei/lib/sessions-zen.jsx` |
| `../mockups/Sensei/lib/bootstrap.jsx` | `../mockups/Sensei/lib/bootstrap-simple.jsx` + `../mockups/Sensei/lib/bootstrap-splash.jsx` |
| `../mockups/Sensei/lib/direction-*.jsx` | direction has landed; see `hq/site.jsx` |
| `../mockups/Sensei/lib/wiz-*.jsx` | wizard rehab shipped; `../mockups/Sensei/lib/setup-wizard.jsx` is current |

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
