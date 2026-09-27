# Entity graph: what a database schema can contribute to the code graph

Research only. No code changed. Measured against this repository's own schema and
adapters on 2026-09-27; every number below is reproducible by the command beside
it.

Three questions were asked:

1. What would the node model need to carry dbd's entity diagram (tables,
   columns, indexes, comments, foreign keys)?
2. Can the same information be extracted from ORM/abstraction layers —
   SQLAlchemy, Entity Framework, and so on — with the manifest scan deciding
   which is in use?
3. How is duplication avoided when two producers describe one table?

The short answers: (1) less than expected — the pattern already exists and the
`node_kind` enum does **not** need extending; (2) **detection is trivial,
extraction is not** — the walks discard exactly the tokens an ORM puts its table
and column names in, and that is a new fact type rather than a new adapter; (3)
by the fqn, which means the schema's own name must be the anchor and the ORM must
defer to it.

---

## 1. What `dbd diagram --json` actually contains

```
cd database && dbd diagram --json -f schema.json
```

Four top-level keys. Measured on this repo:

| key | count | shape |
|---|---|---|
| `project` | — | `{name, …}` |
| `schemas` | 7 | `{name, tables, enums}` |
| `tables` | 121 | `{schema, name, kind, note, noteMd, columns, indexes}` |
| `refs` | 106 | `{from:{s,t,c}, to:{s,t,c}, action}` |

- **columns — 1,291 of them**: `{name, type, pk, nn, def, en, note}`
- **indexes**: `{name, def, unique}`
- **refs**: foreign keys, **both ends fully schema-qualified down to the column**

### What it does not contain, and it matters

`kind` takes exactly one value across all 121 entries: `table`. Against the 280
entities dbd applies from this same tree, the diagram omits:

| absent from the diagram | count here |
|---|---|
| views + materialized views | 46 |
| procedures | 21 |
| functions | 14 |
| enums (counted in `schemas`, not listed) | 107 |
| sequences | 3 |

It also carries **no spans** (no line numbers, so nothing to jump to) and **no
file attribution** (nothing maps a table back to the `.ddl` that declares it).

**So the diagram cannot replace the per-file walk.** The walk produces 695
read/write/call references from view and routine bodies; the diagram has 106
foreign keys. Those are different populations, not a subset relationship.

### The part that IS better than a resolver

The ambiguous references — a bare `orders` needing a search path — occur **only
inside routine and view bodies**, which the diagram does not contain. So the
diagram never held the problem. What it holds is the **complete
schema-qualified inventory**, which is exactly the lookup a resolver needs to
answer "does `sensei.orders` or `extensions.orders` exist?"

For a dbd-managed repo that inventory is available **before any file is walked**.
A bare name can then be resolved during the per-file pass, and the post-scan
barrier is not needed — its cost existed only because the inventory was assumed
to arrive late.

---

## 2. What the node model needs

### The `node_kind` enum does not need extending

It has no `table`, `column`, `index`, `view` or `schema` variant, and that is
fine, because the SQL adapter already resolves this with a two-level vocabulary
(`lang/sql/facts.rs:71`):

```rust
EntityType::Table            => (SymbolKind::Struct,   "table")
EntityType::View             => (SymbolKind::Struct,   "view")
EntityType::MaterializedView => (SymbolKind::Struct,   "materialized_view")
EntityType::Procedure        => (SymbolKind::Function, "procedure")
EntityType::Sequence         => (SymbolKind::Static,   "sequence")
```

The **generic** kind goes in `node_kind` so cross-language queries work; the
**specific word** goes in `declared_type` as `DeclaredType::Stated(label)`. A
column is therefore `(SymbolKind::Field, "column")` and an index
`(SymbolKind::Property, "index")` — additive, no enum migration, no DDL change to
`node_kind` at all.

This is worth stating plainly because the opposite is the tempting move: adding
`table`/`column` to the enum would fork the vocabulary every other language
already shares, and every consumer switching on `node_kind` would need to learn
two spellings for one idea.

### What nodes already has that fits

| need | column | note |
|---|---|---|
| column → table containment | `parent_id` | already self-referencing |
| table/column comment | `docstring` | dbd's `note` / `noteMd` |
| column type | `signature` or `declared_type` | `type` is `text` in the model |
| schema | the fqn's module segment | SQL mints `Form::Item { module: schema }` |

