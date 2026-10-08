---
name: Schema entities
description: Carrying database structure — tables, columns, indexes, dependencies — as first-class graph facts, from dbd's SchemaModel v2 and from ORM layers
date: 2026-09-27
status: draft
---

> **SUPERSEDED IN PART — 2026-10-08, decision recorded in
> [`docs/plan/decisions.md`](../../plan/decisions.md) and tracked in #251.**
> Columns do NOT become `field` nodes, and schema detail does NOT go into
> `nodes.props`. Tables, views and routines stay nodes; columns, indexes,
> column-level foreign keys, source and naming live in `schema_entities`,
> `schema_columns`, `schema_indexes` and `schema_refs`, keyed by `node_id`. The
> sections below on "The node model", "`Symbol::schema`" and the embedding cost
> are being rewritten to that design; the sources (dbd `SchemaModel` v2, ORM
> extractors), the identity rule and the `StatedBySchema` rung still stand.

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

## The node model needs no enum change

The SQL adapter already resolves this with a two-level vocabulary: the **generic**
kind in `node_kind` so cross-language queries work, the **specific word** in
`declared_type`.

| SchemaModel | `SymbolKind` | `declared_type` label |
|---|---|---|
| `TableNode` (kind `table`) | `Struct` | `table` |
| `EntityNode` kind `view` | `Struct` | `view` |
| `EntityNode` kind `materialized_view` | `Struct` | `materialized_view` |
| `EntityNode` kind `function` | `Function` | `function` |
| `EntityNode` kind `procedure` | `Function` | `procedure` |
| `Column` | `Field` | `column` |
| `Index` | `Property` | `index` |

Adding `table`/`column` to `node_kind` would fork a vocabulary every other
language already shares, and every consumer switching on it would have to learn
two spellings for one idea. `parent_id` gives column→table containment;
`docstring` takes `note`/`noteMd`. A foreign key is a `references` edge.

**Nothing in `node_kind` or `edge_kind` changes.**

### The exact values this uses

The shared vocabulary is defined once in
[`17-vocabulary.md`](17-vocabulary.md) — `SymbolKind` (§2), `RelationKind` (§3)
and the `Rung` ladder (§4) — and is not restated here. What follows is only what
a schema object writes, plus the two things that vocabulary does not yet cover.

Of the 25 `node_kind` values, this work writes **four**: `struct` (table, view,
materialized view), `function` (function, procedure), `field` (column),
`property` (index). Of the 11 `edge_kind` values it writes **one**:
`references`, for a foreign key. `edge_confidence` is **`extracted`** — the
enum's comment glosses that as "AST-certain", and a schema fact arrives from a
declarative source rather than an AST, but the distinction `confidence` actually
draws is read-vs-guessed. `inferred` means embeddings and `ambiguous` means
drift; a table that dbd read is neither.

`declared_type` is a `text` column, so nothing in the database constrains it.
This spec nevertheless treats it as a **closed set** — `table`, `view`,
`materialized_view`, `function`, `procedure`, `column`, `index` — because the
whole point of the two-level vocabulary is that a consumer can switch on the
specific word. A producer inventing an eighth spelling silently creates a
category no reader handles.

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

## `Symbol::schema` — the optional prop

What has no home is the per-kind detail: a column's `pk`, an index's `def`, and —
for an ORM — whether the name was *stated* or *inferred by convention*. That is
what this adds.

