# Observatory

The observatory is your daily view into AI-assisted development. Sensei watches every coding session, tracks corrections, learns patterns, and surfaces what matters. Open the app each morning and the observatory tells you the one thing you should pay attention to.

## Daily view

The observatory is a single screen with a collapsible sidebar and a main content area.

**Sidebar** -- navigation (Today, Sessions, Learnings, Libraries, Settings), active projects with per-project FTR indicators, recent or dormant projects (collapsed), and daemon health status.

**Main area** -- the Today view:

- **Greeting strip** -- date and a personalized hello (morning, afternoon, or evening)
- **FTR hero** -- your First-Try Rate as a prominent value with delta versus prior period and a 14-day sparkline trend
- **Hero koan** -- the single most important insight right now, written as a brief observation ("The AI does not know your auth.") with supporting context, a call-to-action button, projected FTR impact, and the evidence trail that produced it
- **Insights** -- up to three categorized observations: a recurring pattern, an adopted teaching with measured impact, a drift detection with urgency
- **Adopted teachings** -- a timeline of rules Sensei has learned and applied, showing when each was adopted, which project it applies to, and what changed
- **Recent sessions** -- the last four to eight sessions with project, title, FTR status, correction count, and duration

## Early mode

When Sensei has observed fewer than roughly five sessions, the observatory enters early mode. It is quiet and unassuming.

The FTR indicator shows a "building" state -- not enough data yet. The hero message says Sensei is listening. It reports how many sessions have been watched, which repositories are showing early signals, and an estimate of how many sessions remain before the first lesson.

Early signals may appear ("Watching prompt style in canvas. Early signal: you prefer terse.") with a listening status indicator. The teachings panel is empty with an explicit note that Sensei needs more sessions.

This prevents disappointment. The user just finished setup and sees nothing actionable yet. Early mode communicates progress so they know the system is working, not broken.

## Mature mode

After five or more sessions, Sensei has enough data to teach.

The FTR score becomes a prominent metric with a 14-day trend. The hero koan card appears with a concrete coaching insight, evidence trail, and action button. Insights surface recurring patterns, teaching effectiveness, and drift detection. The teachings timeline fills with adopted rules and their measured FTR impact.

Every element on this screen connects to an action. A number without a recipe to improve it does not belong here.

## Sessions

Sessions are browsable across all projects. The session list shows each session's project, title, outcome (first-try, corrected, abandoned), FTR status, correction count, turn count, and duration.

**Session detail** -- drill into any session to see its full timeline:

- Timestamped events: start, context loaded, edits, tool calls, corrections, phase transitions, end
- For each tool call: what tool, what inputs, what the response was, whether the assistant used the result
- For each correction: what the user redirected and what the assistant changed
- Workflow phase at each point in the session
- Which rules were followed or violated

Session replay lets you step through the tool call sequence, compare across sessions, and understand where the assistant went off track.

**Retrospectives** -- aggregate analysis across recent sessions. What is going well (high FTR streaks, adopted patterns, shorter sessions), what is not going well (recurring corrections, low FTR projects, abandoned sessions), and cross-project signals.

## Teachings and coaching

Recommendations are the "so what" of every metric observation. Each recommendation is ranked by urgency and tied to evidence.

**Recommendation types** -- create a persona for a struggling module, promote an emerging pattern to a project rule, fix an anti-pattern (duplicated code, god-node), enable a skill that exists but is not active, audit stale code that has drifted from related files.

**Action drawer** -- click any recommendation to open a slide-out panel with:

- Why this matters, in plain language, referencing specific sessions
- Evidence list with session IDs and descriptions
- Projected FTR impact
- ACP selector (Claude Code, Cursor, or copy to clipboard)
- A pre-built prompt, editable before sending
- Reasoning panel output (what the local models concluded and why)

**7-day impact measurement** -- every accepted recommendation enters a measurement window. Sensei captures baseline FTR at the time of the change, then compares over seven days. The verdict is positive (teaching adopted, shown in the observatory), neutral (no significant change, keep monitoring), or negative (things got worse).

**Negative impact detection** -- if a change makes things worse, Sensei raises an alert with the FTR and corrections deltas, the reasoning panel's root cause analysis, and suggested actions: revise the rule, revert the change, keep monitoring, or dismiss. This is the safety net -- if Sensei gives bad advice, you know immediately and have a clear path to undo it.

