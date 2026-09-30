---
name: Module shape and the second pass
description: Where the indexer's layers belong, a post-parse resolver per language, and the two things a manifest knows that nothing asks it
date: 2026-09-29
status: draft
---

# Module shape and the second pass

Four proposals, in dependency order. Two are mostly relocation of things that
already exist and work; two are genuinely missing and are the ones worth the
effort. Every count here is from the tree or the live graph on 2026-09-29.

## 1. What already exists, so it is not reinvented

| | where | shape |
|---|---|---|
| **Language adapters** | `indexer/lang/` | `LanguageAdapter` trait, 12 adapters, registry, **total over `Language`**, 6 enforcing tests |
| **Manifest adapters** | `adapters/manifest/` | `ManifestAdapter` trait, 15 adapters incl. `dbd.rs` |
| **Config adapters** | `adapters/config/` | `ConfigAdapter`, json/toml/yaml |
| **Facts → rows** | `indexer/persist.rs` | `edge_rows_of`, `node_for`, stub-on-miss |
| **Rows → database** | `db/pg_store/` | SQL, id lookup, upsert-by-fqn |
| **Per-file resolution** | `indexer/resolve.rs` | the rung ladder, pure and order-independent |

`indexer/quality/reachability.rs` is **not** a resolve barrier — it measures
graph quality ("is every declaration reached by an edge?"). It was
`indexer/barrier.rs`, a name that described the mechanism rather than the
question and collided with the word the post-parse pass below wants.

So the adapter semantics are already there on both axes. What is wrong is only
**where they live**: the manifest layer sits in `adapters/`, a sibling of the
indexer rather than a part of it, even though the indexer is its only consumer
of consequence.

## 2. The shape

```
indexer/
  lang/         LanguageAdapter  + one module per language      (exists, stays)
  manifests/    ManifestAdapter  + one module per ecosystem     (moves from adapters/)
  resolve.rs    the per-file ladder                             (exists, stays)
  settle/       ScopeResolver    + one per language             (NEW — §3)
  persist.rs    facts → rows, stub-on-miss                      (exists, stays)
  ...
db/pg_store/    rows → SQL, id lookup, upsert by fqn            (exists, stays)
```

Two notes on what does **not** move.

**SQL stays out of the indexer.** `persist.rs` (what a fact becomes) and
`pg_store` (how a row is written) is already the right seam, and collapsing them
would put schema knowledge inside the walker. The real defect on that axis is
narrower: `db/pg_store/graph.rs` has grown indexer-specific logic — the receiver
chase, `receiver_type_fqn` — that belongs on the indexer's side of the seam. That
is a small extraction, not a layer move.

**`adapters/config/` stays where it is.** A config file is read by the document
route, not by the indexer.

## 3. `ScopeResolver` — the second pass

### Why the ladder cannot do it

`resolve.rs` is deliberately **order-independent**: `World::scanned` holds every
identity minted before this file and is documented as *"deliberately never read
— a rung that consulted it would answer differently depending on which file came
first"* (R6), with a test that hands the ladder the whole corpus and shows the
output does not move.

That rule is right and must stay. It is also exactly why some things cannot be
placed in one pass:

- **dbd leaves a schema dangling.** A reference to an object declared in a file
  not yet read has no target, and the ladder must not guess one.
- A rust `use a::b::C` cannot say whether `a::b` is a module or `a::b::C` an
  item, so `specifier_names_a_module` refuses — correctly, because R4 ranks a
  miss above an edge that is right half the time. **9,025 import edges currently
  carry no verdict for this reason.**
- A receiver whose type is only knowable from another file.

### Why a second pass is sound where a rung is not

A rung reading `scanned` is order-dependent because `scanned` grows as the scan
runs. A pass that runs **after every file in the scope has been read** sees the
complete set whatever order produced it. Order independence is preserved by the
barrier, not by refusing to look.

