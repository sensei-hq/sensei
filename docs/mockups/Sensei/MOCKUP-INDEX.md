# Sensei mockups — screen → source file index

**Read this file before starting UI work.** The mockups directory has multiple
variants of most screens (older/newer, simple/detailed, discarded); picking
the wrong file wastes work.

The authoritative shell is
[`lib/observatory.jsx`](lib/observatory.jsx) — its `section === "…"` switch
maps a route id to the component that renders it. This table is built from
that switch (2026-07-06). If a section is added/removed in `observatory.jsx`,
add/remove the row here in the same commit.

## Which file is the source of truth for which screen?

| App route | Observatory `section` id | Component rendered | Source file |
|---|---|---|---|
| `/` (Today) | `home` | `ObsHome` | `lib/observatory-today.jsx` |
| `/projects` | `projects` | (inline in `observatory.jsx`) | `lib/observatory.jsx` |
| `/project/[id]/…` | `project` | `ProjectPageSidebar` | `lib/project-pages.jsx` |
| `/sessions` | `sessions` | `SessionsDigestZen` | `lib/sessions-zen.jsx` |
| `/logs` | `logs` | `ObsLogs` | `lib/project-logs.jsx` |
| `/libraries` | `libraries` | `LibrariesVariantA` | `lib/libraries.jsx` |
| `/insights` (top-nav) | `insights` | `LearningsTriage` | `lib/learnings-v2.jsx` |
| `/memories` (Learnings) | `memories` | `LearningsAnatomyV2` | `lib/learnings-anatomy-v2.jsx` |
| `/upgrades` | `upgrades` | `InappDownstream` | `lib/dojo-inapp.jsx` |
| `/share-review` | `share-review` | `InappShare` | `lib/dojo-inapp.jsx` |
| `/consolidation` | `consolidation` | `ObsConsolidation` | `lib/consolidation.jsx` |
| `/traceability` | `traceability` | `ObsTraceability` | `lib/traceability.jsx` |
| `/impact` | `impact` | `ObsImpact` | `lib/impact.jsx` |
| `/impact` (alert state) | `impact-alert` | `ObsNegativeAlert` | `lib/impact.jsx` |
| `/dojo/connections` | `dojo-connections` | `InappConnection` | `lib/dojo-inapp.jsx` |
| `/dojo/sharing` | `dojo-sharing` | `ObsCollectiveSettings` | `lib/collective-settings.jsx` |
| `/instruments` (Playground) | `instruments-playground` | **`InstrumentsPlaygroundSimple`** | **`lib/instruments-simple.jsx`** |
| `/instruments/replay` | `instruments-replay` | **`InstrumentsReplaySimple`** | **`lib/instruments-simple.jsx`** |
| `/instruments/health` | `instruments-health` | **`InstrumentsHealthSimple`** | **`lib/instruments-simple.jsx`** |

## Deprecated / discarded — DO NOT use as a target

Anything in `lib/discarded/` is off. Additionally these `lib/*.jsx` files are
older variants superseded by the "-simple" or `-v2` file listed above:

| Deprecated file | Superseded by |
|---|---|
| `lib/instruments.jsx` (three-tab shell) | `lib/instruments-simple.jsx` (three independent sidebar destinations). observatory.jsx wires the `Simple` variants. |
| `lib/learnings.jsx` (moved to `discarded/`) | `lib/learnings-v2.jsx` + `lib/learnings-anatomy-v2.jsx` |
| `lib/sessions.jsx` (moved to `discarded/`) | `lib/sessions-zen.jsx` |
| `lib/bootstrap.jsx` (moved to `discarded/`) | `lib/bootstrap-simple.jsx` — but see observatory.jsx for the wired variant if any |
| `lib/direction-*.jsx` (moved to `discarded/`) | direction has landed; see `hq/site.jsx` etc. |
| `lib/wiz-*.jsx` (moved to `discarded/`) | wizard rehab shipped; `lib/setup-wizard.jsx` is current |

## Working rules

1. **Start from `observatory.jsx` when you don't know which file to open.**
   Grep for the section id or route; the rendered component name → the file
   where it's defined.
2. **Prefer files named `*-simple.jsx` or `*-v2.jsx` over their siblings** —
   they are the newer iterations that observatory.jsx wires.
3. **When in doubt, verify against the observatory switch**, not the file
   name. A "simple" name is a strong hint but observatory.jsx is the tiebreaker.
4. **Never target a file in `discarded/`** — those are archived old iterations.

## Design conventions

See [`CLAUDE.md`](CLAUDE.md) in this directory for token / type / spacing rules.
Every screen wraps in `.sensei` (or `.zs` / `.artboard-shell`) to activate the
utility classes.
