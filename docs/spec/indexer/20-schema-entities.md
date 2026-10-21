---
name: Schema entities
description: Database structure — tables, views and routines as graph nodes, columns/indexes/foreign keys in a schema store keyed by them — from dbd's SchemaModel v2 and from ORM layers
date: 2026-09-27
revised: 2026-10-08
status: approved (#251)
---

> **Revised 2026-10-08 (#251, decision in [`docs/history/plan/decisions.md`](../../history/plan/decisions.md)).**
> The first draft made every column a `field` node and stored schema detail in
> `nodes.props`. Both are gone: tables, views and routines stay nodes, and
> columns, indexes and column-level foreign keys live in four `schema_*` tables
> keyed by `node_id`. Nodes stay light; the detail is as rich as dbd's model.

# Schema entities

A database is structure a reader navigates, and today the indexer records almost
none of it. This states how a table, a column, an index and a foreign key become
graph facts, where the source is dbd's `SchemaModel` v2 or an ORM layer, and what
is deliberately not attempted yet.

The measurements behind it came from an entity-graph feasibility analysis
(deleted 2026-10-21; it lives in git history).

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
  so it hands `persist::write_schema(store, folder, &[SchemaFact])` the facts for
  a dbd-managed folder. `persist::write` routes `FileFacts.schema` through the
  same body. **One writer**, so there is one rule for how detail lands.
- **Nodes first, then detail.** A fact whose fqn has no node is written as the
  node through `node_for` (the importer may see an object the walk did not) —
  an idempotent upsert, outside the transaction. The DETAIL of the whole batch
  is one transaction (`PgStore::replace_schema_detail`): entities, columns and
  indexes, then refs, so a key between two objects of one batch never depends
  on their order. A key whose target the store does not describe is returned
  as an `UnplacedRef`, never dropped in silence. The walk's span wins over the importer's absent one: an upsert by
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
4. The dbd importer: `dbd_core::schema_model::build(&Design, None)` for each
   `design.yaml` → `Vec<SchemaFact>` → `persist::write_schema`. Run as the
   `ImportDbdSchema` task, behind the repo's file tasks (so the detail lands on
   walked nodes) and ahead of the `DetectCommunities` barrier. Fqns come from
   the SQL walker's own `object_identity`, with the package the manifest
   states. dbd's `trigger` kind has no `schema_object_kind`; triggers are
   reported as skipped, and any other unknown kind or key action is an error.
   This repository's `database/` is the first corpus.
5. The read endpoint assembling `SchemaModel` v2 from the store.
6. #221's screen: `ErDiagram` (ER scope) + `EntitiesView` / `EntityView`.
7. EF Core, as a GRAPH DERIVATION — see "EF Core: a derivation, not a walk"
   below. 7a this section; 7b the call-site fact; 7c `DeriveOrmSchema`; 7d live
   verification on one C# repository. Then the next ORM by corpus count.

## EF Core: a derivation, not a walk (step 7, revised 2026-10-12)

Step 7 first read "EF Core extractor by convention": a file walk filling
`FileFacts.schema` from `DbSet<Order> Orders`. Measured over the C# corpus, that
would be wrong where it matters:

| shape | files |
|---|---|
| `DbSet<…>` | 71 |
| `: DbContext` | 36 |
| `.ToTable(` | 308 |
| `HasDefaultSchema(` | 40 |
| `.csproj` on `EntityFrameworkCore.SqlServer` / `Npgsql` | 13 / 2 |

- **A convention name is wrong wherever fluent config renames the table**, and
  `.ToTable(` is in four times as many files as `DbSet<`. `naming = convention`
  would label a wrong row, not a weak one.
- **The columns are another file's.** They are the entity class's properties;
  the `DbSet` names only the class. The config usually lives in an
  `IEntityTypeConfiguration<T>` in yet another file.
- **The schema is not in the file.** `HasDefaultSchema` is a call literal, and
  the provider default (`dbo` / `public`) is a `.csproj` dependency.

A walk sees one file (the indexer's first rule). So EF Core is a per-repo
DERIVATION over what the walks already wrote, run after the file tasks exactly as
the dbd import is — and it needs one fact the walks do not yet record.

### 7b · the call-site fact

`FileFacts.calls: Vec<CallSite>` — language-neutral, emitted first by the C#
walk. It knows nothing about EF:

```rust
pub struct CallSite {
    pub from: Fqn,              // the enclosing declaration
    pub at: Span,               // the invocation, as the `Calls` reference spans it
    pub callee: String,         // the method name as written, without type arguments
    pub type_args: Vec<String>, // `Entity<Order>()` → ["Order"]
    pub receiver: Receiver,     // Call(span) | Name(String) | Other | None
    pub args: Vec<CallArg>,     // positional
}
pub enum CallArg {
    Str(String),                                // a string literal, unescaped
    Lambda { params: Vec<String>, member: Option<String>, at: Span }, // `x => x.Name` → member "Name"
    Other,                                      // anything else; kept so positions hold
}
```

Recorded only for a call with at least one `Str` or `Lambda` argument or a type
argument — the shapes a derivation can read; a bare `Foo(x)` carries nothing.
Persisted in `sensei.call_sites`, keyed by `(folder_id, file_path)` and REPLACED
per file (the fact belongs to the file that states it, unlike a node), with
`read_back` round-tripping it.

### 7c · `DeriveOrmSchema`

A per-repo task beside `ImportDbdSchema` (blocked on the file tasks, blocking
the barrier). For each class that extends `DbContext` (an `Extends` relation,
resolved or not — `DbContext` is external):

- **Entities**: the `T` of each `DbSet<T>` property (its `declared_type`), each
  `Entity<T>()` call's type argument, and each `IEntityTypeConfiguration<T>`.
- **Which entity a call configures**: walk the receiver chain to an
  `Entity<T>()` call; or a `Name` receiver bound by a lambda parameter of an
  enclosing `Entity<T>(b => …)` call (by span containment); or a `Name` that is
  a parameter typed `EntityTypeBuilder<T>` of the enclosing declaration.
- **Table**: `ToTable("t"[, "schema"])` → `stated`; else the `DbSet` property's
  name → `convention`.
- **Schema**: `ToTable`'s second literal; else the context's
  `HasDefaultSchema("s")`; else the provider default read from the `.csproj`
  (`dbo` for SQL Server, `public` for Npgsql). **No provider and no statement →
  the entity is SKIPPED and counted**, never filed under a guessed schema.
- **Columns**: the entity class's properties, in declaration order, minus
  navigations (a property typed as another entity, or a collection of one).
  `sql_type` is the CLR type verbatim — the store's column says *declared type*,
  and here the declaration is C#. `HasColumnName("c")` on `Property(x => x.P)`
  renames P. `not_null` only where the type says so (a non-nullable value type);
  a reference type's nullability depends on a project setting no file states,
  so it is left false rather than guessed. Primary key: `HasKey(x => x.P)`, else
  EF's convention `Id` / `<T>Id`.
- **Keys**: a reference navigation `Customer Customer` with a `CustomerId`
  property → a ref to the `Customer` entity's key.
- **Identity**: `sql·<package of the DbContext>·<schema>·<table>·item`. An EF
  table and a dbd table of one database therefore do not meet — they have
  different packages — which is correct until something states they are one
  database.

Source `ef_core`; naming per table as above.

## JPA: the next ORM, also a derivation (step 8, #254)

### 8a · why JPA, and what its corpus says

Counted per repository over the private corpus, then decomposed (spec 18, rules
1–2). JPA leads by an order of magnitude: 529 files declare `@Entity` across 6
repositories, but three of them are copies of one codebase (107–108 shared entity
class names), so the corpus holds **200 distinct entities** in about three
independent codebases. Sequelize has 39 model definitions and SQLAlchemy 4
declarative models; Dapper declares no schema at all.

| shape | files | occurrences |
|---|---|---|
| `@Entity` | 529 | 529 |
| `@Table(name = …)` | 506 | 506 |
| `@Table(schema = …)` | 0 | 0 |
| `@Column(name = …)` | 524 | 4,817 of 4,839 `@Column` |
| `@JoinColumn(…)` | 235 | 531 |
| `@ManyToOne` / `@OneToOne` | 267 | 526 |
| `@OneToMany` / `@ManyToMany` | 77 | 153 |
| `@JoinTable` | 27 | 40 |
| `@Id` | 512 | 512 |
| `@EmbeddedId` / `@IdClass` | 2 | 2 |
| `@MappedSuperclass` / `@Embeddable` | 6 / 6 | |
| `orm.xml` | 0 | |

Two things decide the design:

- **Names are stated, almost everywhere** — and then rewritten. Spring Boot's
  default physical naming strategy (`CamelCaseToUnderscoresNamingStrategy`)
  snake-cases even an explicit `@Table(name = "OrderLine")`. Of 5,850 explicit
  names, 124 are camelCase. The name the database holds is the strategy's output.
- **The schema is never stated in a file that declares an entity.** No
  `@Table(schema = …)`, no `hibernate.default_schema`. Four of the six JPA
  repositories build against `mysql-connector`, where a schema IS a database —
  the one named in `spring.datasource.url` (`jdbc:mysql://host/<db>`).

As with EF Core, a walk sees one file, and the table, its columns, its keys and
its schema live in different files. JPA is a per-repo derivation.

### 8b · the annotation fact

The Java walk records each annotation as a `Decorates` relation and walks its
argument expressions — but keeps no literal. `@Table(name = "orders")` carries
`"orders"` nowhere. EF Core's equivalent gap was closed by `CallSite` (7b); an
annotation is a different shape — it is applied TO a declaration rather than
made FROM one, and its arguments are named — so it gets its own fact:

```rust
pub struct Annotation {
    pub target: Fqn,              // the declaration the annotation is on
    pub at: Span,
    pub name: String,             // as written: `Table`, `jakarta.persistence.Table`
    pub args: Vec<AnnotationArg>, // in source order
}
pub struct AnnotationArg {
    pub name: Option<String>,     // `name = "orders"` → Some("name"); `@Table("orders")` → None
    pub value: CallArg,           // Str for a string literal; Bool/Other as below
}
```

`CallArg` gains `Bool(bool)` — `nullable = false` is the one non-string literal a
schema needs. Recorded only for an annotation with at least one `Str` or `Bool`
argument; a marker annotation is already its `Decorates` relation. Persisted in
`sensei.annotations`, keyed by `(folder_id, file_path)`, REPLACED per file,
round-tripped by `read_back` — the call-site rules exactly.

### 8c · `DeriveOrmSchema` for JPA

The same per-repo task, now also enqueued for a repository with a `pom.xml` or a
`build.gradle[.kts]`. For each class `Decorates`'d by `Entity` whose file imports
`javax.persistence` or `jakarta.persistence`:

- **Table**: `@Table(name = "t")` → `stated`; else `@Entity(name = "n")`; else the
  class name → `convention`. Then the physical strategy: in a Spring Boot
  repository (`spring-boot-starter-data-jpa` in its build) with no
  `spring.jpa.hibernate.naming.physical-strategy` set, snake-case it; a repository
  that sets the property to `PhysicalNamingStrategyStandardImpl` keeps the name
  verbatim; **any other strategy is code — the entity is SKIPPED and counted.**
- **Schema**: `@Table(schema = …)`; else `hibernate.default_schema` /
  `spring.jpa.properties.hibernate.default_schema`; else the driver's: MySQL /
  MariaDB → the database in `spring.datasource.url`, PostgreSQL → `public`,
  SQL Server → `dbo`. A URL with a placeholder (`${DB_NAME}`) states nothing.
  **No statement and no driver default → SKIPPED and counted.** H2 is a test
  database and never chooses the schema.
- **Columns**: the entity's fields in declaration order, then those of each
  `@MappedSuperclass` it extends (an `Extends` relation, transitively). Skip
  `static` fields, `@Transient`, and `@OneToMany` / `@ManyToMany`, which are the
  other table's columns. `@Column(name = "c")` → stated, else the field name, both
  through the physical strategy. `sql_type` is the Java type verbatim, as EF's is
  the CLR type. `not_null`: `@Column(nullable = false)`, a primitive type, or
  `@Id` — never inferred from a wrapper type.
- **Primary key**: the `@Id` fields. `@EmbeddedId` / `@IdClass` (2 files) →
  SKIPPED and counted.
- **Keys**: `@ManyToOne` / owning `@OneToOne` → a column named by
  `@JoinColumn(name = …)`, else `<field>_<target pk column>`, referencing the
  target entity's table. A `mappedBy` side owns no column.
- **Not in 8c**: `@JoinTable` (its columns are nested annotations, which 8b's
  fact does not carry) and `@Embedded` / `@Embeddable` flattening. Each is counted
  in `skipped`, so the gap stays visible.
- **Identity**: `sql·<package of the entity's build module>·<schema>·<table>·item`
  — the EF rule, for the same reason.

Source `jpa`; naming per table and column as above.

8d verifies live on one JPA repository from the corpus, checking a sample of
entities by hand against their annotations and the datasource URL.

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