```rust
/// A second placement pass, after every file in the scope is read.
///
/// Deliberately NOT named `Resolver`: `resolve.rs` owns that word for the
/// per-file ladder, and the two have different guarantees. This one may read
/// the whole scope; that one may not.
pub trait ScopeResolver: Send + Sync {
    fn language(&self) -> Language;

    /// Place what the ladder left open. `scope` holds every declaration the
    /// scan produced — complete, so the answer does not depend on file order.
    ///
    /// Returns a verdict per input, never a filtered list: an input it cannot
    /// place must come back with a REASON, or the caller cannot tell a refusal
    /// from a resolver that silently skipped it (R2).
    fn settle(&self, open: &[Open], scope: &Scope<'_>) -> Vec<Settled>;
}
```

Three constraints that make it honest rather than a second guessing layer:

1. **Total in, total out.** Every `Open` gets a `Settled`, and `Settled` carries
   either a rung or a reason. This is the shape `Resolution` already has.
2. **Its own rungs.** A placement made here was not made by reading the
   importing file, so stamping it `ThroughAnImport` would make the audit trail
   lie. It needs rungs of its own — `SettledByScope` and, for the dbd case,
   `StatedBySchema`, which `20-schema-entities.md` already argues for.
3. **It may not widen.** It places edges the walk already emitted. A second pass
   that could mint new use sites would be a second producer, which is the thing
   this tree spent 8,411 lines getting rid of.

Tracked as #193.

## 4. The manifest says what to parse

### The evidence

Nothing in `ManifestAdapter` answers "which paths beneath me are current source,
and which are a record of the past". Its 14 methods cover names, dependencies,
lockfiles, workspace members, stack labels and `infer_role` — none of them this.

So the scan parses everything it can read, and in this repo that means:

| | files | produced nodes |
|---|---:|---:|
| `ddl/` — current state | 1,809 | 1,807 |
| `migrations/` | 458 | 420 |
| `snapshots/` | 54 | 5 |

**618 SQL identities exist only in migrations** — 10.8% of the 5,710 total. They
are not collisions (0 identities are declared in both, because the package
segment differs), which makes them worse in one specific way: they are
well-formed, they sit beside current schema, and nothing distinguishes a table
that exists from one that was dropped three migrations ago.

A migration is a *change script*, not a declaration of current state. Under a
dbd-managed schema the DDL tree is the state; the migrations are how it got
there, and replaying them is a deployment concern.

### The addition

```rust
/// What a path beneath this manifest IS, so the scan parses state and not
/// history. Default `Source` for every path, which is today's behaviour, so
/// no existing adapter changes by not implementing it.
fn classify_path(&self, rel: &Path) -> PathRole { PathRole::Source }

pub enum PathRole {
    /// Current, declared state. Parse it.
    Source,
    /// A record of a PAST state — dbd `snapshots/`, `migrations/`, a rails
    /// `db/migrate/`. Recorded as a file, never parsed for declarations:
    /// its symbols describe a schema that may no longer exist.
    Historical,
    /// Produced by a build from something else here. Parsing it double-counts
    /// what the source already declared.
    Generated,
    /// Test data, not a declaration about the system.
    Fixture,
}
```

`dbd.rs` implements it from `design.yaml`, which already states where the DDL
tree is — so the classification is READ from the manifest rather than guessed
from a path pattern. That distinction matters: a hardcoded `migrations/` glob
would misfire on a repo that keeps current DDL in a folder of that name.

Everything else keeps its default and nothing changes for it until its adapter
has a reason.

## 5. The manifest knows the contracts

This is the one with reach beyond the indexer.

A manifest is where a package declares what it **publishes** and what it
**consumes**, and those are the same fact from two sides. Today the graph has
251 projects and 194 git folders, and edges never cross a repo boundary — so a
UI calling an API it has a typed client for, in a sibling repo, is two
unconnected graphs.

The material is already on disk: **332 OpenAPI/Swagger documents** and 3 Prisma
schemas in the watched roots.

