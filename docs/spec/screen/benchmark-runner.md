# 較 · Benchmark runner

**Segment:** 02 · Preferences / Extensions
**Route:** `/instruments/benchmark` (Dashboard, layout A) · `/instruments/benchmark/[runId]` (Notebook run report, layout B)
**Source mockup:** [`lib/observatory/benchmark.jsx`](../../mockups/Sensei/lib/observatory/benchmark.jsx) → `BenchmarkRunnerDashboard` (A) + `BenchmarkRunnerNotebook` (B)
**Data:** `sensei.benchmark_runs` (paired same-task/branch, `sensei_enabled` true/false) · `sensei.benchmark_reports` (strategy-comparison payloads). Corpus registry in-repo at `benchmarks/registry.yaml`.
**App file:** _greenfield_
**Daemon files:** _partial / mostly greenfield_ — DDL exists (`database/ddl/table/sensei/benchmark_runs.ddl`, `benchmark_reports.ddl`, staging `import_benchmark_reports.ddl`). The designed owner files (`crates/senseid/src/benchmarks/runner.rs`, `benchmarks/registry.yaml`, `benchmarks/competitors.yaml`) **do not exist yet**; there is no HTTP handler and no `sensei bench` CLI command.
**Status:** **deferred / greenfield** — [[pipeline/benchmarks]] is a deferred pipeline. The tables are declared but empty in practice; the runner, corpus registry, CLI, and website surfaces are unbuilt. **Do not build the run engine from this spec.** This documents the two mockup layouts so the UI is ready when the pipeline lands, and pins the honesty stance that governs both.

## Purpose

The AI-tooling space is full of unproven claims; **sensei must not make claims
without proof** ([[pipeline/benchmarks]]). The benchmark runner runs an
assistant against a corpus of tasks **twice — once without sensei's tools,
memory, and MCPs (run A), once with them fully active (run B) — same model,
same tasks. The diff is the value.** The runner is where a user configures and
reads those paired runs.

Two layouts, one surface:

- **A · Dashboard** (`BenchmarkRunnerDashboard`) — the operator view: a corpora
  list, a recent-runs table, a single-run detail card (A-vs-B, delta strip,
  per-task table), and a "New run" launcher.
- **B · Lab notebook** (`BenchmarkRunnerNotebook`) — a single run as a
  long-scroll narrative report: Abstract → Setup → Headline numbers → Per-task
  results → **Where sensei made the difference** *and where both still failed*
  → Reproduce. This is the shareable, publishable artifact.

**Honesty / negative-results stance (non-negotiable, from [[pipeline/benchmarks]]):**
publish where sensei *doesn't* help. Every published cell needs a run record
behind it; unverified is marked unverified; a task where both A and B failed is
shown, tagged as a follow-up candidate — not hidden. The notebook's "Where both
still failed" block exists precisely to carry the negative result.

Kanji is 較 — *comparison*.

## Data invariants

- A **run record** is `sensei.benchmark_runs`: `folder_id`, `task_description`,
  `branch`, **`sensei_enabled` bool**, timing, and token/cost aggregates. A
  **comparison group = the same (folder, task, branch) with `sensei_enabled`
  on and off** — A is `sensei_enabled=false` (baseline), B is `true` (assisted).
- **Every displayed number traces to a run record.** No cell — dashboard or
  notebook — may show a score, delta, or verdict without a backing row. This is
  the fabrication guard; it is the primary wrong-gate.
- A **corpus** is a repo with a `/tasks` folder; the in-repo registry lists
  public repos (plus sensei itself). `kind` is `public` or `private`; private
  corpora are labelled as such and never published externally.
- **Deltas are derived, never stored as claims:** `delta = B - A` per axis
  (passed, score, tool calls, tokens). Fewer tool calls / fewer tokens is an
  *improvement* (the mockup inverts the sign colour for those axes).
- **Negative results are first-class.** A task where both runs failed (`a:fail,
  b:fail`) is shown in the per-task table and surfaced in the notebook's "Where
  both still failed" block, tagged as a corpus follow-up candidate.
- **Staleness is a defect:** a published number older than one month hurts
  credibility more than none ([[pipeline/benchmarks]] wrong-gate). The report
  carries its `ran_at`.
- Reproducibility: the notebook shows the exact command; a fresh clone + that
  command reproduces the result within a noise threshold.

## Signals shown

### Dashboard (layout A)

Hero (`BnHero`): `{corpora count}` corpora · `{runs count}` runs · `+{avg score
lift}%` (accent).

Corpora list:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Corpus card | `c.label` + `c.kind` chip + `c.repo` + `{tasks} tasks · {langs} · {lastSync}` | one repo-with-tasks; `private` = warning colour, `public` = success | `SWE-bench Lite (subset)` · public · `sensei-hq/swe-bench-lite-tasks` · 24 tasks · python · ran weekly · 2 days ago |
| + import corpus from repo | dashed button | add a corpus by repo | — |

Recent runs list:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Run row | `r.corpus · r.id` + `{started} · {duration}` + `{b.passed}/{b.total}` + `{delta.passed}` | one run; delta green when `> 0` | `rust-refactor · run-12` · today 09:14 · 47m · 16/18 · +5 |

