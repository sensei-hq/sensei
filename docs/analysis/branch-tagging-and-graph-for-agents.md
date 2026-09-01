# Branch tagging, versioning, and what the graph owes coding agents

**Status:** analysis, no code moved. Written 2026-09-01 from live measurements
against the local `sensei` database (430,885 nodes · 715,985 edges · 7,772
folders · 1,121 libraries) and the current source.

Four questions were asked. They turn out to have different answers, and two of
them are answered by facts rather than by design preference.

1. Should a branch switch tag rather than re-walk?
2. Branch-tagged storage vs live-only vs baseline+diff — which?
3. Should libraries be versioned (latest + 2)?
4. **How do we make the graph better for coding agents?** — the primary one.

---

## 1. Measured state

### The graph is mostly unresolved, and it is unresolved unevenly

```
edge kind      total     resolved    %
calls        320,542     207,780   64.8
references   249,979           0    0.0
imports      136,484           0    0.0
extends        7,862           0    0.0
covers           601         601  100.0
─────────────────────────────────────────
overall      715,468     208,381   29.1

nodes.resolved            153,855 / 430,771   (35.7%)
nodes with any tag           1,685 / 430,771   ( 0.4%)
```

This decomposition matters more than the 29.1% headline (#141). **`calls` works
at 64.8%.** Three edge kinds — 394,325 edges, 55% of the graph — are written and
**never resolved at all**. An agent asking "who calls this" gets a usable answer.
An agent asking "what imports this", "what references this", or "what extends
this" gets nothing, from 394k stored rows.

### There is no branch or version dimension anywhere

`sensei.nodes` has no branch, commit, or version column. It has `tags text[]`,
used on 0.4% of rows. `sensei.scan_state` is keyed `(folder_id, file_path)` and
carries `mtime` + `content_hash`. So the graph represents exactly one state: the
working tree as last indexed.

### The incremental gate already handles a branch switch well

`process_git_folder` runs a two-tier gate: skip on unchanged `mtime` without
reading, re-hash only mtime-drifted candidates, and reindex only genuine content
changes. The source names our exact case:

```rust
// (mtime drift with no content change: touch, checkout, branch-switch-to-same.)
```

So the cost of a branch switch is already proportional to **changed files**, not
to the tree. `content_hash` is already the dedup key a baseline scheme would
introduce.

### But the reconcile is scoped to the wrong thing

`root_watcher.rs:389` — on `.git/HEAD` changing:

```rust
if RootWatcher::is_branch_switch(&path) {
    if let Some(root) = watch_root_for_path(&path, &roots) {
        enqueue_scanroot_reconcile(&rt, &queue, vec![root]);   // ← the WATCH ROOT
    }
}
```

A branch switch in one repository enqueues a `ScanRoot` for the **entire watch
root**. On this install `/Users/Jerry/Developer` holds 67 repositories, so
switching a branch in one of them triggers a folder-discovery walk and a
per-folder stat sweep across all 67.

That is the real cost in #130, and it is a **scoping bug, not a reindexing
problem**. The per-file work is already right.

---

## 2. Does branch tagging matter, or is live enough?

The question was asked directly, and the measurements answer it.

### What an agent actually asks for

An agent works in the working tree. Its questions are "what calls this", "where
is this defined", "what will I break". All of them are questions about **the code
that is checked out right now** — which is what live-only storage already
represents, correctly, with no extra dimension.

The questions branch-tagged storage would newly answer — "what does `main` look
like", "how does this differ on the other branch" — are answerable by `git` in
one command, and are rare in agent work. Sensei would be duplicating a store git
already is.

### The decisive argument is the resolution rate

**Adding a dimension to a graph that is 71% unresolved makes the unresolved
problem worse.** Every branch tag multiplies the rows that resolution has to
work over, and resolution is the thing that is failing. Branch recall is a
feature nobody is blocked on; `imports` at 0% is blocking every agent question
about dependencies.

Concretely: `references` + `imports` + `extends` = 394,325 rows that cost storage
and index maintenance and answer nothing today. That is already more dead weight
than a second branch tag would add. Fix the dead weight first.

### Baseline + diff

Elegant, and aimed at the wrong target. It optimises re-indexing cost across
branches — but `content_hash` already makes an unchanged file free (a stat, no
read), so the cost it would save is close to zero. It would buy that near-zero
saving in exchange for a much harder correctness story: which layer owns a node's
identity, what happens to a `calls` edge whose source is baseline and target is
diff, and how a stale baseline is detected.

### The `live` + `current` distinction is real, but it is not storage

The observation that "live = committed + dirty" is right, and worth surfacing —
an agent should know a file it is reading has uncommitted changes. But that is a
**per-file flag** (`dirty: bool`, derivable from `git status`), not a second copy
of the graph. One boolean on `scan_state` answers it.

### Recommendation

**Stay live-only.** Do not add a branch dimension. Instead:

- scope the branch-switch reconcile to the repository whose HEAD moved (#130 as a
  one-line-ish fix), and
- spend the saved effort on resolution, which is what agents are actually
  blocked on.

Revisit tagging only if a measured need appears — an agent workflow that
demonstrably needs cross-branch recall. Right now none exists.

---

## 3. Libraries: versioning is blocked, and premature

### The constraint

```
sensei.libraries
  UNIQUE (ecosystem, name)     ← one row per library, period
  version text                 ← a column, not part of the key
```

So "keep latest + 2" is **not expressible today**. The key would have to become
`(ecosystem, name, version)`, and every consumer that resolves a library by name
would need to say which version it means. That is a real schema and API change,
not a config flag.

### But the corpus does not justify it yet

```
libraries                       1,121
  with docs ingested                2   ← rokkit (94 pages), dbd (36)
  with page_count = 0            1,119
total pages                       130
```

**Two libraries out of 1,121 have any documentation at all.** Versioning a corpus
that barely exists optimises the wrong end. Ingestion coverage is the bottleneck.

### The manifest gateway is the cheap, high-value part

`sensei.library.json` exists in all three sibling repos and declares exactly what
an agent wants — skills, agents, an llms corpus, install commands:

```json
{ "library": "rokkit", "version": ">=1.3",
  "skills": [5], "agents": [3],
  "llms": { "path": "docs/llms", "index": "/llms/index.txt" },
  "install": { "skills": "rokkit skills add <name>", ... } }
```

The tables (`sensei.library_skills`, `sensei.library_agents`), the parser
(`libraries/manifest.rs`) and the MCP tools (`list_library_skills`,
`get_library_skill`, `list_library_agents`) **already exist and work** — I called
`list_library_skills('rokkit')` and it returned skill bodies.

What is missing is coverage and identity:

| library | manifest declares | ingested | version |
|---|---|---|---|
| rokkit | 5 skills · 3 agents | **4 · 2** (stale) | `>=1.3` on each capability row |
| dbd | 1 skill · 1 agent | **0 · 0** | — |
| kavach | 4 skills · 2 agents | **0 · 0** | — |

`libraries.local_path` is empty for all 1,121 rows, so nothing knows where any
library lives on disk, so no manifest can be re-read.

Three defects, all small:

1. **dbd and kavach manifests are never ingested** despite being on disk — and
   dbd already has 36 doc pages, so it is otherwise a known library.
2. **rokkit lost one skill and one agent** (4/5, 2/3). Silent partial ingestion.
3. ~~The manifest's `version` is dropped.~~ **Wrong — corrected 2026-09-01.**
   It is stored as `library_skills.version_range` / `library_agents.version_range`
   (`>=1.3` on all six rokkit rows). `libraries.version` is the REGISTRY version
   (what is installed), which is a different fact and correctly NULL until a
   registry check runs. I conflated the two.

   The real third defect is worse: **nothing can re-ingest a manifest.**
   `libraries.local_path` is empty for **all 1,121 rows**, and manifest ingestion
   runs only inside `index_library`, only when its transient `source` is a
   `LocalDir`. So the path a manifest lives at is never recorded, and rokkit's two
   newest entries (`charts-rokkit`, `rokkit-chart-reviewer` — both present on
   disk) were simply never picked up. That is the cause of the 4/5 and 2/3: stale
   ingestion, not failed body resolution.

There is also an identity split worth deciding deliberately: the manifest says
`library: "rokkit"`, while dependency detection produces `@rokkit/actions`,
`@rokkit/app`, `@rokkit/ui`, … as separate rows. Both exist. Nothing links them,
so an agent that knows it depends on `@rokkit/ui` cannot reach rokkit's skills.

**That link is probably the single highest-value library fix**: it is what turns
"this project uses @rokkit/ui" into "here are the four curated skills for it".

---

## 4. The primary question: making the graph better for agents

Ranked by measured impact, not by how interesting the work is.

### 4.1 Resolve `imports` (136,484 edges, 0%)

The highest-value single change. Import resolution is how an agent answers "what
does this file depend on" and "what will break if I change this" — the two
questions that most often precede an edit. It is also the *easiest* kind to
resolve: an import names a module path explicitly, unlike a call which may be
dynamic.

### 4.2 Resolve `references` (249,979 edges, 0%)

The largest single block of dead rows. "What reads this symbol" is the question
`calls` cannot answer for types, constants and fields.

### 4.3 Close the loop that #141 opened

`get_callers` missed a caller added hours earlier in the same session. With
`calls` at 64.8% that is expected, not surprising — but it means the MCP-first
rule in `CLAUDE.md` currently asks agents to prefer a tool that is right about
two calls in three. Either raise resolution to where the preference is earned, or
narrow the rule to the questions the graph answers well. Silently falling back to
grep — which is what happened — is the outcome to avoid.

### 4.4 Say when an answer is partial

An agent cannot tell a complete answer from a truncated one. `get_callers`
returning three callers is indistinguishable from "there are three callers" and
"there are eight, five of which are unresolved". Attaching the unresolved count
to the response would let an agent know to check with grep — a small change that
converts a silent wrong answer into a known-incomplete one. This is the same
principle the metric reason codes now follow: name the state, do not imply one.

### 4.5 Surface dirtiness

One boolean per file, from `git status`. An agent reading a file with uncommitted
changes should know that the committed history it is reasoning about does not
match what it is looking at.

---

## 5. Recommended sequence

Forward-only; no step depends on a later one.

1. **Scope the branch-switch reconcile to the repository** whose HEAD moved, not
   the whole watch root (#130). Smallest change, immediate effect, no schema.
2. **Ingest every `sensei.library.json`**, keep the declared version, and fix the
   partial rokkit ingestion. Then link detected packages (`@rokkit/*`) to their
   manifest library so a dependency resolves to its skills.
3. **Resolve `imports`**, then `references`. This is the work that makes the graph
   worth preferring over grep.
4. **Report unresolved counts** on `get_callers`/`get_callees` so a partial answer
   announces itself.
5. **Then** revisit library versioning — once more than two libraries have docs,
   the key change has something to be right about.
6. **Do not add branch tagging.** Not now, and not until a workflow measurably
   needs cross-branch recall.

## 6. What I am least sure about

- **Whether `references` is worth resolving at all**, or whether it should be
  dropped. 249,979 rows that have never resolved may be a signal that the
  extractor emits more than the resolver was ever designed to match. Measuring
  what fraction *could* resolve would settle it, and that measurement should
  precede the work.
- **Whether the identity split is one library or two.** A monorepo publishing
  eight packages under one manifest is genuinely both. The link may be a mapping
  table rather than a merge.
