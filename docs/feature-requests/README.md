# Feature Requests — ACP Provider Tracking

Feature requests submitted to ACP providers that block or enhance sensei observatory capabilities. Track the **canonical issue** — if our submission is marked as duplicate, track the original.

## Claude Code (anthropics/claude-code)

| FR | What | Canonical Issue | Our Issue | Status |
|----|------|----------------|-----------|--------|
| FR-1 | Token counts in session hooks | [#11008](https://github.com/anthropics/claude-code/issues/11008) | [#50863](https://github.com/anthropics/claude-code/issues/50863) | open — #11008 is the original, ours flagged as possible dup |
| FR-2 | PostToolUse response preview | — | not submitted | **workaround**: MCP tools re-executable via daemon cache |
| FR-3 | Quota/cost programmatic access | [#50926](https://github.com/anthropics/claude-code/issues/50926) + related [#11008](https://github.com/anthropics/claude-code/issues/11008), [#49588](https://github.com/anthropics/claude-code/issues/49588) | [#50926](https://github.com/anthropics/claude-code/issues/50926) | open |
| FR-4 | Headless/batch session mode | — | not submitted (draft: `claude-code/FR-4-headless-session-mode.md`) | draft |
| FR-5 | SessionEnd hook with metrics | — | not submitted (draft: `claude-code/FR-5-session-end-hook.md`) | draft |

## OpenCode (opencode-ai/opencode)

| FR | What | Canonical Issue | Status |
|----|------|----------------|--------|
| FR-1 | Session metrics + lifecycle hooks | — | draft: `opencode/FR-1-session-metrics.md` |
| FR-2 | Headless/batch mode | — | draft: `opencode/FR-2-headless-mode.md` |

## OpenAI Codex (openai/codex)

| FR | What | Canonical Issue | Status |
|----|------|----------------|--------|
| FR-1 | Session metrics + task result structure | — | draft: `codex/FR-1-session-metrics.md` |

## GitHub Copilot

| FR | What | Canonical Issue | Status |
|----|------|----------------|--------|
| FR-1 | Session metrics + lifecycle events | — | draft: `copilot/FR-1-session-metrics.md` |

---

## Rules

1. **Track canonical issues** — if our submission is marked as duplicate, update this table to point to the original
2. **Draft locally first** — review before submitting to external repos
3. **Note workarounds** — if we find a workaround, mark it here so we know what's still truly blocked
4. **Check periodically** — upstream issues may get implemented without notification
