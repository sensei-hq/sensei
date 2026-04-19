---
name: Desktop Observatory
status: idea
origin: conversation
date: 2026-04-19
description: Redesign the sensei desktop app as a session observatory — analytics, tools, profiles, code intelligence, and community
---

# Desktop Observatory

## Problem

The current desktop app is a code intelligence dashboard — graphs, communities, indexing controls. It answers "what does my code look like?" but not the questions the AI Driven Developer actually asks:

- How efficient are my sessions?
- What is Claude doing and why?
- How fast am I burning through my quota?
- Is this tool/library actually helping?
- Something went wrong — how do I investigate?
- I see problems — how do I act on them?

## Vision

The desktop is a **session observatory** — the place where AI-assisted developers understand, optimize, and debug their AI-powered workflow. Every screen answers a question. Every insight links to an action.

## Information Architecture

### Navigation

```
Sidebar:
  Home (dashboard)
  Sessions
  Projects
  Libraries
  Tools
  Profiles (mindsets, personas, rules)
  Benchmarks
  Community
  Settings

Header:
  daemon status indicator
  token/quota burn rate
```

## Pages

### 1. Home — "How am I doing?"

The daily landing page. Answers at a glance.

**Metrics cards:**
- FTR (first-try-right) score with trend
- Session count (this week / total)
- Rework rate with trend
- Token usage + cost + burn rate

**Tool adherence bar:**
- % MCP tools vs fallback (grep, manual file reading)
- Goal: 90%+ MCP usage

**Recent sessions:**
- Last 5-10 sessions: task, outcome badge, turns, cost
- Click to drill into any session

**Quota gauge:**
- Remaining quota with projected days at current burn rate
- Visual warning when < 20%

**Active context:**
- Current task/issue from workflow state
- Quick actions: "Start session", "View backlog"

### 2. Sessions — "What happened?"

**List view:**
Sortable table: date, task/issue, project, outcome, FTR, turns, corrections, tokens, cost, duration

Filterable by: project, solution, outcome, date range

**Session detail (drill-in):**

Event timeline — every turn, tool call, phase transition, correction:
```
Turn 1: new_request
Turn 2: tool_used → search("compute_metrics") → 3 results
Turn 3: tool_used → get_callers("insert_event") → 5 callers
Turn 4: continuation
Turn 5: tool_used → grep (fallback!) ⚠️
Turn 6: revision_requested — "that's wrong, use MCP"
Turn 7: tool_used → search("insert_event") → corrected ✓
```

Profiles applied — which mindsets/personas fired, which didn't, adherence score

Rules checked — which rules were followed/violated

Clicking on:
- A tool call → shows what it returned (input params + response)
- A profile → shows its questions and whether they were answered
- A warning → explains the issue and suggests a fix

### 3. Projects — "What am I working on?"

**Solutions as collapsible groups:**
- Solution name, repo count, roles
- Indexed status, last session
- Expand to see individual repos

**Repo detail (drill-in):**
- Symbol count (functions, types)
- Graph visualization — how the code is organized
- Complexity hotspots — functions with high cyclomatic complexity
- Dead code candidates — exported but never called
- Duplicates — identical functions across files
- Doc drift — docs that reference changed code

**Actionable insights:**
Every finding has a "Tell Claude" button that generates a prompt:
- "Investigate this dead code cluster"
- "Refactor these 3 duplicate functions into a shared util"
- "Update this doc that references a renamed function"

The prompt is copied to clipboard or opened in Claude Code directly.

### 4. Libraries — "What docs does Claude have?"

**Indexed libraries:**
- Name, section count, last indexed date, freshness indicator
- "Stale" badge if indexed > 30 days ago
- "Used in N sessions" — is this library actively helping?

**Library detail:**
- Sections / components with content preview
- "Simulate: what would get_lib_docs('X') return?" — test the tool
- Re-index button (refresh docs)

**Add library:**
- Name + optional URL
- Auto-discovery from llms.txt
- Preview what will be indexed before committing

**Maintenance dashboard:**
- Which libraries are stale?
- Which are unused (indexed but never queried)?
- Recommended: libraries detected in your code but not yet indexed

### 5. Tools — "What can sensei do?"

**MCP tool catalog:**
Every available MCP tool with:
- Name, description
- Input parameters with types
- Example input/output
- "Try it" — simulate a call with custom params, see live response
- Usage stats: how many times used across sessions

**Tool health:**
- Which tools are returning useful data?
- Which tools are never used?
- Error rates per tool

**Tool comparison:**
- Side-by-side: "search('foo')" vs "grep foo" — what does each find?
- Demonstrates why MCP is preferred

### 6. Profiles — "What's helping and what's not?"

The user doesn't care about profiles for their own sake. They care about outcomes. This page connects the dots: which levers (skills, mindsets, personas, rules, libraries) are improving quality/time/cost, and which are noise or actively hurting.

**Impact view (default):**
Every active lever ranked by impact on session outcomes:
```
Lever                    │ Sessions │ FTR Impact │ Token Impact │ Verdict
─────────────────────────┼──────────┼────────────┼──────────────┼─────────
Analyst mindset          │ 12       │ +15% FTR   │ +8% tokens   │ ✓ keep — worth the token cost
BAT mindset              │ 10       │ +22% FTR   │ +12% tokens  │ ✓ keep — biggest quality gain
Developer mindset        │ 12       │ +5% FTR    │ +3% tokens   │ ✓ keep — low cost, steady
Security Reviewer        │ 0        │ n/a        │ n/a          │ ? unused — remove or needs trigger?
get_lib_docs(rokkit)     │ 4        │ +10% FTR   │ −5% tokens   │ ✓ keep — saves rework
Plugin Developer persona │ 3        │ +8% FTR    │ +6% tokens   │ ~ marginal — review questions
TDD rule                 │ 12       │ +18% FTR   │ +15% tokens  │ ✓ keep — high quality lift
```