The result is a continuous loop: observe sessions, hypothesize through the insights engine, act on a recommendation, measure the impact, and adjust.

## Memory

Memory is how Sensei retains knowledge across sessions. Every correction, every learned pattern, every outcome feeds into a knowledge system that grows and evolves.

**How memories form** -- corrections observed during sessions become memory candidates. After two or more corrections on the same topic, Sensei proposes a memory. Accepted recommendations with positive outcomes also become memories. The assistant itself can report learnings during a session.

**Memory anatomy** -- every memory has a "what" (the rule), a "because" (the consequence of ignoring it, grounded in real incidents), a scope (global, project, module, task type, or stack), a strength score (reinforced by evidence, weakened by time), and references to concrete code (good examples to follow, bad examples to avoid, the pattern definition, and evidence sessions).

**Memory lifecycle** -- memories start at low strength and gain strength each time the same correction is repeated or the memory is confirmed. Memories that go unreferenced for 90 days weaken and eventually archive. Contradicted memories are flagged for review. Battle-tested memories (strength 3 or higher) are surfaced with high priority.

**Memory detail view** -- drill into any memory to see its full story: the reasoning behind it, the original correction quote, scope breakdown, code references with file and line, and a chronological reinforcement history. You can edit the reasoning, adjust scope, promote the memory to a permanent project guideline, or archive it.

**Context assembly** -- at the start of every session, Sensei assembles relevant memories into a consolidated markdown document delivered to the assistant. Which memories are included depends on context: global preferences always, project memories when the project matches, module memories when working in those files, task-type memories when the task matches, and continuity memories when resuming interrupted work. High-strength memories are included first; low-strength memories are dropped if the token budget is tight.

**Context pack tool** -- in long sessions where the assistant starts forgetting rules, Sensei offers a context rotation: snapshot the current progress, clear accumulated noise, and reload fresh memories and active files. This prevents quality degradation without restarting the session.

**Consolidation** -- over time, memories accumulate and overlap. Sensei's reasoning panel identifies candidates for merging and proposes consolidated memories. You review the proposed merge, edit the combined text, and accept (archiving the originals) or keep them separate.

## Metrics

Metrics answer the question: is AI-assisted development getting better?

**First-Try Rate (FTR)** -- the percentage of sessions that complete without user corrections. This is the hero metric. It is tracked per project, per module, per pattern, and as a 14-day trend with delta.

**Supporting metrics:**

- Correction rate per module -- which areas of the codebase consistently trip up the assistant
- Rework count -- files edited across multiple sessions in a short window, indicating instability
- Tool usage -- which MCP tools the assistant calls, which it ignores, and which produce results that get discarded
- Pattern adherence -- whether sessions follow detected project patterns
- Session duration and turn count -- shorter sessions with fewer turns indicate efficiency

**Drill-down** -- every metric links to the sessions that produced it. Click an FTR drop to see which tasks had rework, which corrections were made, and which modules are affected. Click a rework hotspot to see the files and the sessions that touched them.

Metrics without actions are decoration. Every metric on this screen connects to a recommendation or a drill-down that leads to one.

## Settings

Settings is where you revisit any configuration made during setup and adjust ongoing behavior.

- **Re-run setup steps** -- scan roots, projects, libraries, instruments, inference, and assignments are all accessible individually
- **Preferences** -- communication style, sharing, privacy, digest cadence
- **Assistants** -- registered AI coding assistants grouped by family, with registration status, version, and transport configuration. Re-register or deregister as needed.
- **Extensions** -- installed skills, commands, agents, hooks, and plugins with enable/disable toggles and per-project scoping
- **Debug mode toggle** -- surface daemon internals, reasoning traces, and raw event streams for troubleshooting

## Reference

- Mockups: `mockups/lib/observatory.jsx`, `mockups/lib/sessions.jsx`, `mockups/lib/sessions-zen.jsx`, `mockups/lib/learnings.jsx`, `mockups/lib/learnings-v2.jsx`
- Design: [design/01-app.md](../design/01-app.md), [design/02-daemon.md](../design/02-daemon.md)
