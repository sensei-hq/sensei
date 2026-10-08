---
name: first_party is every package the repo owns
description: The resolver's first-party set holds ONE package — the file's own — so no edge can ever cross a package boundary
date: 2026-09-30
status: ready
---

# `first_party` is every package the repo owns, not the file's own

## The defect, in one line

`crates/senseid/src/indexer/pipeline.rs:895`

```rust
first_party: [package.to_string()].into_iter().collect(),
```

The resolver's notion of "first party" is **exactly one package — the file's own**.
Every sibling crate in the workspace is therefore foreign, and a target in one
cannot be placed.

## Measured consequence

On sensei's own index, 2026-09-30, across **16 packages**:

```
 kind       | span         | edges
------------+--------------+-------
 calls      | cross_module |  1358
 references | cross_module |  1123
 ...
 cross_package                    0
```

**Not one placed edge has its source and target in different packages.** Checked
directly against `sensei.edges`, not through a view. `sensei-cli` visibly calls
into `senseid`; `senseid` uses `sensei-bootstrap` and `sensei-logger` throughout.
None of it resolves.

## Why the contract already says otherwise

`World::first_party` (`resolve.rs:257-261`) documents itself:

> *"Every package this scan owns the source of, **from the manifests** — NOT from
> what has been read so far. An import naming one of these crosses no boundary
> however it is spelled, so it resolves local; anything else the import calls
> external is a library (R5)."*

The contract is right. The construction never met it. This is not a design
change — it is making the code do what its own doc comment promises.

## The source of truth already exists

`ProcessManifest` already writes one `folders` row per manifest with
`kind = 'module'`, and **`folders.name` IS the package name**. Measured on
sensei: 18 module folders — `senseid`, `sensei-cli`, `sensei-bootstrap`,
`sensei-logger`, `@sensei/desktop`, `sensei-dojo-web`, `dojo-protocol`, … —
matching the 16 packages the graph observed plus two that declare no indexed
symbols.

So the first-party set is one indexed query, from manifest-derived rows, exactly
as the contract specifies. **No new indexing, no new table, no new scan pass.**

## Scope of this increment

**In:** `first_party` only.

**Out:** `first_party_members`, `declared_members` and `returns` — the other
three empty `World` fields. They unblock `declared_by_its_type` (0 rows in the
entire database) and receiver typing, and they are a larger piece of work with a
different verification. Splitting them keeps this increment to one measurable
claim: *cross-package edges resolve*.

## Design

1. `PgStore::first_party_packages(folder_id) -> BTreeSet<String>` — the names of
   every `kind='module'` folder in the project that owns `folder_id`.
2. `TellFile::about_packages(packages)` alongside the existing
   `TellFile::about(package)`, so the single-package form stays available to the
   tests that legitimately want one.
3. `process_file` asks for the set rather than wrapping its own package.

**Query cost, and why it is not cached yet.** One indexed query returning ~18
rows, per file. At 1,757 files that is ~2s across a repo scan, against a parse
that takes far longer. A per-repo cache is the obvious optimisation and the
obvious source of staleness bugs (a package added mid-scan), so it is deliberately
deferred until measurement says it matters.

## Done gate

1. `first_party_packages` returns 18 names for sensei's repo folder.
2. A forced re-index of **sensei only** (`--force` scoped to the repo, never the
   full set) completes.
3. `sensei.structure_edges` reports a **non-zero `cross_package` count** where it
   reported zero.
4. `sensei-cli → senseid` appears as a placed edge, named specifically.
5. No regression in the existing placed totals — cross-package edges are added,
   never traded for intra-package ones.

## Wrong gate

- **Packages read from fqns instead of manifests.** That is "what has been read
  so far", which the contract explicitly rejects — and it is circular: a package
  no file has resolved into yet would never enter its own first-party set.
- **The repo folder's name used as a package.** `sensei` the repository is not
  `sensei` the package; the `database` module folder happens to be named
  `sensei`, and conflating them would make every DDL symbol first-party to
  everything.
- **Edges appearing but pointing at the wrong package.** A cross-package count
  that rises while `sensei-cli → senseid` is still absent means something matched
  by name alone.

## Related

- [Observatory Diagrams gap analysis](../../analysis/2026-09-30-observatory-diagrams-gap-analysis.md)
- #205 — the Structure screen this unblocks
- #206 — diagram view performance, to be tuned only after this changes the shape