```rust
/// Contracts this package publishes or consumes.
///
/// A CONTRACT is an interface stated outside any one language: an OpenAPI
/// document, a `.proto`, a GraphQL schema, a Prisma schema, an ORM's mapped
/// entity set. Each is a SHARED IDENTITY two repos can both name — which is
/// what makes a cross-repo edge possible at all.
fn contracts(&self, parsed: &ParsedManifest, root: &Path) -> Vec<Contract>;

pub struct Contract {
    pub kind: ContractKind,   // OpenApi | Proto | GraphQl | Prisma | OrmEntities
    pub side: Side,           // Publishes | Consumes
    /// Where the document is, when it is a file in this repo.
    pub at: Option<PathBuf>,
    /// The package name the other side would depend on, when it is a
    /// generated client. This is the join key.
    pub package: Option<String>,
}
```

What it unlocks, in order of how soon it is real:

1. **Inside one repo** — a route handler and the OpenAPI document that describes
   it. A handler with no operation, or an operation with no handler, is a gap
   the graph can state.
2. **Across repos** — the UI repo consumes `@acme/api-client`, the backend
   publishes the document it was generated from. That is one edge, and it is the
   first edge the graph would have that crosses a repository.
3. **Drift** — a consumer calling an operation the publisher no longer declares.
   This is the answer to "connect the whole set of repos together", and it needs
   1 and 2 first.

`20-schema-entities.md` already covers the ORM half of `OrmEntities` — detection
via `stack_labels`, which is content-derived and already implemented.

**Honest limit.** A contract only links two repos when both are indexed and
something names the join. A published OpenAPI document with no consumer in the
scan is an inventory entry, not an edge, and must be reported as such rather
than made to look like a connection.

## 5b. What the manifest must be asked, and what it cannot answer

§5 proposed `contracts()`. Measuring the corpus changes the shape of it, because
the two sides of a contract are discoverable in **different places**.

### The publisher side is manifest-shaped

**332 OpenAPI/Swagger documents across 9 projects, and exactly 1 under build
output.** They are authored and committed, not generated artifacts — so a
manifest can point at them and the pointer is stable.

### The consumer side is mostly NOT

The obvious manifest route is a generator config naming its input spec. Measured
in the watched roots: **8 NSwag configs, and zero for orval, kubb,
openapi-generator, graphql-codegen or buf.** That is not a registry; it is a
handful.

So a UI usually does **not** declare which endpoints it calls. What it declares
is a *dependency* — on a generated client package, or on nothing at all when it
calls `fetch` directly. The endpoints are in the code.

### Therefore: the manifest supplies the KEY, the walk supplies the USE SITES

```rust
/// What this package publishes or consumes, as far as its MANIFEST can say.
///
/// Deliberately not "which endpoints" — a manifest almost never knows. It knows
/// the JOIN KEY: the document this package publishes, or the client package it
/// depends on. Matching a call site to an operation is the walk's job, and
/// pretending otherwise would put a guess in the layer that is supposed to be
/// stating facts.
fn contracts(&self, parsed: &ParsedManifest, root: &Path) -> Vec<Contract>;
```

and, on the language side, a use site becomes an edge only when something names
the operation:

| the code says | what can be claimed |
|---|---|
| imports `getUser` from a generated client | the operation, by identity — a real edge |
| `fetch('/api/users/' + id)` | the PATH TEMPLATE, after normalising the interpolation |
| `fetch(url)` where `url` is computed | nothing. A use site with no target, and a reason |

The third row is the honest majority and must be reported as such. An API graph
that silently drops what it cannot place would claim a completeness it does not
have — the same inversion that made `imports` read as 6% placed.

### `scan_for` — what the manifest says is worth looking at

`classify_path` (§4) answers "parse this or not". This is its sibling and
answers "look for THIS in here":

```rust
/// Extra passes this manifest's ecosystem warrants, beyond the language walk.
///
/// Each is a PROBE, not a guarantee: a package declaring EF Core earns an ORM
/// pass, and that pass may still find nothing. The manifest narrows where to
/// look; it never asserts what is there.
fn scan_for(&self, parsed: &ParsedManifest) -> Vec<Probe>;

pub enum Probe {
    /// An OpenAPI/Swagger document at a stated path.
    OpenApiDocument(PathBuf),
    /// This package's ORM maps types to tables — `stack_labels` already names
    /// which one (`20-schema-entities.md` §ORM).
    OrmEntities(OrmKind),
    /// A `.proto` / GraphQL schema tree.
    SchemaTree(PathBuf),
    /// A generated client, so its call sites carry operation identities rather
    /// than path strings.
    GeneratedClient { package: String, from: Option<PathBuf> },
}
```

