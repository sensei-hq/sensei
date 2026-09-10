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

## 3b. THREE INGESTION ROUTES, and what already exists for each

A library's metadata and docs can arrive three ways. `libraries.source_type`
already names them as an enum — `llms.txt | http | local` — so the routes are
not new; their TRIGGERS and their PRECEDENCE are.

| route | trigger | status |
|---|---|---|
| **local** | folder traversal — stage 2 reads the manifests | BUILT. This stage's S1–S5. |
| **website** | a `docs_url` / llms.txt is known for the package | PARTLY — `indexer/llms_indexer.rs` ingests; nothing decides WHEN to go looking |
| **github** | a repository URL is known for the package | NOT BUILT. `registry.rs` defers GitHub Releases explicitly. |

### S8 — extract the repository URL from responses ALREADY BEING FETCHED

The github and website routes both need a URL, and neither has one: measured,
**2 of 1,121 library rows carry any URL at all**. But
`libraries/registry.rs::registry_latest_url` already fetches, per ecosystem:

    npm    https://registry.npmjs.org/<name>/latest   -> repository.url, homepage
    cargo  https://crates.io/api/v1/crates/<name>     -> crate.repository, crate.homepage
    pypi   https://pypi.org/pypi/<name>/json          -> info.project_urls

**The URLs are in responses the code already downloads and throws away.**
Extract `repository` -> a repo URL and `homepage`/`docs` -> `homepage_url` /
`docs_url`. This is a parse addition to an existing pure helper, unit-testable
against sample bodies exactly as `registry_latest_url` already is — not a new
subsystem, and not a new network call.

It is the prerequisite for three separate things: the github route, the
website route's trigger, and R11.2's deferred upstreaming, which is blocked
today precisely because 1,119 of 1,121 packages have nothing to file against.

**FAIL-CLOSED, matching the module it joins.** A missing or unparseable
repository field yields `None`. Never guess a URL from the package name —
`github.com/<name>/<name>` is wrong far more often than it is right, and a
fabricated URL is worse than none because something will fetch it.

### S9 — source precedence: website > github > local

When more than one route can supply docs for one library, prefer in that
order, and RECORD which one won on the row. The reasoning:

- **website** — the library's own published docs. Curated, current, and what
  its authors intend a reader to see.
- **github** — raw but complete, and the only route with VERSION HISTORY.
- **local** — whatever happens to be vendored in `node_modules/` or the
  registry cache. A real fallback, and often stripped of docs entirely.

**THE PRECEDENCE INVERTS WHEN THE VERSION DOES NOT MATCH, and this is the
subtlety worth stating.** A website documents ONE version, normally the
latest. If the project pins `1.2` and the site documents `3.0`, the site is
the higher-quality source of the WRONG answer, and github's tag for `1.2` is
the correct one. So:

    pick = highest-precedence source THAT CAN SERVE THE PINNED VERSION

with the version taken from `referenced_libraries.version_used` (populated on
1,001 rows today). A source that cannot serve the pinned version is not
preferred over one that can, regardless of rank. If NO source can, serve the
closest available and **label it as a version mismatch** — an answer marked
"this documents 3.0, you are on 1.2" is useful; the same answer unlabelled is
a wrong one (R4).

### S10 — versioned docs: THE SCHEMA CANNOT REPRESENT THIS TODAY

`libraries` is `unique(ecosystem, name)` — **one row per library, therefore
one version.** `library_pages` has no version column at all
(`id, library_id, title, url, local_path, description, content, source_type,
component, embedding, fetched_at, modified_at`). So "docs for v1.2 vs v3.0"
is currently unrepresentable, and so is a skill or agent that differs between
versions.

Two shapes, and **this is a DECISION to take before this stage writes
anything**, because after it there is live data in whichever shape was
guessed:

| option | shape | cost |
|---|---|---|
| A — version column on content | add `version` to `library_pages`, `library_skills`, `library_agents` | 3 tables, and `libraries.version` becomes ambiguous — is it "latest known" or "the one we hold"? |
| B — a `library_versions` level | `libraries -> library_versions -> pages/skills/agents` | one more join; `libraries` becomes purely the identity and `version` moves off it, which is cleaner but touches more |

**Decide this together with R12's `library_content` question** — whether
`skill | agent | page | package` collapse into one table with a type
discriminator. Both questions are "what sits between a library and its
content", answering them separately risks two migrations over the same
146+ rows, and R12 already says the shape should be settled BEFORE
`library_packages` is populated.

Recommendation: **B**, because A puts the same version string on three tables
and nothing keeps them consistent — the drift R10.7e names. But it is the
user's call and it is not blocking S1–S7.

### What is already built and must NOT be rebuilt

`libraries/version.rs` is the policy heart: `Bump` and `UpdateAction` decide
what a version delta MEANS, `advisory.rs` escalates on OSV severity, and
`tasks/library_update_scheduler.rs` drives the tick. Version-drift
recommendations — the second use in the brief — EXIST. What this stage adds
is the version-MATCHED docs half (S9, S10), not the update-recommendation
half.

Note the constraint that module states about itself and keep it: **the apply
path is ALWAYS a docs/skills refresh, never a change to the consuming
project's code.**

## 4. Failure modes

| input | this stage does |
|---|---|
| no `sensei.library.json` anywhere | zero declared groupings. Expected — most dependencies will never ship one. Not a gap to fill by inference. |
| the manifest is malformed | record the failure against the library with the parse error; do not partially apply half a manifest. |
| the manifest names skills/agents at paths that do not exist | record the library, skip the missing entries, name them. A manifest can be ahead of its own repo. |
| a workspace member is not a published package name | do not add it to `library_packages`; the grouping key is the package name a reference can see. |
| two libraries claim the same package | REPORT it. This is the library-level analogue of an A7 collision, and picking a winner would be scan-order dependent. |
| a package appears in no workspace and no manifest | ungrouped, per S5. Complete. |
| a registry response has no `repository` field | `None` (S8). Never derive a URL from the package name — `github.com/<name>/<name>` is wrong more often than right, and something will fetch a fabricated one. |
| the repository URL 404s or is a dead redirect | record the URL with the failure; do not blank it. The next attempt needs to know it was tried and what happened. |
| the website documents a version the project does not use | serve it LABELLED as a mismatch (S9). An unlabelled wrong-version answer is the R4 failure. |
| no source can serve the pinned version | serve the closest with the mismatch label, and count it. This is a gap with a fill path (R11), not an error. |
| a library has two candidate repository URLs | take the registry's, not the manifest's — the registry is the publisher speaking. Record both. |

## 5. Stage report — what you SHOW when the stage is done

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
  stage report.
- No prefix inference exists anywhere in the stage — verified by a test, not by
  reading.
- The `library_content` shape question is answered in writing, either way —
  **together with S10's versioning shape**, since both decide what sits
  between a library and its content.
- `repository`/`homepage` extracted from registry responses already being
  fetched (S8); the 2-of-1,121 URL count moves, and it is measured.
- Source precedence recorded per library, with the version-mismatch label
  present on any doc served for a version the project does not pin (S9).
- No URL is ever derived from a package name, verified by test.
