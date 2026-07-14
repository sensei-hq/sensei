# 拠 · Pipeline · Benchmarks & credibility

**Owner files:**
- Benchmark runner: `crates/senseid/src/benchmarks/runner.rs`
- Corpus registry: `benchmarks/registry.yaml` (in-repo)
- Comparison matrix: `benchmarks/competitors.yaml`

**Companion design doc:** `docs/archive/ideas/19-benchmarking-credibility.md`.

## Purpose

The AI tooling space is full of unproven claims. Sensei must
not make claims without proof. This pipeline runs reproducible
benchmarks on known public repos and records the results —
sensei's own effect on FTR, corrections, and rework compared
against the same assistant working on the same repo without
sensei.

Two products:

1. **Benchmark runs** — a corpus of public repos plus a small
   scripted set of tasks per repo. Runs the assistant once with
   sensei enabled and once without; records the delta.
2. **Competitive landscape** — an honest, verified comparison
   against alternatives (Graphify, CocoIndex, MemPalace, Cursor,
   Aider, etc.). Never publish a cell we haven't verified.

Kanji is 拠 — *evidence*.

## Data invariants

### Corpus

A registry of public repos:

    # benchmarks/registry.yaml
    repos:
      - name: karpathy/nanoGPT
        url: https://github.com/karpathy/nanoGPT
        stack: [python, pytorch]
        size: small
        patterns: [ml-training, config-driven, modular]
      - name: tokio-rs/tokio
        stack: [rust]
        size: large
        patterns: [systems, async, trait-heavy]
      - name: (sensei itself)
        stack: [rust, typescript]
        size: medium
        patterns: [multi-language, adapter, workflow-engine]
      # community-contributed via PR
      - name: community/example-repo
        …

### Task set per repo

- **Standard tasks** — a small set common to every repo
  (understand-the-code / add-a-test / fix-a-bug / write-a-doc).
- **Repo-specific tasks** — one or two tasks curated per repo
  that showcase realistic work.

### Run record

- `sensei.benchmark_runs`:
  - `id`, `repo`, `task`, `assistant_family`, `assistant_model`,
    `sensei_enabled` bool, `session_id`, `ftr` bool,
    `corrections` int, `duration_ms`, `notes`, `ran_at`.
- A comparison group = the same (repo, task, assistant, model)
  with `sensei_enabled` on and off.

### Competitive matrix

    # benchmarks/competitors.yaml
    tools:
      - name: Graphify
        category: code-graph-indexing
        last_evaluated: 2026-04-17
        version_evaluated: 0.4.2
        how_evaluated: installed
        strengths: [...]
        weaknesses: [...]
        sensei_differentiator: "…"

**Rule:** never publish a comparison cell that hasn't been
verified. If we haven't tested a tool, mark the cell
"unverified" and note the source of our assessment.

## Signals produced

| Signal | Consumer |
|---|---|
| Benchmark run records | Website `/benchmarks` page |
| Aggregate FTR deltas | Website hero ("sensei lifted FTR by X on Y repos") |
| Competitive matrix | Website `/compare` page |
| Regression detection on our own numbers | Impact — a self-benchmark going down triggers a Now-column card |

## Done gate

- The corpus includes at least 5 public repos + sensei itself.
- Runs are reproducible: a fresh clone + the benchmark command
  produces the same results within noise threshold.
- The website exposes the raw run data (no cherry-picking).
- The competitive matrix marks unverified cells explicitly.
- Community can contribute a repo to the corpus via PR.
- Our own regression detector fires when a sensei code change
  drops the aggregate FTR delta.

## Wrong gate

- **A benchmark cell in the website has no run record backing
  it.** Fabrication guard failed.
- **Only cherry-picked repos in the corpus.** Bias visible.
- **Competitor cell says "worse than sensei" without a run
  record.** Claim without proof.
- **A published number is > 1 month old.** Stale numbers hurt
  credibility more than none.
- **Sensei's own aggregate drops but no self-regression card
  fires.** Detector missing.

## Related

- [[pipeline/impact]] — verdict measurement mechanics
- [[pipeline/ftr]] — the metric measured
- [[pipeline/analyzer]] — enrichment for benchmark sessions
- (memory: project_website_hub_shipped) (memory) — where the numbers
  publish
- (archive: ideas/19-benchmarking-credibility.md) — source design