**What to look for:**
- High token cost, low FTR impact → remove or simplify
- Never applied → wrong trigger, irrelevant to project, or poorly described
- High FTR impact, modest token cost → keep and possibly strengthen
- Negative FTR impact → actively causing problems, investigate

**Discovery: "How do I identify personas for my repo?"**
- Sensei analyzes your session corrections and groups them by root cause
- "60% of corrections were about missing user perspective" → suggests an end-user persona
- "3 sessions failed because auth wasn't considered" → suggests a security reviewer mindset
- Persona/mindset recommendations based on YOUR project's actual pain points, not generic templates

**Actions:**
- Toggle any lever on/off
- Edit questions, rules, descriptions inline
- "Suggest improvements" — analyzes recent sessions and recommends changes to profiles
- "Create persona from session data" — extracts a persona from patterns in corrections

**Personas (.sensei/personas/):**
- List with usage stats
- Click to view questions, journey, pain points
- "This persona's validates were checked 2/5 times — coverage gap"

**Rules (.sensei/rules.md):**
- Rule-by-rule adherence stats
- "TDD: 92% adherence" — click to see the 8% violations
- Edit rules inline

### 7. Benchmarks — "Is sensei helping?"

**For evaluation:**
- "I have a repo — will sensei benefit my case?"
- Configure test tasks (e.g., "add a feature", "fix a bug", "refactor a module")
- Run with sensei vs baseline
- Compare: turns, accuracy, time, cost

**Benchmark results:**
- Score cards per task
- Aggregate improvement metrics
- Shareable report (export as markdown or link)

**Corpus management:**
- Test scenarios / tasks
- Expected outcomes
- Historical runs

### 8. Community — "How do I participate?"

**Share:**
- Share benchmark results (anonymized)
- Share a pattern that worked well
- Share a mindset or persona

**Report:**
- Report a tool issue
- Suggest an improvement
- Report a finding that could help others

**Support:**
- GitHub stars / sponsor
- Contribute skills, mindsets, personas
- Plugin marketplace (future)

**Discover:**
- Browse shared patterns from other users
- Browse shared personas/mindsets
- See aggregate benchmark data ("sensei improves FTR by X% on average")

### 9. Settings

- Daemon status + health
- ACP configuration
- Workspace: scan folders, reset data
- Display preferences
- Global skills enable/disable

## Design Principles

1. **Three metrics drive everything** — Quality (FTR), Time (turns), Cost (tokens). Every page connects back to at least one. If a feature doesn't move these numbers, question whether it belongs.
2. **Every number is clickable** — FTR links to sessions, tool adherence links to calls, persona usage links to profile
3. **Every insight has an action** — "Dead code found" → "Tell Claude to investigate". Not just display — enable the next step.
4. **Show impact, not inventory** — Don't list mindsets. Show which ones improved FTR and which cost tokens without helping. The user tunes levers by impact, not by reading descriptions.
5. **Token/cost awareness everywhere** — header shows burn rate, sessions show cost, dashboard shows quota remaining
6. **Suggest, don't prescribe** — "60% of corrections were about UX" → suggests a persona. Doesn't force it. The user decides what levers to add/remove based on their data.
7. **Simulate before committing** — test a tool call, preview a library index, run a benchmark on a subset before going all-in
8. **Stale data is flagged** — library docs indexed 3 months ago get a warning. Unused tools get a hint. Mindsets never applied get a nudge.

## Personas served

| Persona | Primary pages | Key question |
|---------|--------------|--------------|
| AI Driven Developer | Home, Sessions, Profiles | "How efficient am I and how do I improve?" |
| Plugin Developer | Tools, Profiles, Libraries | "Is my plugin/skill working and helping?" |
| API Consumer | Tools | "What tools exist and how do I use them?" |
| Evaluator (new) | Benchmarks, Home | "Should I adopt sensei for my team?" |
| Contributor (new) | Community, Profiles | "How do I share what I've learned?" |

## What gets removed from current app

| Current page | Disposition |
|-------------|-------------|
| `/s/[id]/arch` | Merged into Project detail |
| `/s/[id]/trace` | Removed — placeholder, no real data |
| `/s/[id]/skills` | Merged into Profiles |
| `/acp` | Merged into Settings |
| `/catalog` | Replaced by Tools page |
| `/graph` redirect | Removed — graph lives inside Project detail |
| Setup wizard | Keep but simplify |

## Implementation approach

1. Extract shared utilities first (ftrClass, getStatus, STATUS_CLS, catalogs)
2. Build new page shells with the revised nav
3. Migrate data from existing pages into new structure
4. Add new functionality incrementally (tool simulation, benchmark runner, community features)

## Open questions

- Should "Tell Claude" generate a clipboard prompt or open Claude Code directly?
- How do we track mindset/persona application? (Needs daemon-side event logging)
- Benchmark comparison requires running sessions — is this automated or manual?
- Community features need a backend — hosted service or GitHub-based?
