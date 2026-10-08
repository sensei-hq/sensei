---
name: The world the resolver is never given
description: Four of World's five fields are empty in production, so three rungs can never fire
date: 2026-09-30
status: ready
---

# The world the resolver is never given

## The defect

`TellFile::about` (`pipeline.rs:893`) builds the resolver's `World` with four of
its five fields as fresh empty collections:

```rust
members:  BTreeSet::new(),
declared: BTreeSet::new(),
returns:  BTreeMap::new(),
scanned:  BTreeSet::new(),
```

`first_party` was fixed in #207/#208. The other three are still empty, and
nothing ever writes to them. `index_repo` — which does build a real world at
`index.rs:117` — has **18 call sites and every one is inside a `#[cfg(test)]`
module**.

## Measured consequence

| | |
|---|---|
| `declared_by_its_type` rows in the ENTIRE database | **0** |
| `receiver_type_unknown` unresolved call edges, sensei | **30,676** |
| `Ladder::through_what_the_receiver_returns` | early-returns on `world.returns.is_empty()` (`resolve.rs:694`) |
| the unit-struct branch (`resolve.rs:698-703`) | unreachable — it needs only `declared_members`, but sits *below* the `returns` guard |
| `ExternalBoundary` reclassification (`resolve.rs:922`) | gated on `!world.first_party_members.is_empty()` |

The resolver has three rungs it can never climb, and a refusal it can never make.

## What each field is, exactly

Taken from the builders that already exist, so the DB version cannot drift from
them:

| field | builder | rule |
|---|---|---|
| `first_party_members` | `member_names_of` | symbol NAMES where kind ∈ {Method, Field, Property} |
| `declared_members` | `members_declared_by` | the member IDENTITIES (fqns) a type owns |
| `returns` | `returns_declared_by` | fqn → declared type, `&` stripped, `Self` resolved to the enclosing type from the fqn |

The DB carries all three: `nodes.name`, `nodes.fqn`, `nodes.kind`,
`nodes.parent_id`, and `nodes.props->>'declared_type'` — which is populated on
**every** node row.

**`Owns` is persisted as a `references` edge carrying `props->>'relation' =
'owns'`** (`persist.rs:637`, `persist.rs:677`) — 408,156 such rows database-wide.
That is the faithful source, and `members_declared_by`'s own doc says why a
`SymbolKind` filter cannot stand in for it: it "cannot be stood in for by a
`SymbolKind` filter without getting a different answer in each language".

**Measured, and it matters:** the kind filter gives sensei **8,911** member
fqns; the `owns` relation gives **1,806**. Had the gate below been written
against the approximation it could only have been met by using the wrong
query.

## Caching is mandatory here, unlike #207

Measured on sensei: **3,403 member names + 1,806 member fqns + 10,612 return
types ≈ 15,800 rows** per world. `process_file` runs per FILE, so querying
per-file would move ~40 million rows across one repo scan.

`first_party` was ~18 rows and could be read per file. This cannot.

### The cache is correct, not just fast — and that is the important part

`World` is documented as *"A BARRIER artifact like the type table: built from a
completed pass"*. The world a file resolves against is the **previous** pass's
knowledge, deliberately — that is what makes R6 (order independence) hold. A
world rebuilt mid-scan would mean file 500 resolved against more knowledge than
file 1, and the same repo would produce different graphs depending on scan order.

So: build once per repo scan, reuse for every file in it. Stale within the scan
**by design**.

**Invalidation has exactly one writer:** `process_repo_files` — the manifest gate
— runs once per repo immediately before the file fan-out. It clears the entry;
the first `process_file` rebuilds it. Any other invalidation point would be a
second writer and would reintroduce the order dependence.

## Scope

**In:** `first_party_members`, `declared_members`, `returns`, plus the per-repo
cache they require.

**Out:** `scanned`. It is deliberately never read (R6) and must stay empty —
filling it would be the order dependence the barrier exists to prevent.

**Out:** moving the unit-struct branch above the `returns.is_empty()` guard.
Related and small, but a separate behaviour change with its own verification.

## Done gate

1. A world built for sensei's repo folder holds > 3,000 member names, > 1,700
   member fqns (from the `owns` RELATION, not a kind filter) and > 10,000 return
   types.
2. The cache is built once per repo scan — asserted by a test that counts
   builds across N files, not by reading the code.
3. `process_repo_files` clears it; a test proves a second scan rebuilds.
4. Forced re-index of **sensei only**.
5. **`declared_by_its_type` rows > 0** — the rung that has never once fired in
   the database's history.
6. `receiver_type_unknown` on sensei falls materially below 30,676.
7. `external_boundary` rises — the reclassification that was gated off.
8. No regression: placed totals do not fall.

## Wrong gate

- **A world rebuilt per file.** Correct output, 40M rows moved, and the order
  dependence R6 forbids.
- **`scanned` populated.** The one field that must stay empty.
- **`Self` left unresolved in `returns`.** `returns_declared_by` resolves it to
  the enclosing type; a DB version that stores the literal `"Self"` would point
  every such member at a type named `Self`.
- **`&` left on a stated type.** Same builder strips it; keeping it means
  `&PgStore` never matches `PgStore`.
- **Members taken from a `kind` filter instead of the `owns` relation.** It is
  the documented trap and it is not subtle: 8,911 against 1,806 on sensei. A
  gate written against the larger number would be passed by the wrong query.

## Related

- #207, #208 — `first_party`, the first two fields of the same struct
- #205 — the Structure screen, which this widens further