### What genuinely has no home

| fact | why props is the answer |
|---|---|
| `pk`, `nn`, `unique` | booleans meaningful only for a column/index |
| `def` (default expression) | free text, one kind of node only |
| index `def` — `(session_id, created_at DESC)` | an expression, not a name |
| `action` on a ref (`cascade`) | an edge attribute for one edge kind |

These belong in `props`, which is what it is for: attributes that exist for one
kind and would be NULL on 99% of rows as columns. That matches the decision on
the verdict columns from the opposite direction — `resolved_via` is scalar and
universal, so it earns a column; `pk` is neither.

### `edge_kind` and foreign keys

`edge_kind` is `calls, implements, extends, imports, depends_on, traces_to,
references, covers, rationale_for, duplicates, similar_to`. A foreign key is a
`references` edge; the `action` (`cascade`) goes in props. **No new edge kind
needed.**

### Cost, stated up front

1,291 columns for 121 tables in *this* repo — roughly 10 column nodes per table.
Indexing columns is therefore an order-of-magnitude increase in node count for
the SQL half, and `nodes` already carries an `embedding vector(384)` column. A
decision is needed on whether column nodes are embedded; if they are, the storage
and the embed-task cost scale with that 10×.

---

## 3. ORM extraction: detection is easy, extraction is blocked

### Detection — already solved, no new mechanism

`ManifestAdapter::stack_labels(&self, content) -> Vec<&'static str>`
(`adapters/manifest.rs:134`) is content-derived and already does exactly this
shape of inference:

- `npm.rs` — a dependency named `svelte` means a Svelte project
- `dbd.rs` — `source.dialect` chooses `["sql", "postgresql"]` vs `["sql", "tsql"]`

So detecting an ORM is a dependency-name check in an adapter that already parses
dependencies: `sqlalchemy` / `django` in pyproject, `Microsoft.EntityFrameworkCore`
in a csproj, `prisma` / `drizzle-orm` / `typeorm` in package.json,
`hibernate-core` / `spring-boot-starter-data-jpa` in pom.xml, `doctrine/orm` in
composer.json, `gorm.io/gorm` in go.mod, `activerecord` in a Gemfile.

Adapters exist for every one of those ecosystems: `pyproject, dotnet, npm, maven,
gradle, composer, go, ruby, cargo, swiftpm, cmake, dbd`.

### Extraction — the walks discard the tokens that carry the answer

This is the blocking finding, and it is a property of `Symbol`, not of any one
language:

```rust
pub struct Symbol { fqn, kind, name, span, visibility, docstring, declared_type, params }
```

There is **no field for an initialiser value and none for an annotation
argument**. Confirmed per language:

| language | what the walk does with the metadata | source |
|---|---|---|
| Java | annotations ARE walked, as **use sites** — a reference to the annotation type | `java/walk.rs:421 self.annotations(...)` |
| C# | `attribute_list` is **explicitly filtered out** | `csharp/walk.rs:575,719` |
| Python | a decorator is a use site; "the definition is what declares a name" | `python/walk.rs:261` |

So we can already record *that* a class is annotated `@Entity`, as an edge to
`Entity`. We cannot read `@Table(name = "orders")`, `[Table("orders")]`, or
`__tablename__ = "orders"` — the string literal is an annotation argument or a
field initialiser, and neither survives the walk.

Every code-first ORM puts its table and column names in exactly those positions:

| framework | table name | column name |
|---|---|---|
| SQLAlchemy | `__tablename__ = "orders"` (initialiser) | `mapped_column("order_no", …)` (argument) |
| Django | derived from class + app label, or `class Meta: db_table` | field kwargs |
| EF Core | `[Table("orders")]` or fluent `ToTable("orders")` | `[Column("order_no")]` |
| JPA/Hibernate | `@Table(name="orders")` | `@Column(name="order_no")` |
| Doctrine | `#[ORM\Table(name: "orders")]` | `#[ORM\Column(name: …)]` |
| GORM | `TableName()` method returning a literal | struct tag `gorm:"column:order_no"` |
| ActiveRecord | convention from class name | convention from DB |

