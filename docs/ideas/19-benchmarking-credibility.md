---
name: Benchmarking & Credibility
description: Competitive comparison, reproducible benchmarks on known repos, community-contributed results, and honest limitation documentation
date: 2026-04-17
status: idea
related: 07-metrics-analytics.md
---

# Benchmarking & Credibility

## Problem

The AI tooling space is full of unproven claims. MemPalace boasted 97% recall — then users found basic installation broken and numbers dropped. Zencode has good ideas but questionable implementation. Goose was a UX disaster. Every tool claims to be revolutionary; none provides reproducible evidence.

Sensei should not make claims without proof. We need a benchmark system that:
- Runs Claude on a repo with and without sensei, captures metrics
- Uses a corpus of known, public repos (not cherry-picked examples)
- Is reproducible — anyone can run the same benchmark and verify
- Is honest — documents cases where sensei doesn't help or makes things worse

## Competitive landscape

### Comparison matrix

Track tools across categories. Honest assessment — what they do well, what they don't, and where sensei fits.

| Tool | Category | Strengths | Weaknesses | Sensei differentiator |
|------|----------|-----------|------------|----------------------|
| **Graphify** | Code graph indexing | Good structural analysis, community detection, god node analysis | No workflow integration, no pattern enforcement, no metrics | Sensei uses the graph to guide the AI, not just visualize it |
| **CocoIndex** | Code indexing | Semantic search, embeddings | Index-only, no feedback loop, no session continuity | Sensei goes beyond indexing to enforcement and measurement |
| **MemPalace** | LLM memory | Persistent memory across sessions | Plugin installation issues, inflated recall claims, basic operations broken | Sensei uses documents + guardrails (file-based, verifiable) not opaque memory stores |
| **Redis LangCache** | LLM caching | Fast semantic cache, reduces API calls | Cache-only, no quality feedback, no pattern awareness | Sensei caches context intelligently (resolution levels), not just responses |
| **Chub** | Library documentation | Fetches and indexes library docs | Docs-only, no usage pattern extraction, no enforcement | Sensei extracts library usage patterns, not just API reference |
| **Zencode** | AI coding workflow | Good concept — structured AI development | Implementation quality issues, UX problems | Sensei is command-driven (not a separate UI), embeds in existing tools |
| **Goose** | AI coding agent | Autonomous coding | UX disaster, hard to control, no feedback loop | Sensei keeps the human in the loop — AI is guardian, not autonomous agent |
| **Cursor** | AI code editor | Great inline completion, fast | Proprietary, no workflow structure, no metrics | Sensei works WITH Cursor (multi-coordinator), adds workflow + measurement |
| **Aider** | CLI AI coding | Good git integration, simple UX | No pattern awareness, no metrics, no desktop visualization | Sensei adds the intelligence layer on top |

> **Rule: never publish a comparison cell we haven't verified.** If we haven't tested a tool, mark the cell as "unverified" and note the source of our assessment.

### What to track per competitor

For each tool, maintain:
- **Last evaluated**: date of our assessment
- **Version evaluated**: specific version
- **How evaluated**: installed and tested vs. reviewed docs vs. user reports
- **Honest strengths**: what they genuinely do well
- **Honest weaknesses**: what doesn't work, verified by testing
- **User sentiment**: community feedback (GitHub issues, Discord, Reddit)

---

## Benchmark system

### Corpus: reference repos

A registry of known, public repos that cover different stacks, sizes, and complexities. Anyone can add a repo.

| Repo | Stack | Size | Complexity | Why included |
|------|-------|------|-----------|-------------|
| karpathy/nanoGPT | Python, ML | Small | Medium | Well-known, clean structure, ML patterns |
| sveltejs/kit | TypeScript, Svelte | Large | High | Framework internals, monorepo, complex architecture |
| tokio-rs/tokio | Rust | Large | High | Systems code, async patterns, trait-heavy |
| shadcn-svelte | TypeScript, Svelte | Medium | Medium | Component library, UI patterns |
| (sensei itself) | Rust + TypeScript | Medium | High | Dog-fooding — we benchmark ourselves |
| (community repos) | Various | Various | Various | Community-contributed via registry |

**Registry format:**
```yaml
# benchmarks/registry.yaml
repos:
  - name: karpathy/nanoGPT
    url: https://github.com/karpathy/nanoGPT
    stack: [python, pytorch]
    size: small
    patterns: [ml-training, config-driven, modular]
    added_by: sensei-team
    verified: true

  - name: community/example-repo
    url: https://github.com/user/repo
    stack: [typescript, svelte]
    size: medium
    patterns: [component-library, monorepo]
    added_by: community
    verified: false  # not yet verified by sensei team
```

### Task corpus

Predefined tasks to run on each repo. Tasks are categorized by type and difficulty.

