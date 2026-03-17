---
name: pattern-based-development
description: Use before implementing any new feature, component, module, or integration
— checks PATTERNS.md for an applicable recipe before writing new code. Prevents
re-inventing structure that already exists in this codebase.
---

# Pattern-Based Development

## Overview

Before writing new code, check whether a structural pattern already exists for what you're about to implement. If it does, load the pattern's skill file and follow its recipe. Record the pattern use for tracking.

## Procedure

1. Read `PATTERNS.md` (repo root) — scan for patterns relevant to the current task
2. Match task description to pattern entries:
   - "Add an MCP tool" → `mcp-tool` pattern
   - "Add a CLI command" → `cli-command` pattern
   - "Add a new adapter" → `adapter` pattern
3. **If pattern found:**
   a. Load `skills/<pattern-name>/SKILL.md`
   b. Call `record_pattern_use("<pattern-name>")` MCP tool to start tracking
   c. Present the recipe to the agent before implementation
   d. Implement following the recipe exactly — file structure, exports, registration
4. **If no matching pattern:**
   - Proceed with standard implementation
   - Consider running `identifying-patterns` skill to document this new pattern afterwards
5. At session end, `checkpoint()` automatically links outcome and changed files to the pattern usage row

## Why This Matters

When every implementation of a pattern follows the same recipe:
- Reviewers know exactly what to look for
- New contributors can follow the same steps
- The codebase stays internally consistent

## Example

Task: "Add a `record_pattern_use` MCP tool"

1. Read `PATTERNS.md` → find `mcp-tool` pattern entry
2. Load `skills/mcp-tool/SKILL.md`
3. Call `record_pattern_use("mcp-tool")`
4. Follow recipe:
   - Create `packages/server/src/tools/record-pattern-use.ts`
   - Export `recordPatternUse(client, repoId, sessionId, patternName)`
   - Import and register in `mcp-server.ts` with `server.tool(...)`
   - Write tests in `record-pattern-use.spec.ts`