**Conclusion: code-first ORM extraction needs a new fact — annotation/decorator
arguments and field initialiser literals — before any of it is possible.** That
is a change to `Symbol` and to every walk, which is a much larger piece of work
than an adapter, and it should not be started on the assumption that detection
being easy makes extraction easy.

What *is* derivable today without that fact: the class, its fields, their
declared types, and the type references between them — i.e. the **shape** of the
entity graph under convention-based naming, with none of the explicit names. For
Django and ActiveRecord, where naming is convention by default, that is most of
the model. For SQLAlchemy and JPA, where the name is usually written out, it is
the part that matters least.

### The much cheaper class: declarative schema files

Several ecosystems state the schema in a **file** rather than in annotations, and
those need no walk change at all — they are manifest-shaped, exactly like dbd's
`design.yaml`:

| framework | file | format |
|---|---|---|
| Prisma | `schema.prisma` | its own DSL, fully explicit |
| Drizzle | `schema.ts` | TypeScript, but declarative |
| Rails | `db/schema.rb` | generated, fully explicit |
| dbd | `design.yaml` + `ddl/` | already done |
| Alembic / EF / Django migrations | `migrations/*` | code, and a *history* rather than a state |

Prisma and Rails `schema.rb` are the strongest candidates after dbd: one file,
fully qualified, no inference. Migrations are the weakest — they describe a
sequence of changes, so deriving current state means replaying them, and a
partially-applied history gives a schema that does not exist anywhere.

---

## 4. Duplication: the fqn decides, so the schema must be the anchor

`nodes_unique_fqn` makes the fqn the lookup key — "two declarations that mint one
fqn are one node" (`persist.rs`). So duplication is not avoided by de-duplicating
after the fact; it is avoided by **both producers minting the same fqn**, or it is
not avoided at all.

For a repo with both a dbd schema and an ORM describing it, that means:

- The **table's identity is the database's**: `Form::Item { lang: Sql, package,
  module: schema, name: table }`. An ORM producer must mint that same fqn for the
  table it maps to, or there will be two nodes for one table.
- The **model class keeps its own identity** in its own language, and the
  relationship between them is an **edge** — the class `references` (or a new
  `maps_to`) the table. That is the honest shape: a SQLAlchemy `Order` class and
  a `sensei.orders` table are two different things that correspond, not one thing
  described twice.
- Which means an ORM producer that cannot determine the schema-qualified table
  name **must not guess one**. A guessed `public.orders` against an actual
  `sensei.orders` is a wrong edge, and R4 ranks that below no edge. This is the
  same rule that made the SQL walk's first-schema fallback wrong.

That ordering has a consequence worth stating: where a dbd schema exists it is
the authority, and the ORM layer adds the mapping edge. Where no schema exists,
the ORM is the only producer and its entities are honest nodes of the ORM's own
language — not fabricated database tables.

---

## 5. The sidebar being optional falls out for free

The proposal that entity info/diagram is a sidebar excluded when a project has no
database needs no new signal: `stack_labels` already yields `sql` for a
dbd-managed folder, and it is already persisted — `folders.stack` is a
`jsonb not null default '[]'` array set by `ProcessGitFolder`
(`ddl/table/sensei/folders.ddl:13`), so the query is
`folders.stack ? 'sql'`. A project with no
database produces no entity nodes, so the panel has nothing to render and can be
hidden on an empty result rather than on a feature flag.

---

## 6. What this suggests, in dependency order

1. **The verdict columns** (`resolved_via`, `unresolved_reason` as real columns
   written by persist). Independent of all of this, and it is what makes every
   claim below measurable rather than asserted.
2. **The SQL walk consults the schema inventory** so a bare name resolves
   in-pass. Uses the dbd design load that is already a dependency; no barrier.
3. **Tables, columns, indexes and foreign keys as nodes/edges** from dbd, using
   the existing two-level kind vocabulary and props for the column-only
   attributes. Decide the column-embedding question first — it is a 10× node
   multiplier.
4. **Prisma and `schema.rb` adapters**, if a second declarative source is wanted.
   Same shape as dbd, no walk changes.
5. **Annotation arguments and initialiser literals as a fact.** The prerequisite
   for every code-first ORM, and the only item here that touches `Symbol` and
   every walk. Worth its own spec.

Items 1–4 are additive. Item 5 is the one that should not be started casually,
and nothing in 1–4 depends on it.
