---
name: AI Session User
category: persona
description: Developer using sensei in their daily AI-assisted coding sessions
goals:
  - Get oriented quickly at session start
  - Find relevant code without reading files manually
  - Trust that the AI follows project rules and patterns
pain_points:
  - Session context is lost between conversations
  - MCP tools return too much or too little data
  - AI ignores rules.md and makes assumptions
validates:
  - Does session start provide useful orientation?
  - Do MCP queries return actionable results?
  - Are project rules visible and enforced?
---

# AI Session User

A developer who uses sensei daily to help with coding tasks. They don't care how sensei works internally — they care that it makes their sessions productive.

## Journey

1. Opens Claude Code in their project
2. Session starts — expects to see project context, active task, rules
3. Asks Claude to work on a feature or bug
4. Expects Claude to use MCP tools (search, callers, patterns) instead of grepping
5. Expects Claude to follow project rules and mindsets without being reminded
6. Ends session — expects progress to be checkpointed

## Questions

Ask these when building or changing anything this persona touches:

1. **Does the session start give me useful context?** — Within the first response, do I know what project I'm in, what the active task is, and what rules apply?
2. **Is the AI using the tools?** — When I ask about code, does the AI use MCP search/callers/patterns, or does it fall back to grep and guessing?
3. **Are my rules being followed?** — If I defined "TDD, tests first" in rules.md, does every coding task start with tests?
4. **Is progress being tracked?** — If I close the session and come back tomorrow, will the AI know what I was working on?
5. **Can I course-correct easily?** — When I say "that's wrong" or "not that approach", does the AI adjust, or does it repeat the same mistake?

## What frustrates them

- **Cold starts** — new session has no memory of yesterday's work. Has to re-explain context.
- **Rule drift** — AI followed the rules yesterday but ignores them today. Inconsistent behaviour across sessions.
- **Tool avoidance** — AI falls back to grep/file reading when MCP tools would give better results. Feels like the tools aren't being used.
