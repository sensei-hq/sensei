---
name: Playground & Insights Engine
description: Interactive MCP tool exploration, session replay, usage analytics, local-model-powered reasoning, and change-impact tracking
date: 2026-04-23
status: idea
related: 07-metrics-analytics.md, 20-local-inference.md, 24-desktop-observatory.md, 08-codebase-intelligence.md
---

# Playground & Insights Engine

## Problem

The current MCP playground mockup lets you browse tools and try them. That's a starting point, but it misses the real value: **understanding how tools are actually being used in sessions, which ones matter, and what to do about the ones that don't.**

An assistant calls MCP tools dozens of times per session. Some tools are used constantly (search, get_callers). Some are never called (check_drift, recommend_next). Some are called but produce results the assistant ignores. The developer has no visibility into any of this — and no way to answer:

- Which tools does the assistant actually use? Which does it ignore?
- When a tool IS used, what did the assistant ask and what did it receive?
- Is the tool effective — did sessions that used it have better FTR?
- If a tool is underused, is it because the skill/command doesn't instruct the assistant to use it?
- If I tune a skill or add a rule, did it actually improve things?

These are insights that compound. Each one leads to an action, and each action's impact needs tracking.

## Three capabilities

### 1. Playground — Interactive tool exploration and testing

Browse all MCP tools (sensei + third-party installed services). For each tool:

- **Try it** — fill in inputs, execute, see the response. Inputs auto-adapt by type (project selector, library picker, text, enum buttons).
- **See examples** — curated request/response pairs showing what the tool does in context.
- **Scope switching** — switch between sensei MCP and installed third-party MCPs (postgres-mcp, stripe-mcp, etc.) to explore their tool catalogs.

This is the "what CAN these tools do?" question.

### 2. Replay — Session tool usage analysis

For any session, replay the tool calls the assistant made:

- **Tool call timeline** — every MCP call in order: which tool, what inputs, what response, how long.
- **What the assistant asked** — the exact parameters sent.
- **What the assistant received** — the exact response returned.
- **What the assistant did with it** — did it use the response in its next action, or ignore it?
- **Replay controls** — step through calls, compare across sessions, filter by tool.

This is the "what DID the assistant do?" question.

### 3. Insights — Usage analytics and effectiveness correlation

Aggregated across sessions, per-project or system-wide:

**Usage patterns:**
- Tool usage frequency — which tools are called most/least
- Tool usage by project — some projects use graph tools heavily, others don't
- Tool usage over time — is adoption increasing after a skill change?
- Unused tools — registered but never called (why? skill not instructing it? tool name unclear?)

**Effectiveness correlation:**
- Sessions that used tool X had Y% FTR vs Z% without
- Specific tool calls that preceded corrections — the tool returned good data but the assistant misused it
- Tool response quality — did search() return relevant results? Did get_callers() find the right callers?

**Skill/command tuning signals:**
- "search() is used 47x but get_patterns() is used 0x → the skill doesn't mention patterns"
- "get_callers() is called but assistant ignores the result in 30% of cases → response format may be confusing"
- "After adding the auth persona, search() calls in lumen-auth sessions dropped 40% and FTR rose 14% → persona provides enough context that search isn't needed"

This is the "what SHOULD we change?" question.

---

## Insight consolidation

Insights don't live only in the playground. They are collected across multiple viewpoints and consolidated:

### Collection points

| Source | What it captures |
|--------|-----------------|
| Session replay | Tool call sequences, correction patterns, tool response quality |
| Code graph | Rework hotspots, god-nodes, duplicate clusters, stale files |
| Pattern detection | Emerging patterns, anti-patterns, pattern adherence |
| Library analysis | Usage drift, undocumented APIs, missing library rules |
| FTR trends | Per-project, per-module, per-pattern effectiveness |

### Consolidation levels

| Level | What it shows | Where in UI |
|-------|-------------|-------------|
| **System** | Cross-project insights, global tool effectiveness, overall FTR | Observatory daily view |
| **Project** | Project-specific tool usage, module-level FTR, project recommendations | Project overview |
| **Session** | Individual session replay, tool call timeline, corrections | Session detail |

All insights feed into the **recommendations engine** — the same system that produces urgency-ranked action cards in the observatory and project views.

---

## Local model reasoning (MOE consensus)

Some insights require reasoning beyond heuristics. Pattern detection, prompt classification, correction analysis, and recommendation generation benefit from LLM inference.

### Architecture: Mixture-of-Experts consensus

Instead of trusting a single model's judgment, use 2-3 local models that discuss, debate, and reach consensus:

```
Insight signal (e.g. "auth module FTR dropped 14% this week")
        │
        ▼
┌───────────────────────────────────────────────┐
│  Reasoning panel (local models via Ollama)     │
│                                                │
│  Model A (Gemma4 27B)    — proposes root cause │
│  Model B (Qwen3 14B)     — challenges/refines  │
│  Model C (Llama 4 Scout) — synthesizes action   │
│                                                │
│  Consensus protocol:                           │
│  1. Each model independently analyzes signal   │
│  2. Models see each other's analysis           │
│  3. Points of agreement → high-confidence      │
│  4. Points of disagreement → flagged for human │
└───────────────┬───────────────────────────────┘
                │
                ▼
        Recommendation with confidence score
        + reasoning trace (visible to user)
```

### What the panel reasons about