```yaml
# benchmarks/tasks.yaml
tasks:
  - id: add-feature-simple
    description: "Add a new CLI flag that enables verbose logging"
    type: feature
    difficulty: easy
    measures: [ftr, turn_count, pattern_adherence]

  - id: add-feature-pattern
    description: "Add a new language adapter following the existing adapter pattern"
    type: feature
    difficulty: medium
    measures: [ftr, turn_count, pattern_adherence, locate_accuracy]
    requires_pattern: true  # only applicable to repos with detected patterns

  - id: fix-bug-with-tests
    description: "Fix the failing test in module X and ensure no regressions"
    type: bug
    difficulty: easy
    measures: [ftr, turn_count]

  - id: refactor-complex
    description: "Refactor the highest-complexity function to reduce cyclomatic complexity below 10"
    type: refactor
    difficulty: hard
    measures: [ftr, turn_count, testability_improvement, rework_rate]

  - id: understand-codebase
    description: "Explain the architecture of this repo — main components, data flow, key patterns"
    type: comprehension
    difficulty: medium
    measures: [accuracy, completeness, turn_count]
```

### A/B methodology

For each (repo, task) pair, run twice:

**Without sensei (control):**
- Fresh Claude Code session, no sensei plugin
- No MCP, no hooks, no guardrails, no workflow commands
- Just the AI and the codebase
- Capture: turns taken, files modified, test results, time elapsed

**With sensei (treatment):**
- Fresh Claude Code session, sensei plugin active
- Full workflow: session start → locate → decompose → test → implement → review
- Capture: all sensei events + same control metrics

**Comparison metrics:**

| Metric | Definition | Better = |
|--------|-----------|----------|
| FTR | Task completed without user correction | Higher |
| Turn count | User ↔ AI turns to complete task | Lower |
| Locate accuracy | Did the AI find the right files on first try? | Higher |
| Pattern adherence | Did the AI follow existing patterns? | Higher (with sensei) |
| Rework count | Times the AI had to redo work | Lower |
| Test coverage | Tests written vs. code changed | Higher |
| Time to complete | Wall clock time | Lower (ideally) |

### Honesty: when sensei doesn't help

Document and publish cases where sensei adds overhead without benefit:

- **Tiny changes**: Adding a one-line fix doesn't need a workflow. Sensei's locate step adds turns without value.
- **New empty repos**: No patterns to detect, no graph to query. Sensei's intelligence layer has nothing to work with.
- **Non-code tasks**: Writing docs, config changes, DevOps scripts — workflow commands add friction.
- **Speed-critical prototyping**: When the user explicitly wants fast, messy code. Sensei's TDD and review steps slow this down.

These should be published alongside positive results. Credibility comes from honesty, not cherry-picking.

---

## Community contribution model

### How users contribute

1. **Add a repo**: PR to `benchmarks/registry.yaml` with repo URL, stack, patterns
2. **Add a task**: PR to `benchmarks/tasks.yaml` with task description, type, measures
3. **Run a benchmark**: `sensei benchmark run --repo <url> --task <id>` → produces JSON results
4. **Submit results**: PR to `benchmarks/results/` or auto-submit to sensei API

### How results are published

- **Static website**: aggregated results per repo, per task, with/without sensei
- **Per-repo page**: detailed breakdown for each benchmark repo
- **Community results**: tagged with contributor, marked verified/unverified
- **Trend over time**: as sensei improves, re-run benchmarks to show progress

### Trust model

| Result type | Trust level | Display |
|-------------|------------|---------|
| Sensei team, verified | High | Published with full methodology |
| Community, verified by team | Medium | Published with verification note |
| Community, unverified | Low | Published with "community-submitted, unverified" badge |
| Self-reported (no raw data) | None | Not published — require reproducible evidence |

---

## Benchmark CLI

```bash
# Run a benchmark
sensei benchmark run --repo karpathy/nanoGPT --task add-feature-pattern

# Run all tasks on a repo
sensei benchmark run --repo karpathy/nanoGPT --all

# Run without sensei (control)
sensei benchmark run --repo karpathy/nanoGPT --task add-feature-pattern --control

# Compare results
sensei benchmark compare --repo karpathy/nanoGPT --task add-feature-pattern

# Submit results
sensei benchmark submit --results benchmarks/results/nanogpt-add-feature.json
```

---

## Open questions

| # | Question |
|---|----------|
| 1 | How do we ensure benchmark reproducibility? LLM responses are non-deterministic. Run N times and report mean ± stddev? |
| 2 | How do we prevent gaming? (Optimizing sensei for benchmark tasks rather than real-world usage) |
| 3 | Should benchmarks use a specific Claude model version, or test across models? |
| 4 | How often do we re-run benchmarks? On every sensei release? Monthly? |
| 5 | Can the benchmark system itself be used as a CI/CD quality gate for sensei development? |
| 6 | How do we benchmark the desktop experience? Automated Playwright tests against desktop views? |