New-run launcher:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Run benchmark | button `({tasks} tasks · ~{ceil(tasks*2.5)}m)` | executes each task twice — first without sensei, then with sensei + MCPs | `Run benchmark (18 tasks · ~45m)` |

Run detail (right, for the selected run):

| Element | Value | Meaning | Example |
|---|---|---|---|
| Verdict | `run.verdict` | one-line human summary | "sensei wins: 5 more passes, 39% fewer tool calls." |
| A vs B cards (`BnRunCard`) | side `A`/`B` + label + passed/total + score% + tool calls + tokens | baseline vs assisted, B highlighted accent | A `claude-sonnet-4.5 · no sensei` 11/18 61% · B `… · sensei` 16/18 89% |
| Delta strip (`BnDelta` ×4) | passed · score · tool calls · tokens | `B - A`; tool-calls/tokens inverted (down = good) | +5 · +28% · −56 · −42k |
| Per-task table | id · task · without · with · note | per-task pass/fail with the reason | `t01` · Reduce render-thrash in `<Toolbar>` · fail · pass · "Sensei skill 'react-perf-watch' triggered." |

### Notebook (layout B)

`NbBlock` sections, in order:

| Block | Content | Example |
|---|---|---|
| Abstract | corpus + model + "run A (disabled) vs run B (active)" + `run.verdict` | "We executed the Rust refactor corpus (18 tasks) twice with claude-sonnet-4.5…" |
| Setup · what changed | A (baseline: bare assistant, no MCPs, no memory) vs B (all extensions, memories on context match, fallback chain + MOE, named MCPs) | B MCPs: tsserver, fs-read, react-devtools, session-replay |
| Headline numbers | same 4 `BnDelta` tiles + one prose line | "more passes with fewer tool calls and fewer tokens" |
| Per-task results | id · task · A · B · commentary | full `taskBreakdown` |
| **Where sensei made the difference** | left: tasks won by skill/agent triggers · **right: where both still failed** | won: t01 react-perf-watch, t04 migration-runner · failed: t06 borrow-check in canvas/event.rs — no memory exists yet, tagged candidate |
| Reproduce | the exact `sensei bench run …` + `sensei bench resume {id}` commands | `sensei bench run --corpus … --baseline none --variant default` |
| Footer actions | Re-run on latest sensei · Export markdown · Share with collective → | — |

## Done gate

> These gates apply **when the benchmark pipeline is built**. Until then the
> gate is: the two layouts render from `EXT_DATA.benchmark` seed data and carry
> the honesty affordances (negative-results block, verified-only cells).

- Every dashboard number and every notebook cell traces to a `benchmark_runs`
  row — no orphan cells (fabrication guard).
- A run is a **pair**: A (`sensei_enabled=false`) and B (`true`) on the same
  (folder, task, branch); the A-vs-B cards and delta strip are computed from the
  pair, not entered by hand.
- The per-task table shows **every task, including both-failed rows**; the
  notebook's "Where both still failed" block lists them tagged as corpus
  candidates.
- Private corpora are labelled `private` and excluded from any external publish.
- Deltas colour tool-calls/tokens as improvements when they go *down*.
- The notebook's Reproduce command actually reproduces the run within noise on a
  fresh clone.
- A report exposes its `ran_at`; a number older than a month is flagged stale,
  not shown as current.
- "Run benchmark" launches the paired run in the background with progress — it
  does not block the UI.

## Wrong gate

- **A cell has no run record behind it.** Fabrication guard failed — the cardinal
  sin of [[pipeline/benchmarks]]. Applies to every score, delta, and verdict.
- **Only wins are shown; both-failed tasks are hidden.** The negative-results
  stance is violated; the "Where both still failed" block is the required
  counterweight and must not be omitted.
- **A vs B are not the same task/branch/model.** The comparison isn't
  controlled — the only variable must be `sensei_enabled`.
- **A published number is stale (> 1 month).** Stale numbers hurt credibility
  more than none.
- **Corpus is cherry-picked** (only repos where sensei wins). Bias visible; the
  corpus must include a range plus sensei itself.
- **A "sensei wins" verdict with no backing pair.** Claim without proof.
- **Delta sign is wrong** — more tool calls shown as an improvement, or a token
  increase coloured green.
- **Someone builds the run engine straight from this spec.** The pipeline is
  deferred; the runner (`benchmarks/runner.rs`), registry, and CLI are unbuilt.
  This spec is UI + honesty-stance only until [[pipeline/benchmarks]] is picked up.

## Related

- [[pipeline/benchmarks]] — the deferred pipeline this surfaces; owns the invariants + honesty rules
- [[pipeline/impact]] — a self-benchmark regression fires a Now-column card
- [[pipeline/ftr]] — the metric the benchmark measures the delta on
- [[screen/observatory-impact]] — where regression on our own numbers surfaces
- (memory: project_website_hub_shipped) — where aggregate numbers publish (`/benchmarks`, `/compare`)