`stack_labels` already exists and is content-derived, so `scan_for` is mostly a
projection of a fact the adapter already reads rather than new detection.

## 5c. The full cycle

The stages exist. Two are missing and both slot into machinery already built —
the task queue's `blocked_by`, which is how `DetectCommunities` is already held
until every `ProcessFile` finishes, and which is the sole writer of
`folders.status = 'indexed'`.

```
ScanRoot
  └─ ProcessGitFolder                     a repo is found
       ├─ ProcessManifest                 package, deps, role,
       │                                  classify_path, scan_for      ← §4, §5b
       └─ ProcessRepoFiles
            └─ ProcessFile  × N           walk + per-file ladder
                 │
            ╔════╧═════════════════════════════════════════════╗
            ║  ResolveScope        NEW — blocked_by(file_ids)  ║  ← §3
            ║  every file read, so the scope is complete:      ║
            ║  place what the ladder left open, with a rung    ║
            ║  of its own or a reason                          ║
            ╚════╤═════════════════════════════════════════════╝
                 │
       EmbedNodes + DetectCommunities      repo complete → 'indexed'
                 │
            ╔════╧═════════════════════════════════════════════╗
            ║  ResolveCrossRepo    NEW — blocked_by(repos)     ║  ← §5
            ║  join publisher to consumer on the contract key  ║
            ╚══════════════════════════════════════════════════╝
```

Three things this ordering settles.

**`ResolveScope` goes BEFORE the folder is marked indexed.** `DetectCommunities`
is the sole writer of `'indexed'`, and a folder announced complete over a graph
that still has placeable edges open is announcing the wrong thing. It becomes
one more entry in the same `blocked_by` chain.

**`ResolveCrossRepo` is scoped to the ROOT, not to a repo.** It cannot be a
repo-complete step, because the other side may not be scanned yet. Its input is
the set of indexed repos, and it must be re-runnable: a repo indexed later
supplies a consumer for a publisher already on disk. That makes it closer to
`DetectCommunities` — a whole-scope derivation — than to a file task.

**It must say what it could not join.** A published document with no consumer in
the scan is an INVENTORY entry, not a connection, and the surface has to render
those differently or it will read as "nothing depends on this API" when the
truth is "the consumer is not indexed". `unresolved_reason` already has the
vocabulary for exactly this distinction — `external_boundary` is a refusal, not
a fault.

`indexer/cross_repo.rs` currently exists and is **empty** — a one-line
placeholder. That is where this lands.

## 6. Sequence

0. **Done:** `quality/` groups `acceptance`, `reachability` and `corpus`, each
   named for what it is.
1. **Move `adapters/manifest/` → `indexer/manifests/`.** Mechanical, no
   behaviour change, and it is what makes the rest read as one layer. Do it
   first so later diffs are about behaviour.
2. **Extract the receiver chase out of `db/pg_store/graph.rs`.** Small, and it
   settles the db seam before anything new lands on it.
3. **`classify_path` + `dbd.rs`.** Independent of everything else. Gate: the 618
   migration-only identities stop being minted, and `ddl/` is unaffected.
4. **`ScopeResolver`** (#193), with `SettledByScope`. Gate: this repo's placed
   edges cross a crate boundary, and the 9,025 verdict-less import edges carry a
   verdict — either placed, or refused with a reason.
5. **`scan_for` + `contracts`** (§5b), publisher side first — it is the side
   the corpus actually has.
6. **`ResolveCrossRepo`** (§5c) into `indexer/cross_repo.rs`, which is an empty
   placeholder today. Gate: the first edge in this graph that crosses a
   repository, and a stated count of publishers with no consumer indexed.

Steps 1–3 are independent and can run in any order. 4 is the one that changes
what the graph can answer; 5 is the one that changes what it is *about*.
