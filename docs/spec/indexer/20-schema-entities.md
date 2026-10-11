---
name: Schema entities
description: Database structure — tables, views and routines as graph nodes, columns/indexes/foreign keys in a schema store keyed by them — from dbd's SchemaModel v2 and from ORM layers
date: 2026-09-27
revised: 2026-10-08
status: approved (#251)
---

> **Revised 2026-10-08 (#251, decision in [`docs/plan/decisions.md`](../../plan/decisions.md)).**
> The first draft made every column a `field` node and stored schema detail in
> `nodes.props`. Both are gone: tables, views and routines stay nodes, and
> columns, indexes and column-level foreign keys live in four `schema_*` tables
> keyed by `node_id`. Nodes stay light; the detail is as rich as dbd's model.

# Schema entities

A database is structure a reader navigates, and today the indexer records almost
none of it. This states how a table, a column, an index and a foreign key become
graph facts, where the source is dbd's `SchemaModel` v2 or an ORM layer, and what
is deliberately not attempted yet.

The measurements behind it are in
[`docs/analysis/2026-09-27-entity-graph-feasibility.md`](../../analysis/2026-09-27-entity-graph-feasibility.md).

## The one rule this rests on

**The database's own name is the identity; every other producer defers to it.**

`nodes_unique_fqn` makes the fqn the lookup key, so duplication is not removed
afterwards — it is prevented by two producers minting one fqn, or it is not
prevented. A table is therefore identified by
`Form::Item { lang: Sql, package, module: schema, name: table }` whether dbd or an
ORM described it.

A model class keeps its **own** identity in its own language. The correspondence
between `Order` and `sensei.orders` is an **edge**, not a shared node: they are
two things that correspond, not one thing described twice. Collapsing them would
make a C# class disappear from C# queries.

And the corollary that decides the hard cases: **a producer that cannot determine
the schema-qualified name must not invent one.** A guessed `public.orders` against
an actual `sensei.orders` is a wrong edge, which R4 ranks below no edge. This is
the same rule that made the SQL walk's first-schema fallback wrong.

## Where the facts come from

Two producers, and they answer different questions.

| | `SchemaModel` v2 (dbd ≥ 0.22) | the file walk |
|---|---|---|
| tables, columns, indexes | yes, schema-qualified | tables only |
| views, matviews, routines | yes (`entities`) | yes |
| foreign keys | yes (`refs`), column-to-column | table-level only |
| read/write/call graph | yes (`deps`), pre-resolved | yes, per file |
| spans (line numbers) | **no** | yes |
| file attribution | **no** | yes |
| enums, sequences | **no** | yes |

Neither is a superset. The model has the resolved structure; the walk has the
location. A reader who wants "jump to where this table is declared" needs the
walk; one who wants "what reads this table" is better served by the model, where
the answer is already resolved.

### v2's vocabulary already matches ours

This is not a coincidence worth papering over — it means no translation layer:

- `DepEdge.kind` is `reads | writes | calls | member`, which is exactly
  `facts::RefKind::{Reads, Writes, Calls}` plus the `Member` that maps to
  `TypeUse` (see `lang/sql/facts.rs`).
- `DepEdge.unresolved` carries the same doctrine as `Resolution::Unresolved`:
  *"the edge is real, the endpoint is not placeable"* — dim, never drop. That is
  R2/A2 stated in dbd's words.

So `deps` maps 1:1 onto references, and an unresolved endpoint becomes an
`Unresolved` target rather than a dropped edge.

## Where it lives: light nodes, a rich schema store

Two stores, one identity.

**The graph carries the objects.** A table, view, materialized view, function or
procedure is a node, exactly as the SQL walk already writes it (165 tables, 52
views and 41 routines on this repository's index). Table-level relationships are
edges: a foreign key is `references`, a view reading a table and a routine
calling a routine come from dbd's `deps`. That is what lets callers, impact and
search answer "what code touches `orders`" across code and schema.

**The schema store carries the detail.** Columns, indexes and column-level
foreign keys are not nodes. A column node would multiply the SQL half of the
graph roughly tenfold (1,291 columns over 121 tables in the measured corpus) and
force an embedding decision per column; detail in `nodes.props` would bloat every
node read to serve one screen. Both were the first draft; both are rejected.

### The node vocabulary

The two-level vocabulary is unchanged: the **generic** kind in `node_kind`, the
**specific word** in `declared_type`.

| object | `SymbolKind` | `declared_type` |
|---|---|---|
| table | `Struct` | `table` |
| view | `Struct` | `view` |
| materialized view | `Struct` | `materialized_view` |
| function | `Function` | `function` |
| procedure | `Function` | `procedure` |

`declared_type` is a `text` column but a **closed set** of these five words. A
producer inventing a sixth silently creates a category no reader handles.
Columns and indexes write no node, so `field` and `property` are not written by
this work. **Nothing in `node_kind` or `edge_kind` changes.** `edge_confidence`
is `extracted`: a fact dbd read, not inferred or ambiguous.

dbd's `deps` map onto `RefKind`, which already has every kind needed:
`reads` → `Reads`, `writes` → `Writes`, `calls` → `Calls`, `member` → `TypeUse`.

### `resolved_via` needs one new rung, and this is why

The eight existing rungs all answer the same question — *what, in this file,
told us where the target lives* — and a schema fact answers a different one.
dbd's `SchemaModel` arrives already resolved, schema-qualified, with no file
behind it at all. None of `DeclaredHere`, `ThroughAnImport` or the rest is true
of it.

Reusing the closest one would make the audit trail lie. §4's whole claim is that
"every edge can be audited back to its reason", so an edge stamped
`DeclaredHere` when no file declared anything is worse than an unlabelled edge:
it is a *wrong* reason, which R4 ranks below no reason.

```rust
/// The schema itself states the relationship — a dbd `SchemaModel` ref, a
/// declarative schema file. No source file was read to place it.
///
/// ABOVE every other rung. The one rule this spec rests on is that the
/// database's own name is the identity and every other producer defers to
/// it; a rung ordered below the source-code rungs would contradict that at
/// exactly the point the two disagree.
StatedBySchema,
```

**`unresolved_reason` needs nothing new, but it does need a stated mapping.**
dbd's `DepEdge.unresolved` means the endpoint is not placeable — the same
doctrine as `Resolution::Unresolved`. That maps to **`ExternalBoundary`**: the
reference names something outside what this scan indexes, which is precisely
what that reason was measured into existence for. It is deliberately *not*
`NoImportInScope` (a source-language notion with no meaning in SQL) and not
`Unplaced` (which asserts the ladder has not run yet, and must be empty once it
has).

**Open, and it belongs to step 2:** an ORM-sourced mapping whose table name came
from convention rather than a stated name is a *weaker* claim than
`StatedBySchema`, so it wants its own rung below the source-code ones rather
than sharing this one. It is left out here because nothing emits it until
step 4, and a rung with no writer is a vocabulary entry that cannot be audited.

## The schema store

Four tables in the `sensei` schema, each keyed to the node it describes, with
`ON DELETE CASCADE` so a node removed by a re-index takes its detail with it.

| table | one row per | columns |
|---|---|---|
| `schema_entities` | schema object node | `node_id` (PK → `nodes.id`), `kind` (the closed set above), `db_schema`, `source`, `naming`, `note`, `note_md` |
| `schema_columns` | column | `node_id` (→ `schema_entities`), `ordinal`, `name`, `sql_type` (verbatim), `primary_key`, `not_null`, `unique_key`, `foreign_key`, `enum_typed`, `default_expr`, `note`; unique (`node_id`, `name`) |
| `schema_indexes` | index | `node_id`, `name` (nullable — dbd's may be unnamed), `definition` (`(status, placed_at DESC)` — an expression, so text), `unique_index` |
| `schema_refs` | column pair of a foreign key | `from_node_id`, `from_column`, `to_node_id`, `to_column`, `on_delete` (`cascade` / `restrict` / `set_null` / `set_default` / `no_action`, or NULL when unstated) |

The fields are dbd `SchemaModel` v2's, renamed only where a SQL keyword
collides: `Column {ty, pk, nn, en, def, fk, uq, note}`, `Index {def, unique,
name}`, `Ref {from {s,t,c}, to {s,t,c}, action}`. A composite foreign key is one
row per column pair, which is how v2 carries it.

`source` and `naming` are enums (`ddl/enum/sensei/`), not free text:

- `source`: `dbd`, `ef_core`, `dapper`, `sqlalchemy`, `django_orm`, `jpa`,
  `prisma`, `drizzle`, `typeorm`, `active_record`, `gorm`, `other`. `other`
  keeps the detected dependency name in `note`, because dropping an
  unenumerated ORM's real mapping to NULL would make it indistinguishable from
  "no ORM here".
- `naming`: `stated` (the source wrote the name — DDL, `.ToTable("orders")`)
  or `convention` (a `DbSet<Order> Orders`, an ActiveRecord class name). Never
  promoted to `stated` by agreement with another producer: when dbd also
  declares the table, dbd's row IS the table and the ORM's is a mapping.

**The ER screen reads the store, not the graph.** An endpoint assembles dbd's
`SchemaModel` shape — `tables` (entities + columns + indexes), `refs`,
`entities`, `deps` — from these tables and the graph's dependency edges, and
rokkit's `toGraphInput` takes it from there. The wire shape stays dbd's contract,
so dbd's viewer and sensei's screen read the same thing.

## Routing: the indexer's output must reach the store

**Today it cannot.** The walk's per-file product is
`facts::FileFacts { symbols, references, relations, imports }`, and
`persist::write` turns exactly those into rows. There is no slot for schema
detail, so a producer that extracted columns would have them dropped before
persistence and only nodes and edges would land. The routing is therefore part
of this work, not a follow-up:

```rust
/// Schema detail for one object the same output declares as a node. Keyed by
/// the node's fqn, so the writer resolves it to the node id in the same
/// transaction that wrote the node.
pub struct SchemaFact {
    pub fqn: Fqn,
    pub source: SchemaSource,
    pub naming: Naming,
    pub kind: SchemaKind,          // table | view | materialized_view | function | procedure
    pub db_schema: String,
    pub note: Option<String>,
    pub note_md: Option<String>,
    pub columns: Vec<SchemaColumn>,
    pub indexes: Vec<SchemaIndex>,
    pub refs: Vec<SchemaRef>,      // outgoing foreign keys, column pairs
}
```

- `FileFacts` gains `schema: Vec<SchemaFact>` — an ORM extractor is a file walk
  and returns it beside its symbols.
- The dbd importer is not a file walk (`SchemaModel` has no file attribution),
  so it produces a `SchemaBatch { package, facts: Vec<SchemaFact> }` for a
  dbd-managed folder. **Both go through one writer**, `persist::write_schema`, so
  there is one rule for how detail lands.
- **Nodes first, then detail, one transaction.** A fact whose fqn has no node
  is written as the node (the importer may see an object the walk did not), then
  its detail. The walk's span wins over the importer's absent one: an upsert by
  fqn never erases a location another producer recorded.
- **Replace, don't merge.** Re-indexing an object deletes and rewrites its
  columns, indexes and refs in that transaction, so a column dropped from the
  DDL disappears rather than lingering.
- **`persist::read_back` reads the store back**, so the existing round-trip
  properties (`no_fact_is_lost_…`, `every_…_survives_the_round_trip`) cover
  schema facts. A writer that skipped the store must FAIL those tests, not pass
  them by omission.

Typed rather than free JSON for the reason `DeclaredType` is an enum: a typed
producer cannot emit `primary_key: "yes"`, and every consumer reads one spelling.

## Embedding

Tables, views and routines are nodes and embed as nodes do. Columns and indexes
are not nodes, so the per-column embedding question the first draft raised does
not arise. A table's embedding text may include its column names and notes —
one vector per table, carrying what a column name alone cannot.

## ORM layers: what is possible now, and what is not

Detection needs no new mechanism. `ManifestAdapter::stack_labels(content)` is
already content-derived and already does this inference (npm reads a `svelte`
dependency; dbd reads `source.dialect`), and adapters exist for every relevant
ecosystem.

Extraction splits three ways, and the split is measured rather than assumed
(counts from the private corpus, `SENSEI_CORPUS`):

### Possible today, no new fact

**The entity inventory, by convention.** The C# walk records `declared_type` for
properties, so `DbSet<Order> Orders` yields `Stated("DbSet<Order>")`. With the
`: DbContext` base-class relation the walk already emits, that gives the entity
set with `Naming::Convention`. Measured: 71 files declare a `DbSet<…>`, 36 declare
`: DbContext`.

**The deviation surface**, which is the "partial usage" question stated as a
measurement. `FromSqlRaw` / `ExecuteSqlRaw` / `FromSqlInterpolated` are ordinary
calls and are already edges: 139 files. Against 36 `DbContext`s, that says this
codebase is substantially not ORM-mediated — and it is answerable by a query over
edges the graph already holds, with no new extraction at all.

At project level the same shape answers "full or partial": 34 of 58 `.csproj`
depend on EF Core and 4 on Dapper, so mixed data access is a fact about the
codebase rather than a hypothesis.

### Needs one new fact

**The explicit names.** `.ToTable("orders")` and `.HasColumnName("order_no")` are
method calls whose argument is a string literal, and `Symbol` has no field for a
call argument. That fact — *literal arguments at a call site* — is what upgrades
`Naming::Convention` to `Naming::Stated`. It does not gate the inventory.

This corrects an earlier reading. The attribute path (`[Table("…")]`) looked like
the blocker because C# filters `attribute_list` out of the walk entirely, but the
corpus says attributes are barely used: `[Table(` in 2 files, `[Column(` in **0**,
against `.ToTable(` in 308 and `.HasColumnName(` in 229. **EF Core here is fluent,
so the blocker is call-argument literals, not annotations.**

### The cheap alternative: declarative schema files

Prisma's `schema.prisma` and Rails' `db/schema.rb` state the schema in a file —
manifest-shaped, like dbd's `design.yaml`, fully qualified, no walk change at all.
Migrations are the weakest source in any ecosystem: they are a *history*, so
current state means replaying them, and a partially-applied history yields a
schema that exists nowhere.

## Sequence (#251 — each step its own commit, TDD, mutation-probed)

1. This spec.
2. DDL: the four tables + the two enums, with grants/RLS per schema
   conventions; `dbd inspect`, the SQL planner gate.
3. `SchemaFact` + `FileFacts.schema` + `persist::write_schema` / `read_back`,
   with the round-trip property extended.
4. The dbd importer: `dbd_core::schema_model::build(&Design, scope)` for a
   dbd-managed folder → `SchemaBatch`. This repository's `database/` is the
   first corpus.
5. The read endpoint assembling `SchemaModel` v2 from the store.
6. #221's screen: `ErDiagram` (ER scope) + `EntitiesView` / `EntityView`.
7. EF Core extractor by convention, then the next ORM by corpus count.
   Call-argument literals (`.ToTable`) as their own spec, upgrading
   `convention` to `stated`.

## Done gate

- Indexing this repository's `database/` fills `schema_columns` with exactly the
  columns dbd's own model reports, `schema_refs` with its foreign keys, and adds
  **zero** column or index nodes.
- Dropping a column from the DDL and re-indexing removes exactly that row.
- Mutating `persist::write` to skip the store fails the round-trip tests.
- The ER screen renders this repository's schema with columns, keys and FK lines.

## Not in scope

- **The viewer.** dbd 0.22's notes describe the diagram viewer being extracted
  into a package dbd, sensei and Rokkit share. This spec ends at the facts; the
  rendering is that package's.
- **Replacing the walk for dbd-managed repos.** The model has no spans and no file
  attribution, so it cannot answer "where is this declared". Both producers run.
