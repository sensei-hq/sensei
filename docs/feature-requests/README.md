# Feature Requests — ACP Provider Tracking

Feature requests submitted to ACP providers that block or enhance sensei observatory capabilities. Track the **canonical issue** — if our submission is marked as duplicate, update to track the original.

## Claude Code (anthropics/claude-code)

| FR | What | Canonical Issue | Our Issue | Status |
|----|------|----------------|-----------|--------|
| FR-1 | Token counts in session hooks | [#11008](https://github.com/anthropics/claude-code/issues/11008) | [#50863](https://github.com/anthropics/claude-code/issues/50863) | open — #11008 is the original, ours flagged as possible dup |
| FR-2 | PostToolUse response preview | — | not submitted | **workaround**: MCP tools re-executable via daemon cache |
| FR-3 | Quota/cost programmatic access | [#50926](https://github.com/anthropics/claude-code/issues/50926) | [#50926](https://github.com/anthropics/claude-code/issues/50926) | open — /cost and /usage data exists, needs --json |
| FR-4 | Headless/batch session mode | [#50927](https://github.com/anthropics/claude-code/issues/50927) | [#50927](https://github.com/anthropics/claude-code/issues/50927) | open |
| FR-5 | Task-level lifecycle hooks | [#50931](https://github.com/anthropics/claude-code/issues/50931) | [#50931](https://github.com/anthropics/claude-code/issues/50931) | open — SessionEnd workaround: SessionStart closes prev + PreCompact + Stop |

## OpenCode (anomalyco/opencode)

| FR | What | Canonical Issue | Our Issue | Status |
|----|------|----------------|-----------|--------|
| FR-1 | Session metrics + lifecycle hooks | [#23454](https://github.com/anomalyco/opencode/issues/23454) | [#23454](https://github.com/anomalyco/opencode/issues/23454) | open |
| FR-2 | Headless/batch mode | [#23455](https://github.com/anomalyco/opencode/issues/23455) | [#23455](https://github.com/anomalyco/opencode/issues/23455) | open |

Note: opencode-ai/opencode is archived. Active repo is anomalyco/opencode.

## OpenAI Codex (openai/codex)

| FR | What | Canonical Issue | Our Issue | Status |
|----|------|----------------|-----------|--------|
| FR-1 | Structured session metrics in codex exec | [#18600](https://github.com/openai/codex/issues/18600) | [#18600](https://github.com/openai/codex/issues/18600) | open — related: [#10233](https://github.com/openai/codex/issues/10233) (JSON status), [#6432](https://github.com/openai/codex/issues/6432) (exec review) |

Note: Codex already has `codex exec` for headless execution — FR focuses on structured metrics output.

## GitHub Copilot

| FR | What | Canonical Issue | Status |
|----|------|----------------|--------|
| FR-1 | Session metrics + lifecycle events | — | draft: `copilot/FR-1-session-metrics.md` — no clear repo to submit to; Copilot architecture (IDE extension) is fundamentally different |

Note: Copilot has no headless mode and no plugin/hook system comparable to CLI tools. Feature requests may need to go through VS Code extension API or GitHub feedback channels.

---

## Rules

1. **Track canonical issues** — if our submission is marked as duplicate, update this table to point to the original
2. **Draft locally first** — review before submitting to external repos
3. **Note workarounds** — if we find a workaround, mark it here so we know what's still truly blocked
4. **Check periodically** — upstream issues may get implemented without notification
5. **Search before raising** — check for existing similar issues; track those instead of duplicating
