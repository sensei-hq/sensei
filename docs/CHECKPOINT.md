# Checkpoint

**Slice** — indexer-v2 **stage 11** (`docs/spec/indexer/11-file-index.md`):
one file in, nodes and edges out. **4 of 12 increments done.** Rust only (§11).

**Done** — `c7cd34ea` clippy cleanup (5 errors under `--all-targets`) ·
`3852a199` **S8**, a method is keyed on its type and its name; the trait is a
`TraitImpl` edge · `12d5c096` one impl header stated two identities for one
type · `a13e1e65` the collapsed-spelling translator deleted (391 lines) behind
a source guard · **I4** `Named` vs `Candidate` — `Home::Stated`/`Tabled`,
`Observation::Named`, `Rung::NamedByThisFile` + its seed row.

**MEASURED over this repository, per increment** — resolved references
96,386 → 96,418 (S8) → 96,420 (I2) → 96,336 (I3) → **97,049** (I4).
§10's bar is ≥96,304. `NoImportInScope` 4,107 → 3,495.

**Two measurement traps, both now written into the tests** — (1) the corpus IS
this repo, so deleting production code lowers `resolved` with no resolver
change: read BOTH columns and the TOTAL (I3 fell 84 and was not a regression).
(2) A ratchet that prints nothing on success can only be lowered by editing
the test; the split-impl share now prints.

**Ratchets moved, each with its justification written in beside it** — A7 rust
collisions 3 → 4 (the one sanctioned raise; S8 merges `ModelProvisioning`'s
inherent `status_all` with its `ReadinessProbe` one — the ONLY pair in this
repo). Split-impl anchoring 13 → 12. First-party references naming an identity
no declaration mints **320 → 1,300** — see `docs/backlog.md`, decided with the
user: stage 12 must resolve these or mark them EXTERNAL, never leave a ghost.

**Next — I5 is NOT an increment** (the bare-name guard already covers it; both
its mutation probes were run and went red). Start at **I6a**: a type this file
never names is not the external boundary — `Home::Unstated`, so `Home::NotOurs`
stops meaning "the table said nothing". Then I6b (rust walk drops `TypeHomes`),
I7 (`guard_sources` must read `index.rs` — spec S1's claim that it already does
is FALSE), I8 (`index_file`/`FileInput`/`FileIndex`), I9 (Delete mode), I11
(conservation), I12 (measure + record).

    cargo test -p senseid --bin senseid -- indexer::
    cargo test -p senseid --bin senseid -- --ignored --nocapture index::corpus

**Open questions** — spec §5/§8 draw the `TraitImpl` edge trait→type; the code
and the whole `implements` column run type→trait. Kept the code's direction;
the spec needs correcting. Spec §4.2's `via` list omits `NamedByThisFile`.

**Gate at HEAD** — fmt clean, clippy `--all-targets -D warnings` 0, senseid
3274/0 (full workspace 3,627 at the slice start). 4 ignored tests fail for
environmental reasons and are the same 4 the previous checkpoint named:
dbd-rs path, gateway config, 2 installer hooks. None in the indexer.
