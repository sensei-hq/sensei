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

**THE FETCHERS ALL EXIST.** `LibSource` (`indexer/lib_indexer.rs:353`) is
exactly these three, `detect_lib_source` classifies a URL into them, and
`resolve_library_pages` implements all three arms. What is missing is not
fetching — it is the TRIGGER that decides when to fetch.

| route | `LibSource` | what it walks | trigger status |
|---|---|---|---|
| **local** | `LocalDir(path)` | `resolve_local_llms_root`: the path itself if named `llms` or holding `index.txt`/`llms.txt`/`llms-full.txt`, ELSE `<path>/docs/llms`. Then `.txt` files at top level + `components/`. **Markdown is not read.** | **MISSING** |
| **git** | `GitHubTree{owner,repo,branch,path}` | a `github.com/{owner}/{repo}/tree/{branch}/{path}` docs dir | MISSING |
| **website** | `Website(url)` | an llms index or single file | MISSING |

`index_library` requires a task carrying a URL and errors without one, so
every route is manual today.

### The measured evidence — two OPPOSITE failures, neither detected

| library | `docs/llms/` on disk | pages | state |
|---|---|---:|---|
| rokkit | 119 `.txt` | 94 | current, fetched 2026-08-03 |
| dbd | **ABSENT** | 36 | **STALE** — pages point at `~/Developer/dbd-rs/docs/llms/llms.txt`, a directory that no longer exists. Fetched 2026-06-30 and served as current ever since. |
| kavach | **15 `.txt`** | **0** | **NEVER INDEXED** — the content is right there |

kavach is the sharper of the two, because nothing is missing except the
trigger: `ingest_manifest` (`libraries/mod.rs:107`) already recorded
`local_path`, so the system KNOWS where kavach is, its `docs/llms/` holds 15
`.txt` files, and it has 4 skills and 2 agents ingested from its manifest. It
has zero pages purely because no one enqueued the walk.

dbd is the inverse and the more dangerous: content was fetched once, the
source was deleted, and the pages are still returned as though current. R11
says a gap is DATA with a fill path — this is a gap that does not even
register as one.

### S7b — the LOCAL trigger, and a staleness check

- **S7b.1.** When `ingest_manifest` records a `local_path`, ENQUEUE an
  `index_library` for that path. That is the local route's trigger and it
  closes kavach. The path is already in hand; nothing needs discovering.
