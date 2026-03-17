# Skill Description Auditor Design

> **Status:** approved
> **Date:** 2026-03-17

## Goal

A skill that reads every `SKILL.md` in the repo's `skills/` directory and `~/.claude/skills/`, scores each description against a weakness rubric, proposes stronger rewrites, and applies approved changes.

## Weakness Rubric

A description is **weak** if it matches any of these:

| Failure mode | Example |
|---|---|
| No "Use when" trigger | `"Helps with code patterns"` |
| Too vague — no specificity | `"Use for code tasks"` |
| Reactive-only | `"Use when something goes wrong"` — misses proactive triggers |
| Describes the skill not the trigger | `"This skill enforces zero errors"` |
| Single trigger, missing proactive case | Only says when to use defensively, not when to use proactively |
| Missing what the agent gains | No payoff — agent can't assess value before loading |

A description is **strong** if it:
- Opens with `Use when <specific situation>`
- Names at least one proactive trigger (before X, when starting Y)
- States what the agent gains (`— provides`, `— ensures`, `— prevents`)
- Is specific enough that two different skills wouldn't share it

**Strong template:**
```
Use when <specific situation> — <what the agent gets>.
Also use before <proactive trigger>.
```

---

## Procedure

```
name: auditing-skill-descriptions
description: Use when adding new skills, reviewing the skill library for
quality, or when skills aren't firing at the right moments — scores every
skill description against a weakness rubric and rewrites weak ones.
Also use periodically to keep the skill library sharp as the codebase evolves.
```

1. **Discover** — glob `skills/*/SKILL.md` (repo-local) and `~/.claude/skills/*.md` (global)
2. **Score each description** — apply rubric, classify as Strong / Weak / Missing
3. **Report** — print a table: skill name | current description (truncated) | verdict | failure modes
4. **Rewrite weak ones** — for each Weak/Missing: propose a rewritten description using the strong template, grounded in the skill's actual content
5. **Show side-by-side** — present current vs proposed for each
6. **Apply on approval** — user approves individually or all-at-once; skill rewrites the frontmatter `description` field in place

---

## Scoring Output Format

```
Skill                        Verdict   Issues
─────────────────────────────────────────────────────────────
zero-errors-policy           STRONG    —
managing-project-sessions    STRONG    —
detecting-doc-drift          WEAK      reactive-only, missing proactive trigger
decomposing-broad-tasks      WEAK      too vague
compressing-content          MISSING   no description field
```

---

## Rewrite Example

**`detecting-doc-drift`** (current):
```
Use when you suspect design docs are out of sync with code, before committing
changes that affect documented APIs or flows, or when setting up a repo to
continuously monitor docs.
```

**Proposed:**
```
Use before committing any change that touches a documented API, flow, or
design decision — catches doc drift before it reaches the repo. Also use
when design docs feel stale or when a feature has evolved significantly
since its spec was written.
```

---

## Scope

- Repo-local skills: `skills/*/SKILL.md`
- Global skills: `~/.claude/skills/*.md` (read-only audit unless user opts in to rewrite)
- Does NOT touch superpowers plugin skills (`~/.claude/plugins/`)

---

## Files Created/Modified

| File | Action |
|------|--------|
| `skills/auditing-skill-descriptions/SKILL.md` | Create |
