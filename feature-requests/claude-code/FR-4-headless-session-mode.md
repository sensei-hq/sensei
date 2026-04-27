---
target: anthropics/claude-code
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# FR-4: Headless/batch session mode for automated benchmarks

## Summary

There is no way to programmatically start a Claude Code session with a predefined prompt, let it execute, and capture the result. This blocks building automated benchmarks, regression tests, and A/B comparisons of different configurations.

## Use Case

A developer wants to evaluate whether sensei (mindsets, personas, MCP tools) actually improves their workflow. They want to:

1. Define 5 test tasks ("add a feature", "fix a bug", "refactor a module")
2. Run each task twice: once with sensei, once without
3. Compare: turns, corrections, outcome, time
4. Get a report: "sensei improved FTR by 15% but added 8% more tokens"

Today this requires manually running 10 sessions and recording results by hand.

## Proposed Solution

```bash
claude --headless \
  --prompt "Fix the null pointer exception in src/parser.rs" \
  --max-turns 20 \
  --output-json session-result.json
```

Output:
```json
{
  "session_id": "...",
  "outcome": "completed",
  "turns": 8,
  "tokens_in": 24000,
  "tokens_out": 8000,
  "duration_seconds": 45,
  "files_modified": ["src/parser.rs"],
  "tools_used": ["search", "Read", "Edit"],
  "corrections": 0
}
```

## Alternatives

- `claude -p "prompt"` exists but is single-turn, not agentic
- Recording manual sessions works but doesn't scale and introduces human variance
- Replaying recorded sessions (like VCR tests) would also work but requires a recording mechanism

## Impact

This unlocks:
- Automated quality benchmarks for AI-assisted development tools
- A/B testing of different prompt strategies, skills, or configurations
- CI integration — "run these 5 tasks, fail if FTR drops below 80%"
- Reproducible evaluation for researchers comparing AI coding assistants
