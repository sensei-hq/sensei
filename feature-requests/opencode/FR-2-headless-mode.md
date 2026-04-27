---
target: opencode-ai/opencode
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# Headless/batch mode for automated benchmarks

## Summary

No way to programmatically start a session with a predefined prompt, let it execute autonomously, and capture structured results. This blocks automated benchmarks and A/B testing of different configurations.

## What's needed

```bash
opencode --headless \
  --prompt "Fix the null pointer exception in src/parser.rs" \
  --max-turns 20 \
  --output-json result.json
```

Output: structured JSON with session_id, outcome, turns, tokens, files_modified, tools_used, corrections.

## Use case

- Benchmark: "Does adding MCP tools improve FTR?" — run same 5 tasks with/without, compare
- CI: "Run these tasks, fail if FTR drops below 80%"
- Research: reproducible evaluation of AI coding assistants
