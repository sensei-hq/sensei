---
name: Extensible Mindsets & Personas
description: Process mindsets (how to work) + persona mindsets (who we're building for) — configurable per project, used for design and validation
date: 2026-04-18
status: idea
related: 01-workflow-system.md, 18-testability-tdd.md
---

# Extensible Mindsets & Personas

## Problem

The three core mindsets (analyst, developer, BAT) guide HOW to work. But projects also need to consider WHO they're building for. A healthcare app has patients, doctors, and admins — each with different needs. A developer tool has end users, plugin authors, and API consumers. Without persona awareness, the AI builds for an abstract "user" instead of specific people with specific journeys.

## Two dimensions

| Dimension | What it answers | Scope | Examples |
|-----------|----------------|-------|---------|
| **Process mindsets** | How should I approach this work? | Universal (plugin) | Analyst, developer, BAT, UX designer, security reviewer |
| **Persona mindsets** | Who am I building for? What do they need? | Project-specific | End user, admin, API consumer, plugin developer |

## Core process mindsets (universal)

Already built:
- **Analyst** — understand the problem before designing
- **Developer** — understand the implementation before coding
- **BAT** — verify from user's perspective

To add:
- **UX Designer** — is the interface intuitive, accessible, consistent? Does the journey flow?
- **Security Reviewer** — what can go wrong? Validation, auth, data exposure, injection?
- **Performance Engineer** — what's the cost? Complexity, memory, network? Scale?
- **DevOps/SRE** — deployable, monitorable, rollback-safe? What breaks at 3am?

## Project personas (per repo)

Defined in `.sensei/personas.yaml` or a section in rules:

```yaml
personas:
  - name: end-user
    description: "Non-technical user accessing the dashboard"
    goals: ["See quality trends", "Get actionable advice"]
    pain_points: ["Too much jargon", "No clear next steps"]
    validates: ["Is language plain?", "Is there a call to action?"]

  - name: plugin-developer
    description: "Developer creating sensei skills"
    goals: ["Create a skill quickly", "Test it works"]
    validates: ["Does tutorial work end-to-end?", "Are errors helpful?"]
```

## How personas connect to the workflow

- `/sensei:idea` — which personas are affected by this problem?
- `/sensei:blueprint` — consider each persona's journey through the feature
- `/sensei:build` — validate against active persona's criteria
- `/sensei:review` — check from each persona's perspective
- Feature specs — Gherkin scenarios tagged with persona

## Open questions

| # | Question |
|---|----------|
| 1 | Should personas be stored in YAML or markdown? YAML is structured, markdown is richer. |
| 2 | How many process mindsets is too many? 7 might overwhelm. Start with core 3 + add on demand? |
| 3 | Should the AI automatically cycle through all personas during review, or only the active one? |
| 4 | Can personas be shared across projects? (e.g., "API consumer" is common) |
