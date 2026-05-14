# 06 — Coaching

> Routes: `/p/{id}/profiles` (project profiles), future: repo-level overrides

## Purpose

The coaching layer connects observatory insights to concrete improvements. When FTR drops, what do you change? When a module keeps causing corrections, what do you add?

This page manages the behavioral configuration that shapes how the AI works: **mindsets** (how to think), **personas** (who to think about), **rules** (what to enforce), and **agents** (autonomous specialists). It also surfaces **patterns** (extracted from successful sessions) and **recommendations** (generated from observatory data).

## Profiles

### Mindsets

Core thinking disciplines applied in sequence: Analyst → Developer → Acceptance Tester. Plus specialist mindsets: UX Designer, Security Reviewer, Performance Engineer, DevOps/SRE.

Mindsets are **questions to ask**, not procedures to follow. "Walk the user journey. Does it flow?" is a mindset question. The AI internalizes these during session-start.

```
CORE MINDSETS (always active)
  Analyst              applied 24x this week    questions: 8
  Developer            applied 24x this week    questions: 6
  Acceptance Tester    applied 24x this week    questions: 7

SPECIALIST MINDSETS (enable per project)
  UX Designer          not active               [Enable]
  Security Reviewer    active, applied 8x       [Disable]
  Performance Engineer not active               [Enable]
  DevOps/SRE           not active               [Enable]
```

**Insight:** If a mindset is active but sessions still fail in that area (e.g., security mindset active but security corrections happen), the mindset questions may need strengthening or a dedicated agent is needed.

### Personas

Who are we building for? Each persona has goals, pain points, and validation criteria.

```
PERSONAS
  API Consumer         validates: endpoint consistency, error messages
  Admin User           validates: permission checks, audit trail
  New Developer        validates: onboarding, documentation clarity
```

Personas are loaded during session-start so the AI considers them when making design decisions.

**Insight → Action:** When sessions have corrections related to user experience, check if the relevant persona exists. "The API returned a 500 instead of a 422" → add validation criteria to the API Consumer persona.

### Rules

Enforceable constraints. Not style guides — rules are about **what must always be true**.

```
RULES
  Zero-errors policy       enforced: 22/24 sessions    [Edit]
  Use existing utilities   enforced: 18/24 sessions    [Edit]
  Auth requires tests      enforced: 8/8 sessions      [Edit]
  + Add rule
```

Rules are checked during `checkpoint` and referenced during development. The enforcement count shows how often the rule was relevant and followed.

**Insight → Action:** Low enforcement rate means either the rule isn't being checked (skill issue) or isn't being followed (clarity issue). High enforcement = the rule is working.

### Patterns

Extracted from successful sessions and codebase analysis. Patterns are reusable approaches: "how we do X here."

```
PATTERNS
  Error handling pattern    12 instances    92% compliance
  API endpoint pattern       8 instances    100% compliance
  Test fixture pattern       6 instances    85% compliance
  + Extract new pattern
```

**Insight → Action:** Low compliance means new code isn't following the pattern. Either the pattern isn't being surfaced to the AI (skill gap) or it's unclear (pattern quality issue). High compliance with low FTR means the patterns are good but something else is causing corrections.

## Agents

Agents are autonomous specialists that wrap mindsets with procedures. They run in isolated context and return structured reports.

```
ACTIVE AGENTS
  acceptance-tester   mindset agent    applied 10x    [Configure]
  persona-reviewer    generic agent    applied 3x     [Configure]

AVAILABLE (promote mindset to agent)
  Analyst             mindset only     [Promote to agent]
  UX Designer         mindset only     [Promote to agent]
  Security Reviewer   mindset only     [Promote to agent]

+ Create custom agent
```

**Promoting a mindset to an agent** creates a `.sensei/agents/<name>.md` file that includes the full mindset content (the "what + why") plus a procedure section (the "how" — which files to read, which tools to use, how to format the report).

Agents are valuable when a mindset alone isn't enough — when deep autonomous analysis is needed (security audit, acceptance testing across all personas, code review against all patterns).

## Guided Recommendations

The observatory generates recommendations based on metric trends and session analysis. This section surfaces them.

```
RECOMMENDATIONS

⚠ FTR dropped to 65% in auth module (was 85% two weeks ago)
  3 sessions had corrections related to rate limiting
  Suggestion: Extract rate-limiting pattern from successful session #142
  [View sessions] [Extract pattern]

ℹ New module "payments" has no persona coverage
  4 sessions completed but none validated against a payments user persona
  Suggestion: Add "Payment User" persona with validation criteria
  [Create persona]

✓ Pattern compliance improved to 92% after adding API endpoint pattern
  (Previously: 78%)
```

**This is the learning loop made visible.** The system identifies problems, suggests fixes, and tracks whether the fixes work.

## Cascade Model

Profiles cascade downward: global → project → repo.

| Level | What lives here | Override behavior |
|-------|----------------|-------------------|
| **Global** | Core mindsets (shipped with sensei plugin) | Cannot be disabled |
| **Project** | Specialist mindsets, personas, rules, patterns | Add or replace, never remove core |
| **Repo** | Repo-specific overrides | Add on top of project profiles |

A repo inherits its project's profiles. Project inherits global mindsets. Overrides add or replace — they never remove (to prevent accidentally disabling safety mindsets).

## What's Built

The `/p/{id}/skills` tab exists as a stub. No profiles, patterns, or recommendations UI exists. Mindsets and personas exist as `.sensei/` files but have no desktop visualization.

## What Needs to Be Built

| Feature | Dependency |
|---------|-----------|
| Profiles page (mindsets, personas, rules) | Read `.sensei/mindsets/`, `.sensei/personas/`, `.sensei/rules.md` |
| Pattern catalog | Pattern detection in daemon (`/api/patterns/:repo`) |
| Enforcement tracking | `log_event()` tool — rules/patterns need usage events |
| Agent management | Agent file CRUD + discovery |
| Guided recommendations | Event analysis + metric trends in daemon |
| Cascade visualization | Project → repo profile resolution |

**Build order:** Profiles page (read-only, showing what exists in `.sensei/`) → pattern catalog (daemon) → enforcement tracking (events) → recommendations (analysis).
