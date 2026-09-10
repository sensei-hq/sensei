# Stage 2b — library discovery: packages to libraries, skills, agents, docs

Whole-system spec: `docs/design/indexer-v2.md` R10.7f, R10.7g, R10.7h, R11.1,
R11.2, R12. Depends on stage 2. Does NOT depend on any parsing.

## 1. Purpose

Populate the grouping that turns a package reference into everything known
about the library it belongs to. `library_packages` — the join at the centre
of that chain — has **ZERO rows** today, so 130 documentation pages across 128
components, 10 skills and 6 agents are indexed and unreachable from any node.

This is the single largest G1 payoff in the whole plan and it is placed at 2b
because it is INDEPENDENT of parsing: `library_packages` is keyed by package
NAME, so it can be populated as soon as stage 2 has read the manifests. Putting
it after persistence would imply a dependency that does not exist and would
queue the biggest payoff behind the slowest stage.

## 2. Inputs and outputs

    read_library_manifest(json: &str)             -> LibraryManifest       PURE
    group_from_workspace(members: &[PackageInfo]) -> Vec<PackageGrouping>  PURE
    save_library(LibraryManifest, provenance)     -> LibraryId             IO

| input | stage 2's manifest facts: package names, workspace members, dependency list |
|---|---|
| output | rows in `libraries`, `library_packages`, `library_skills`, `library_agents`, and a located pages corpus |

## 3. Requirements

- **S1** (R10.7h). When a dependency ships `sensei.library.json` at its root,
  read it. One file yields the library, its skills, its agents and the location
  of its docs corpus — four of the five tables in the chain.
- **S2** (R10.7h). The manifest does NOT carry the package list. Nothing in
  `rokkit/sensei.library.json` states that `@rokkit/ui` belongs to `rokkit`.
  Populate `library_packages` from the repo's WORKSPACE MEMBERS (`package.json`
  `workspaces`, `Cargo.toml` `members`), which stage 2 already extracted via
  `detect_workspace_members`. That is equally declared, just stated elsewhere.
- **S3** (R10.7f, R11.1). **NEVER infer a grouping.** `@rokkit/*` -> `rokkit`
  is a tempting prefix rule and it is a guess: `@types/node` belongs to no
  "types" library and gateway's crates share no prefix at all. An invented
  grouping is a fabricated fact about a dependency and it would be believed.
- **S4** (R11.1). Record PROVENANCE on every grouping — `declared_by_dependency`
  (the manifest), `declared_by_user`, or absent. `library_kind` already carries
  `detected | imported` and is the natural home. A manifest is the dependency
  speaking about itself; a user is a local judgement. They differ in authority
  and must stay distinguishable forever.
- **S5** (R11.1). An ungrouped package stays ungrouped. That is COMPLETE, not
  wrong — the graph is not missing something, it is simply not yet grouped.
- **S6** (R11.2, IN SCOPE HALF ONLY). When a user supplies a grouping, emit the
  `sensei.library.json` they could keep. Do NOT contact the dependency's
  repository: measured, all 1,121 `libraries` rows are `kind = 'detected'` and
  exactly **2 carry any URL at all**, so there is nothing to file against for
  1,119 of them. The upstream half is deferred with its input named.
- **S7.** Populate `homepage_url` from `parse_dependencies` where the manifest
  supplies it. This is the prerequisite S6's deferred half needs, it is a field
  not a subsystem, and it is why the 2-of-1,121 number should move.

## 4. Failure modes

| input | this stage does |
|---|---|
| no `sensei.library.json` anywhere | zero declared groupings. Expected — most dependencies will never ship one. Not a gap to fill by inference. |
| the manifest is malformed | record the failure against the library with the parse error; do not partially apply half a manifest. |
| the manifest names skills/agents at paths that do not exist | record the library, skip the missing entries, name them. A manifest can be ahead of its own repo. |
| a workspace member is not a published package name | do not add it to `library_packages`; the grouping key is the package name a reference can see. |
| two libraries claim the same package | REPORT it. This is the library-level analogue of an A7 collision, and picking a winner would be scan-order dependent. |
| a package appears in no workspace and no manifest | ungrouped, per S5. Complete. |

## 5. Checkpoint output

    {"stage":"02b-library-discovery","at":"<iso8601>",
     "packages_seen":1121,"library_manifests_found":<n>,
     "libraries_upserted":<n>,"library_packages_rows":<n>,
     "grouped_packages":<n>,"ungrouped_packages":<n>,
     "provenance":{"declared_by_dependency":<n>,"declared_by_user":<n>},
     "skills":10,"agents":6,"pages_located":130,
     "homepage_url_populated":<n>,
     "collisions":[],
     "sample_grouping":{"library":"rokkit","packages":["@rokkit/ui","@rokkit/core"],
                        "provenance":"declared_by_dependency",
                        "skills":5,"agents":3,"pages":130}}

`grouped + ungrouped` must equal `packages_seen`. `ungrouped` starting near
1,121 and falling is the metric this stage exists to move; it is not a failure
count.

## 6. Verification

| test | mutation that must break it |
|---|---|
| `read_library_manifest` on the real `rokkit/sensei.library.json` yields 5 skills, 3 agents and a docs path | drop a field from the parser |
| `library_packages` is populated from workspace members, not from the manifest | make S2 read the manifest for packages — it has none, so the table stays empty |
| `@types/node` is NOT grouped under a "types" library | add a prefix rule |
| two gateway crates sharing no prefix ARE grouped when a workspace declares them | make grouping prefix-based |
| a user grouping and a manifest grouping are distinguishable after a round trip | store both with the same `library_kind` |
| an ungrouped package produces no row and no error | treat ungrouped as a failure |
| two libraries claiming one package is REPORTED, not silently resolved | pick the first |
| the node -> package -> library -> pages chain returns rokkit's `List` page for a `@rokkit/ui` `List` reference | leave `library_packages` empty |

That last one is the acceptance test for the whole stage: it is the G1 payoff
stated as a query, and it is unanswerable today.

## 7. Watch out

**The model is already right; the data is missing.** An earlier draft of the
design recorded that the library level "does not exist". It does —
`sensei.library_packages (package_name, library_id, source)` is exactly the
grouping, joinable to the fqn's package segment with no new column, and it even
carries `source` for the provenance R11.1 requires. It has zero rows. A missing
subsystem and an empty table need very different work.

**`sensei.libraries` currently holds PACKAGES, not libraries.** `@rokkit/actions`,
`@rokkit/app` and `@rokkit/core` are separate rows, all `kind = 'detected'`,
because with `library_packages` empty there is nothing to group them under.
Populating the grouping does not mean deleting those rows — it means a
`rokkit` row appears above them. Decide and write down what happens to the
package-level rows before writing the migration; leaving both levels in one
table with no discriminator is how the next reader gets it wrong.

**`library_skills` and `library_agents` are near-identical** (`library_id, name,
focus, body, source, source_path, version_range, origin, scope`; skills add
`tokens`/`generated_at`). R12 notes that a single `library_content` with a type
discriminator would make a fifth kind cost a row rather than a table. **Decide
that shape BEFORE populating `library_packages`**, because after this stage
there is live data in the pattern and it becomes a migration of 146+ rows.

## 8. Definition of done

- `library_packages` is non-zero and every row carries provenance.
- The `node -> package -> library -> pages/skills/agents` query returns real
  content for at least one real reference.
- Ungrouped packages are counted, not errored, and the count is in the
  checkpoint.
- No prefix inference exists anywhere in the stage — verified by a test, not by
  reading.
- The `library_content` shape question is answered in writing, either way.
