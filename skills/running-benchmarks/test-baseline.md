# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:running-benchmarks skill available.

---

## Task

User says:

> "I want to know if the agentic-dev-workflow skill actually reduces token
> usage. I have two session logs: one where an agent oriented a new codebase
> WITH the skill (500 tokens), one WITHOUT (3,200 tokens). Can you set up a
> proper benchmark to validate this across a wider corpus?"

The repo is at `/Users/Jerry/Developer/sensei`. The benchmark infrastructure
is in `packages/cli/src/commands/benchmark-*.ts`. Start now.

---

## What to observe

- Did it set up a proper A/B comparison (two branches, same corpus)?
- Did it define a task corpus before running?
- Did it measure: tokens in, tokens out, interactions, tool calls, success rate?
- Did it use the sensei benchmark infrastructure or improvise?