- **S7b.2.** `read_local_source_files` ERRORS when the root has no `.txt`
  files. That error is the STALENESS SIGNAL — record it against the version
  (`files.skip_detail`'s analogue for libraries) rather than letting the task
  fail silently. dbd would have surfaced on the first re-run.
- **S7b.3.** **Do not delete stale pages on a failed fetch.** dbd's 36 pages
  are the last known-true content; deleting them on a transient read error
  trades a stale answer for no answer. Mark the version stale, keep the
  content, and label it on read — the same rule R10.7 applies to a dirty file.
- **S7b.4.** The local convention is `docs/llms/*.txt`, NOT markdown. A
  library with a rich `docs/` tree and no `docs/llms/` yields nothing, which
  is correct behaviour and a confusing result — say so in the stale/empty
  reason rather than reporting an empty success.

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

**DECIDED (2026-09-10): option B — a `library_versions` level, with all
library CONTENT linked to a version.** The DDL moves into stage 0 so the table
is opened once. Shape:

    libraries          identity only: kind, name, ecosystem, description
      └──< library_versions   (id, library_id, version, resolved_version,
                               source_type, base_url, docs_url, fetched_at)
             ├──< library_pages    -> library_version_id
             ├──< library_skills   -> library_version_id
             └──< library_agents   -> library_version_id

Three consequences to carry into the migration:

- **`version` moves OFF `libraries`.** Populated on 1,083 of 1,121 rows today,
  so the migration is one `library_versions` row per existing
  `(library, version)`, then drop the column. `libraries` keeps
  `unique(ecosystem, name)` and becomes purely the identity.
- **`resolved_version` sits beside `version`** (S11): `version` is the KEY,
  including the literal `latest`; `resolved_version` is the concrete version
  `latest` meant at `fetched_at`. Without it, "is our latest still latest?"
  is unanswerable.
- **`referenced_libraries.version_used` stays TEXT, not an FK.** A folder can
  pin a version we have never fetched docs for, and a foreign key would either
  block that or force a phantom row — the get-or-create failure R13 forbids
  one table over. Join opportunistically; a miss is an honest "no docs for
  this version".

**`library_packages` stays at the LIBRARY level, not the version.** Grouping is
about identity, and the node -> package -> library chain (R10.7g) resolves from
an fqn that carries NO version, so a version-keyed grouping could not be walked
from a node at all. Package membership CAN drift across releases — `@rokkit/x`
present in v1, gone in v2 — and that is a DEFERRED refinement (R11: the
evidence is captured, the gap is named), not a reason to key the grouping on
something the caller does not have.

The rejected option, recorded so it is not re-proposed:

| option | shape | cost |
|---|---|---|
| A — version column on content | add `version` to `library_pages`, `library_skills`, `library_agents` | 3 tables, and `libraries.version` becomes ambiguous — is it "latest known" or "the one we hold"? |
| B — a `library_versions` level | `libraries -> library_versions -> pages/skills/agents` | one more join; `libraries` becomes purely the identity and `version` moves off it, which is cleaner but touches more |

**DECIDED (2026-09-11) together with R12's `library_content` question:
COLLAPSE.** `library_skills`, `library_agents` and `library_pages` became one
`library_content` table discriminated by `kind` (`skill | agent | page`),
keyed to `library_version_id`. The three shared nine columns and the agent
table's own comment described it as "mirrors library_skills minus the
generation fields"; a fifth kind cost a fourth table and a fourth arm in every
"everything about library X" query, and now costs a row.

Kind-specific columns are nullable BY DESIGN, each documented with its owning
kind, so a null reads as "not applicable to this kind" and never as "unknown".

`library_packages` is deliberately NOT folded in. It is a GROUPING — package
name to library — not content, it carries no body, and R10.7g resolves it from
an fqn that has no version. Putting it under a version-keyed content table
would make the node → package → library chain unwalkable.

**The 146-row migration this section warned about did not happen: stage 0's
wipe had already emptied all three tables**, verified in `sensei`,
`sensei_test` and `sensei_e2e` before dropping. Note for the next time a table
is retired: `dbd reconcile` does NOT drop a table whose DDL file was deleted —
`0 pruned` — so the DROP has to be explicit.

Recommendation: **B**, because A puts the same version string on three tables
and nothing keeps them consistent — the drift R10.7e names. But it is the
user's call and it is not blocking S1–S7.

### S11 — `find_libraries`: enqueue resolved work, never a puzzle

An extension of stage 2's manifest pass, not a new walk. `parse_dependencies`
already returns `Vec<DepVersion>` per manifest, and **`DepVersion` already
carries the payload shape**:

    lib_name, version, raw_version, source, dev, local_source

So the task gets name + version + ecosystem and spends no time re-deriving
what the manifest already said. The task's job is FETCH AND PROCESS.

**TRAP 1 — `DepVersion.source` is NOT the fetch source.** It holds the
MANIFEST FILENAME: `"Gemfile"`, `"Package.swift"`, `"Cargo.toml"`. That is
provenance. The S9 route (website / github / local) is a different concept
and the two must not share a field name — call the new one `origin` or
`fetch_source`. Conflating them is a silent bug: both are strings, both are
plausibly "source", and nothing type-checks the difference.

**TRAP 2 — scan_repo is OFFLINE and must stay that way.**
`parse_dependencies(&self, content: &str) -> Vec<DepVersion>` is synchronous
and takes CONTENT. Resolving the fetch source needs the registry (S8), which
is network. So `find_libraries` **cannot decide the origin**, and making it
try would put network I/O in front of the structure barrier (R14) and break
a scan on a plane.

The split that keeps both properties:

    scan_repo (offline)   -> enqueue (ecosystem, name, version, origin?)
                             origin carried ONLY if already cached on the
                             library row from a previous resolution
    the task (network)    -> resolve origin if absent, CACHE it, fetch, process

First encounter resolves once; every later encounter carries it. That is the
"don't decipher it again" property in the steady state, without a network
call at scan time.

**Dedup: enqueue only if `(ecosystem, name, version)` has no content** — and
the version must be the RESOLVED PIN, which the manifest does not have.

**CORRECTION — an earlier draft of this spec claimed "927 of 1,001
`version_used` values are exact pins". That counted STRING SHAPE, not
semantics, and it is wrong.** `indexer/lib_indexer.rs::clean_version` strips
`^ ~ >= <= > < =`, so `^7.0.0` is stored as `7.0.0` — a range FLOOR wearing
the shape of a pin, and indistinguishable from one once the operator is gone.
Concretely: `app/package.json` declares `"@sveltejs/kit": "^2.8.0"` and the
graph stores `2.8.0`, while the installed version is whatever 2.x resolved to.
`svelte` is stored at `3.17.3` and `3.19.1` in other repos for the same
reason.

Storing a floor as if it were a pin is an R4 fabrication: a plausible value a
caller cannot tell from a real reading. Fetching docs for `7.0.0` when `7.4.2`
is installed is exactly the silently-wrong answer S9's mismatch label exists
to prevent — except here nothing knows to label it.

**So: THE MANIFEST SELECTS WHICH PACKAGES; THE LOCKFILE SUPPLIES WHICH
VERSION.**

| question | source | why |
|---|---|---|
| which packages do we care about? | MANIFEST | it declares only what the project chose — this is what keeps the transitive tree out |
| what version is actually installed? | LOCKFILE | it is the only place the resolved pin exists |

Reading a lockfile to LOOK UP versions for an already-selected set of direct
deps does NOT pull in the transitive tree — you index into it by name, you do
not enumerate it. The two constraints are independent, and an earlier draft
of this section wrongly treated them as one, concluding "do not read
lockfiles" from "do not index transitive deps".

No adapter reads a lockfile today — verified, zero references to
`package-lock`, `pnpm-lock`, `yarn.lock`, `Cargo.lock`, `poetry.lock` or
`bun.lock` anywhere in `crates/`. So this is new work, one lockfile reader per
ecosystem, and it is the prerequisite for the dedup key meaning anything.

**Until a lockfile is read, keep the operator.** `DepVersion.raw_version`
already preserves `^7.0.0`; `version_used` stores the cleaned form and throws
the evidence away. A range recorded AS a range is honest and can be resolved
later; a range recorded as a pin cannot be distinguished from one and will be
believed.

**`latest` IS a version, and that is right — with one addition.** The 74
non-pin values are dominated by `*` (19 rows): unpinned dependencies. Mapping
them to the literal version `latest` removes the need for a separate
"latest-known" concept, exactly as proposed, and makes the dedup key total.

But **`latest` is a MOVING TARGET**, so the row must also record WHICH
concrete version it resolved to when fetched:

    (react, 'latest')  resolved_version = '18.2.0'  fetched_at = …

Without `resolved_version`, "do we have latest?" is answerable and "is our
latest still latest?" is not — and the second is the question
`library_update_scheduler` exists to ask. With it, staleness is a comparison
against `registry.rs`'s `VersionSource::latest`, which is already built.

Two rows for one library (`latest` and `18.2.0`) is CORRECT, not duplication:
one project pins, another floats, and they want different docs.

**DIRECT DEPENDENCIES ONLY. Never the transitive tree.** This already holds
and the reason is structural, not a filter: `parse_dependencies` takes the
MANIFEST's content, and a manifest declares only what the project chose —
`package.json`'s `dependencies`/`devDependencies`, `Cargo.toml`'s
`[dependencies]`. The transitive closure lives in the LOCKFILE, which no
adapter reads.

**But DO read the lockfile for the VERSION** of each package the manifest
already selected — see S11. The two things a lockfile offers are separable
and only one of them is unwanted:

| lockfile gives | verdict |
|---|---|
| the resolved pin for a package we already chose | **WANTED.** It is the only place that pin exists. |
| the full transitive closure | **NOT WANTED.** One to two orders of magnitude larger, packages nobody here writes code against. |

Take the first by LOOKING UP the direct deps by name; never by enumerating
the file. An earlier draft of this section conflated the two and concluded
"do not read lockfiles", which would leave every version a range floor.

Two exclusions that already work the same way and must be kept:

- **`local_source` deps are not libraries.** npm `link:`/`workspace:`/`file:`
  and Cargo `path=` resolve to a local sibling; `DepVersion`'s own comment
  records that the writer routes `Some(_)` to `project_dependencies` and
  SKIPS the external-library upsert. That is correct — a workspace sibling is
  first-party code, and stage 4 indexes it properly.
- **`dev` is a flag, not an exclusion.** Dev dependencies are still DIRECT and
  you write real code against them (test frameworks, build tools). Keep them,
  keep the flag, and let a consumer filter. Dropping them would make "how do
  I test this" unanswerable for the very packages that answer it.

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

## 5. Verification

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

## 6. Watch out

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

**RESOLVED — `library_skills` / `library_agents` / `library_pages` are now one
`library_content` table** discriminated by `kind`, keyed to
`library_version_id`. See S10 above for the decision and what it cost (nothing
— the tables were already empty). A fifth kind of content is now a row.

## 7. Definition of done

- `library_packages` is non-zero and every row carries provenance.
- The `node -> package -> library -> pages/skills/agents` query returns real
  content for at least one real reference.
- Ungrouped packages are counted, not errored, and the count is in the
  the run.
- No prefix inference exists anywhere in the stage — verified by a test, not by
  reading.
- [x] The `library_content` shape question is answered in writing — COLLAPSE,
  together with S10's versioning shape (option B, `library_versions`). Both
  are implemented; see S10.
- `repository`/`homepage` extracted from registry responses already being
  fetched (S8); the 2-of-1,121 URL count moves, and it is measured.
- Source precedence recorded per library, with the version-mismatch label
  present on any doc served for a version the project does not pin (S9).
- No URL is ever derived from a package name, verified by test.