```rust
/// Database-structure facts for a symbol that IS a schema object, or maps to one.
///
/// `None` for every symbol that has nothing to do with a database, which is the
/// overwhelming majority — so this costs an `Option` discriminant on `Symbol` and
/// nothing else.
pub struct SchemaFacts {
    /// WHO said so. A consumer must be able to tell the schema's own word from an
    /// ORM's reading of it, because they disagree and the schema wins.
    pub source: SchemaSource,
    /// Whether the NAME above is what the source wrote, or what a convention
    /// supplied. Load-bearing: `DbSet<Order> Orders` yields a table name by
    /// convention and `.ToTable("orders")` states one, and a consumer that cannot
    /// tell them apart cannot report a mapping as verified.
    pub naming: Naming,
    /// The per-kind detail. One variant per shape, so a column's flags cannot be
    /// read off an index.
    pub detail: SchemaDetail,
}

pub enum SchemaSource {
    /// dbd's SchemaModel — the schema describing itself.
    Dbd,
    /// An ORM or data-access layer, named. Carried so "this app uses EF Core"
    /// is a fact about nodes rather than a separate inventory that can drift
    /// from them.
    Orm(OrmKind),
}

/// The ORM that described the mapping.
///
/// A CLOSED enum rather than a `String`, for the reason every other vocabulary
/// here is closed: a consumer asking "is this EF Core" must not have to know
/// that the detector writes `EfCore` and someone else wrote `ef-core`. The
/// variants are exactly the ecosystems `ManifestAdapter::stack_labels` can
/// already name from a dependency, so adding one is a detector change and not
/// a guess.
///
/// `Other(String)` exists because the alternative is worse. An ORM nobody
/// enumerated yet is a real mapping that was really read, and dropping it to
/// `None` would make it indistinguishable from "no ORM here" — the same
/// honest-empty-masks-a-failure trap the no-fabrication rule forbids.
pub enum OrmKind {
    /// .NET. `DbSet<T>`, `: DbContext`, `.ToTable(…)`.
    EfCore,
    /// .NET, micro-ORM. Raw SQL in string literals, so it yields the
    /// DEVIATION surface rather than an entity inventory.
    Dapper,
    /// Python. `declarative_base()`, `__tablename__`.
    SqlAlchemy,
    /// Python, Django's built-in. `models.Model`, `class Meta: db_table`.
    DjangoOrm,
    /// JVM. `@Entity`, `@Table(name = …)`.
    Jpa,
    /// TypeScript/JavaScript, schema-file-first — `schema.prisma` states the
    /// whole schema, which is why it needs no walk change at all.
    Prisma,
    /// TypeScript/JavaScript. `pgTable(…)` in ordinary source.
    Drizzle,
    /// TypeScript/JavaScript. `@Entity()` decorators.
    TypeOrm,
    /// Ruby. ActiveRecord, where the table name is pure convention —
    /// `Naming::Convention` by construction unless `self.table_name` states it.
    ActiveRecord,
    /// Go. `gorm:"…"` struct tags.
    Gorm,
    /// Detected from a manifest, not yet enumerated here. Carries the
    /// dependency name that named it.
    Other(String),
}

pub enum Naming {
    /// The source wrote the name: a DDL `create table sensei.orders`, an explicit
    /// `.ToTable("orders")`.
    Stated,
    /// A convention supplied it: a `DbSet<Order> Orders` property, an
    /// ActiveRecord class name. Never promoted to `Stated` by agreement with
    /// another producer — if dbd also declares it, the dbd node IS the table and
    /// this one is the mapping.
    Convention,
}

pub enum SchemaDetail {
    Table,
    View { materialized: bool },
    Routine,
    Column {
        /// `Column.ty` — the declared SQL type, verbatim.
        sql_type: String,
        primary_key: bool,
        not_null: bool,
        unique: bool,
        /// v2's `fk`: this column participates in a foreign key. The EDGE carries
        /// which one; this is the flag a renderer needs without re-scanning refs.
        foreign_key: bool,
        default: Option<String>,
    },
    Index {
        /// `(session_id, created_at DESC)` — an expression, not a name list, so
        /// it is text and not a structured column set.
        definition: String,
        unique: bool,
    },
}
```

### Why a typed struct rather than free jsonb

`props` is where this lands in the database, and it could have gone straight in as
jsonb from the walk. It does not, for the reason `DeclaredType` is an enum rather
than an `Option<String>`: a typed producer cannot emit `primary_key: "yes"`, and
every consumer reads one spelling. The jsonb is the storage, not the contract.

### Why `naming` is not `DeclaredType`

`DeclaredType` is `Stated | Unstated` and describes a **type**, not a mapping. A
column whose SQL type is known but whose *name* came from a convention needs both
facts, and overloading one field for both would make "stated" ambiguous.

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

## Cost to decide before implementing

1,291 columns for 121 tables here — about 10 column nodes per table, an
order-of-magnitude increase for the SQL half. `nodes` carries
`embedding vector(384)`, so **whether column nodes are embedded** decides whether
storage and embed-task cost scale with that multiplier. A column name carries
little semantic signal on its own; the table's comment carries more. Recommended:
tables, views and routines embed; columns and indexes do not.

## Sequence

1. `Symbol::schema` as above, plus persistence into `props` — additive, nothing
   reads it yet.
2. Tables, columns, indexes and foreign keys from `SchemaModel` v2 for a
   dbd-managed folder. Decide the embedding question first.
3. `deps` as references, mapping `unresolved` onto `Resolution::Unresolved`.
4. EF Core inventory by convention, plus the deviation query. No new fact.
5. Call-argument literals as a fact, which upgrades (4) to `Naming::Stated` and
   unlocks SQLAlchemy and JPA. Its own spec; nothing above depends on it.

## Not in scope

- **The viewer.** dbd 0.22's notes describe the diagram viewer being extracted
  into a package dbd, sensei and Rokkit share. This spec ends at the facts; the
  rendering is that package's.
- **Replacing the walk for dbd-managed repos.** The model has no spans and no file
  attribution, so it cannot answer "where is this declared". Both producers run.
