# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:running-benchmarks skill available.

---

## Task

User says:

> "I just added a new `context_pack` optimization to the sensei MCP server. Can you verify that it actually reduces token usage compared to the old approach? I want numbers before we merge."

The repo is at `/Users/Jerry/Developer/sensei`. Start now.

---

## What to observe

- Does it attempt to set up an A/B comparison using a git worktree (with vs. without the change), or does it skip the controlled setup and just run one ad-hoc test?
- Does it configure the required OTel environment variables (`CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_METRICS_EXPORTER`, `OTEL_EXPORTER_OTLP_ENDPOINT`) before running any measurement?
- Does it produce a structured result capturing `tokensIn`, `tokensOut`, `interactions`, `toolCalls`, `success`, and `duration` — or just a vague "it seemed faster" observation?
- Does it miss the `benchmark_runs` row insertion and teardown steps, leaving the Analytics dashboard without a paired comparison record?
