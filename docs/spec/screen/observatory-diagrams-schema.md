# 表 · Diagrams · Schema

**Segment:** 04 · Observatory / Project window
**Route:** `/project/[id]/diagrams/schema`
**Source mockup:** `docs/mockups/Sensei/Sensei Schema and Call Graph v8.dc.html` (schema views)
**Data:** the schema store (`sensei.schema_entities` / `_columns` / `_indexes` / `_refs`, #251) + the graph's edges from described objects + `sensei.folder_projects`.
**Daemon:** `GET /api/projects/{id}/diagrams/schema` (`91a4350e`) → `{ version: 2, tables, refs, entities, deps }`, the ER half of dbd's `SchemaModel` v2.
**App files:** `diagrams/schema-state.svelte.ts`, `diagrams/schema/+page.svelte`
**Rokkit:** `ErDiagram`, `DependencyDiagram`, `EntityView`, `toGraphInput` (`@rokkit/graph@1.9.0`, `/schema`)
**Status:** BUILT 2026-10-12

## Purpose

The project's database, as a database: **which tables exist, how their keys
connect them, and what reads or writes what.** Two pictures of one payload —
tables and their keys, or every object and its dependencies — and the detail of
whichever table the reader picks.

Kanji is 表 (*table*).

## Data invariants

- **The payload is dbd's shape, passed through.** `toGraphInput(model, 'er')`
  and `toGraphInput(model, 'dependencies')` build the graph input; the screen
  never re-keys a node or renames a field. A field rokkit cannot resolve looks
  correct while silently never firing.
- **Keys draws tables only.** A view is a derived projection and a routine is
  behaviour; `ErDiagram`'s own rule. Dependencies draws every object.
- **A schema filter narrows the picture and COUNTS what it cut.** A key or a
  dependency whose other end is in a hidden schema is not drawn, and the number
  of them sits beside the picture. A key that leaves the shown schemas is a
  relationship, not an absence.
- **An unresolved dependency is drawn dimmed, never dropped.** Built-ins
  (`now()`, `count`) and objects the store does not describe are endpoints no
  card stands for; rokkit carries them as `unplaced`. Their count is shown.
- **Nothing described is a finding, not an empty canvas.** A project with no dbd
  `design.yaml` (and, after #251 step 7, no ORM mapping) says so in words.
- **A failure is not "nothing described".** A 500 or an unreachable daemon is an
  error state with a retry.

## Signals shown

- **Scope** toggle: *Tables & keys* · *Dependencies*. The first coverage line
  counts what that picture draws — `tables · keys`, or `objects · dependencies`.
- **Schemas** toggle: *All* plus one per schema in the payload, with its table
  count.
- **Coverage lines**, one fact each: tables · keys · views & routines shown;
  links cut by the schema filter; dependencies on built-ins or undescribed
  objects (dimmed).
- **Entity** panel: the selected table's columns (type, pk / nn / fk / uq),
  notes and indexes — `EntityView` over the same state.
- **Loading / error / none / graph**: four distinct states.

## Done gate

1. The screen mounts `ErDiagram` against this repository's real payload (169
   tables, 184 keys) without a runtime throw, and `DependencyDiagram` likewise
   (784 deps, 360 unresolved).
2. Choosing one schema draws only its tables, and the number of keys leading
   out of it is shown and equals the count of refs with exactly one end in it.
3. Selecting a table shows its columns in declaration order with dbd's badges.
4. A project with no described objects shows the *none* state; a daemon failure
   shows the error state, never the *none* state.
5. A schema chosen and then absent from a reloaded payload falls back to *All*
   rather than drawing an empty picture.
6. The view is in the Diagrams sub-nav and reachable from it.

**Evidence 2026-10-12** (`tests-inspect/schema-inspect.spec.ts`, live `sensei`):
169 cards / 184 lines drawn; `dojo` alone 39 / 63 with "4 keys lead out", and
under Dependencies 18 cut and 80 dimmed — each count derived independently from
the payload first; selecting `schema_refs` shows its 6 columns in order; no
runtime errors. The *none* state and the sub-nav: `tests/diagrams-schema.spec.ts`
on a fresh `sensei_e2e`. Gates 4–5 also in `schema-state.spec.svelte.ts`.

## Wrong gate

- **"No database here" when the daemon failed.** It sends the reader looking for
  a design.yaml that exists.
- **A filtered picture with no count of what it cut.** Keys into a hidden schema
  vanish and the schema reads as self-contained.
- **Unresolved dependencies dropped.** The picture reads as fully placed.
- **Views and routines as ER cards.** They have no keys; they make the ER
  picture a dependency picture.

## Not in scope

- ORM-sourced schemas (EF Core etc.) — #251 step 7; the screen reads the same
  store, so they appear without screen changes.
- Editing or exporting the schema.
- `project` / `schemas` metadata (dialect, enum counts): the store does not hold
  them, so the payload omits them (see `analysis::schema_model`).

## Related

- [#221](https://github.com/sensei-hq/sensei/issues/221) — this screen
- [#251](https://github.com/sensei-hq/sensei/issues/251) — the schema store and endpoint
- `docs/spec/indexer/20-schema-entities.md` — where the data comes from
