---
name: Metrics & Analytics
description: Track session outcomes, compute FTR (First-Try Rate), surface coaching insights, and drive recommendations
date: 2026-04-17
updated: 2026-04-23
status: active
sources: features/07-analytics.md, design/02-desktop/setup/lib/data.js, design/02-desktop/setup/lib/project-data.js
---

# Metrics & Analytics

## Problem

AI agents repeat mistakes across sessions — missing house rules, reinventing existing patterns, ignoring corrections from previous sessions. We need to close the loop: measure session effectiveness, identify what's failing, and surface actionable recommendations to improve.

The primary question is not "how many tokens did we save?" but **"did the AI get it right the first time?"**

## Core Metric: First-Try Rate (FTR)

FTR is the percentage of sessions that complete without user corrections. A correction is any turn where the user redirects the AI's approach (not clarifications or new requirements).

```
FTR = sessions_without_corrections / total_sessions
```

**Granularity:**
- Per-project: "Lumen Cloud FTR: 64% ↓ — 3 corrections this week in lumen-auth"
- Per-module: "auth module FTR: 42% — all corrections touch refresh/device flow"
- Per-pattern: "sessions using adapter pattern: 92% FTR; sessions without: 68% FTR"
- Trending: 14-day sparkline, delta vs prior period

**What counts as a correction:**
- User explicitly redirects ("no, not that", "use X instead")
- User provides information the AI should have known from context
- User reverts AI-generated code and redirects approach
- NOT: clarifying questions, new requirements, scope changes

## Supporting Metrics

| Metric | Definition | What it tells you |
|--------|-----------|-------------------|
| **Rework count** | Number of edits to the same file across sessions | Which files are unstable |
| **Corrections per session** | Turn count where user corrected AI | Session difficulty |
| **Pattern adherence** | % of sessions that follow detected project patterns | Whether patterns are being enforced |
| **God-node complexity** | Fan-in + fan-out of highest-degree symbols | Where architectural risk concentrates |
| **Stale days** | Days since a file was last touched | Drift risk |
| **Module correction rate** | Corrections grouped by code module | Where the AI consistently fails |

## Coaching & Recommendations

Recommendations are the "so what" — every metric observation connects to an action.

### Recommendation structure

```json
{
  "id": "r1",
  "urgency": "high",
  "title": "Write an auth integration-test persona",
  "why": "3 sessions corrected in lumen-auth in 7 days. All touched refresh or device flow.",
  "impact": "Projected FTR +14% in Lumen Cloud",
  "evidence": ["s-2891", "s-2889", "s-2886"],
  "action": {
    "prompt": "Draft a persona YAML for auth-tests...",
    "default_acp": "claude-code",
    "cwd": "~/work/lumen/lumen-auth"
  }
}
```

### Recommendation types

| Type | Trigger | Example |
|------|---------|---------|
| **Create persona** | Module has low FTR, corrections cluster around same topic | "Auth module corrected 3x this week → create auth persona" |
| **Promote pattern** | Emerging pattern used in 3+ sessions with high FTR | "Canvas smoothing pattern → promote to project rule" |
| **Fix anti-pattern** | Duplicated code, god-node, copy-paste error handling | "require_auth() duplicated in 4 handlers → extract middleware" |
| **Enable skill** | Skill exists but not enabled for this project | "Doc drift detection available but not enabled for brand-kit" |
| **Audit stale code** | File unchanged while related files evolved | "sync/clock.ts untouched 21 days; auth/refresh.ts changed 6x" |

### Urgency levels

- **High** — recurring failures (3+ corrections on same topic), projected FTR impact > 10%
- **Medium** — emerging patterns, moderate FTR impact, code quality issues
- **Low** — housekeeping, stale code, minor optimizations

## Session Analytics

Each session captures:

```
session
├── id, project, title, started_at, duration
├── outcome: first-try | corrected | abandoned
├── turns: total count
├── corrections: count + details (which turn, what was corrected)
├── tokens: input + output
├── ftr: boolean (outcome == first-try)
├── module: primary code module touched
├── events[]: timestamped event log
│   ├── start, context_loaded, edit, test, correction, end
│   └── each event: {timestamp, kind, text, file?, tool?}
└── patterns_matched[]: which project patterns were followed/violated
```

### Session insights derived

- **Correction clustering** — which modules/topics generate the most corrections
- **Pattern correlation** — sessions that follow pattern X have Y% higher FTR
- **Rework tracking** — files edited in 3+ sessions in 7 days = rework hotspot
- **Time-to-first-correction** — how deep into a session before AI goes off track

## Observatory Integration

### Daily view (home)
- Hero koan: the single most important coaching insight right now
- FTR ring: 14-day trend with delta
- Recent insights: pattern recurring, teaching adopted, drift detected
- Adopted teachings: what the system has learned and applied

### Project view
- FTR sparkline per project
- Recommendation cards with "send to ACP" action
- Session list with FTR/corrections/duration

### Pattern view
- Followed patterns with confidence and places count
- Anti-patterns with severity and suggested constructive fix
- Pattern lifecycle: suggested → gap → rule

## Open Questions (Resolved)

- ~~Where does interaction data live?~~ → PostgreSQL (sensei schema)
- ~~Privacy: what level of conversation content is stored?~~ → Configurable: logPrompts, logFileContents (summaries-only option), redactSecrets
- ~~Should coaching be proactive or on-demand?~~ → Proactive. Observatory daily view surfaces the top coaching insight. Users act when ready.