| Task | Input | Output |
|------|-------|--------|
| **Root cause analysis** | FTR drop + session replays | "3 corrections all touched refresh-token flow. No persona covers this module." |
| **Action proposal** | Root cause + project context | "Create auth-tests persona with clock-skew and mutex rules" |
| **Impact prediction** | Historical pattern + proposed action | "Similar persona in lumen-canvas improved FTR by 18% in 7 days" |
| **Change validation** | Before/after metrics | "After adding persona: FTR +14%, search() calls −40%, corrections −67%" |

### Key design principles

1. **Transparent reasoning** — the user sees the models' debate, not just the conclusion. Trust comes from showing the work.
2. **Consensus, not majority vote** — disagreement is a signal. If models disagree on root cause, the recommendation says "uncertain — here are two hypotheses."
3. **Human in the loop** — the panel proposes, the user decides. No autonomous changes to skills, rules, or personas.
4. **Graceful degradation** — if Ollama isn't running, insights fall back to heuristics (frequency counts, threshold alerts). Less sophisticated but still useful.
5. **Model-agnostic** — any Ollama-compatible model works. Users configure their preferred panel in settings.

---

## Change-impact tracking

Every action taken from a recommendation must be tracked to close the loop:

```
Recommendation created
  "Add auth persona" (projected FTR +14%)
        │
        ▼
User accepts → action executed
  persona created, enabled for lumen-cloud
        │
        ▼
Impact measurement begins
  - Baseline FTR captured at time of change
  - 7-day window for comparison
        │
        ▼
Impact report generated
  ┌────────────────────────────────────┐
  │ Change: auth-tests persona added    │
  │ Date: 2026-04-23                    │
  │ Baseline FTR: 64%                   │
  │ Current FTR: 78% (+14%)             │
  │ Corrections before: 3.2/session     │
  │ Corrections after: 1.1/session      │
  │ Tool usage: search() −40%           │
  │ Verdict: POSITIVE                   │
  └────────────────────────────────────┘
```

### What gets tracked

| Tracked | Before | After | Delta |
|---------|--------|-------|-------|
| FTR (per project and per module) | Baseline snapshot | Rolling 7-day | Positive / negative / neutral |
| Corrections per session | Baseline average | Rolling average | Direction + magnitude |
| Tool usage patterns | Frequency snapshot | Rolling frequency | Which tools gained/lost usage |
| Session duration | Baseline average | Rolling average | Shorter = more efficient |
| Pattern adherence | Baseline % | Rolling % | Did the new rule stick? |

### Negative impact detection

If a change makes things worse:
- **Alert:** "Auth persona added 5 days ago. FTR dropped from 64% to 58%. Corrections increased."
- **Reasoning panel analyzes:** "The persona's clock-skew rule conflicts with the existing retry pattern. Sessions correct the persona's guidance."
- **Suggested action:** "Revise persona to defer to existing retry pattern for skew handling."

This creates a **continuous improvement loop**: observe → hypothesize → act → measure → adjust.

---

## Data model additions

### `tool_calls` table (new)

Captures every MCP tool call made during sessions.

```
tool_calls
├── id                uuid PK
├── session_id        uuid FK → sessions
├── folder_id         uuid FK → folders           -- which repo context
├── tool_name         text                         -- e.g. "search", "get_callers"
├── service_name      text                         -- e.g. "sensei", "postgres-mcp"
├── input_params      jsonb                        -- what the assistant sent
├── response          jsonb                        -- what the tool returned
├── duration_ms       integer
├── used_in_response  boolean                      -- did assistant use the result?
├── turn_number       integer                      -- which turn in the session
├── created_at        timestamptz
```

### `change_impacts` table (new)

Tracks the impact of actions taken from recommendations.

```
change_impacts
├── id                uuid PK
├── recommendation_id uuid                         -- which recommendation triggered this
├── project_id        uuid FK → projects
├── change_type       text                         -- persona_added, rule_promoted, skill_enabled, ...
├── change_detail     jsonb                        -- what specifically changed
├── baseline          jsonb                        -- {ftr, corrections_avg, tool_usage, ...}
├── current           jsonb                        -- same shape, rolling values
├── verdict           text                         -- positive | negative | neutral | pending
├── measured_at       timestamptz                  -- when impact was last computed
├── created_at        timestamptz
```

### `reasoning_traces` table (new)

Stores the local model panel's reasoning for transparency.

```
reasoning_traces
├── id                uuid PK
├── insight_id        uuid                         -- what triggered the reasoning
├── project_id        uuid FK → projects
├── models_used       text[]                       -- ["gemma4:27b", "qwen3:14b"]
├── exchanges         jsonb                        -- [{model, role, content}, ...]
├── consensus         jsonb                        -- {conclusion, confidence, disagreements}
├── action_proposed   jsonb                        -- the recommendation generated
├── created_at        timestamptz
```

---

## Open questions

| # | Question |
|---|----------|
| 1 | How many local models is practical? 2 is fast, 3 is richer debate, 4+ may be diminishing returns. |
| 2 | Should the reasoning panel run on every insight automatically, or only when the user asks "why?"? |
| 3 | What's the minimum model size for useful reasoning? Gemma4 12B might suffice for simpler tasks. |
| 4 | How do we handle model hallucination in the reasoning panel? Cross-validation between models helps but doesn't eliminate it. |
| 5 | Should change-impact tracking use a fixed window (7 days) or adaptive (until statistically significant)? |
| 6 | Can we use the reasoning trace as training signal to improve the local models over time (fine-tuning on project-specific reasoning)? |
| 7 | Privacy: reasoning traces contain session content. Should they be redactable? |
