---
name: Workflow System — Configuration
description: Workflow recipes, two-layer config model, and per-command behavior overrides
date: 2026-04-17
parent: 01-workflow-system.md
---

# Configuration

## Design philosophy

Commands are the product. Workflows are suggestions. Everything is configurable.

Users can invoke any command at any time — the command sets the active phase regardless of what recipe they started with. Recipes exist to help new users get started, not to constrain experienced ones.

---

## Workflow Recipes

Recipes are **onboarding templates**, not enforced pipelines.

| Recipe | Suggested sequence | Best for |
|--------|-------------------|----------|
| **Full** | All phases, strict gates | Production features, team projects |
| **Lean** | Analyze → Plan → Build → Validate | Known problems, clear solutions |
| **Exploratory** | Ideate → Experiment → (loop) | R&D, prototyping, "I don't know what I want yet" |
| **Maintenance** | Analyze → Build → Validate | Bug fixes, dependency updates, cleanup |
| **Custom** | User-defined subset | Per-project configuration |

---

## Two-layer config model

1. **Global** (`~/.sensei/config.yaml` or equivalent) — user's default preferences across all projects
2. **Project** (`.sensei/config.yaml` or equivalent) — overrides for a specific repo

Project config takes precedence over global config. Missing keys fall back to global, then to sensei defaults.

---

## Per-command configuration

Each phase command has a behavior template that the user can override.

```yaml
# Example: user configures how /sensei:idea works for them
commands:
  idea:
    output_dir: "docs/ideas"        # default; user might prefer "specs/concepts"
    naming: "{name}.md"             # default; user might prefer "01-{name}.md"
    template: "idea.md"             # which template to use
    ask_questions: true             # AI asks structured questions before writing
    max_depth: "problem-space"      # don't go into solution design

  analyze:
    output_dir: "docs/analysis"
    naming: "{name}.md"
    requires_prior: []              # no mandatory prior phase (user can jump in)
    no_code: true

  blueprint:
    output_dir: "docs/blueprints"
    naming: "{name}.md"
    requires_prior: ["analysis"]    # must have an analysis doc first (optional gate)
    no_code: true                   # enforce no code generation in this phase

  experiment:
    output_dir: "docs/experiments"
    naming: "{name}.md"
    branch_prefix: "experiment/"    # git branch naming

  plan:
    output_dir: "docs/plans"
    naming: "{name}.md"
    requires_prior: ["blueprint"]

  build:
    tdd: true                       # always write tests first
    feature_branch: true            # create branch per feature
    patterns_file: "PATTERNS.md"    # load before coding
    review_on_complete: true        # auto-trigger /sensei:review after each feature

  review:
    strictness: "advisory"          # "strict" | "advisory" | "minimal"
    checks:                         # which checks to run
      - duplication
      - pattern-drift
      - test-coverage
      - doc-accuracy
      - solid-violations
```

---

## What this means in practice

- User A likes `docs/ideas/` with numbered prefixes — they configure `naming: "01-{name}.md"`
- User B jumps straight to blueprints and calls them "RFCs" in `docs/rfcs/` — they configure `output_dir: "docs/rfcs"`
- User C doesn't use idea/analyze phases at all — those commands still work if invoked, but the workflow recipe doesn't suggest them
- User D wants strict quality gates — they set `review.strictness: "strict"`
- Sensei provides sensible defaults; the user tunes to taste
