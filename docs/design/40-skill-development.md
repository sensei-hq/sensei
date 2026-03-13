---
id: skill-development
type: design
implements:
  - feature: skill-tooling
    items: [skill-creation, skill-testing, skill-installation]
---

# Skill Development

## Overview

Skills are markdown files that agents load to follow a consistent protocol, apply a repeatable technique, or access reference knowledge. They live in the project repo and can be installed globally so every future session — in any project — benefits from them. Sensei provides the structure and methodology for developing skills that actually work: a test-driven process (TDD-for-skills) that validates a skill against realistic pressure before it is promoted. Superpowers provides the runtime that discovers and loads installed skills. The two work together: sensei hosts the skill source, defines the development workflow, and coordinates installation; superpowers makes skills available to the agent at session start.

---

## Skill Types

Three skill types cover the full range of agent behaviour that skills can encode.

| Type | Purpose | When to create |
|------|---------|----------------|
| Discipline | Enforces rules the agent must always follow | When baseline testing shows agents rationalise their way out of a behaviour under pressure (e.g., skipping a safety check, not reading a file before editing it) |
| Technique | Teaches a repeatable how-to process | When an agent can complete a task but does so inconsistently or misses systematic steps (e.g., how to write a feature doc, how to run the indexing pipeline) |
| Reference | Provides authoritative API or domain knowledge | When an agent repeatedly looks up the same information or gets details wrong (e.g., MCP tool signatures, project-specific conventions) |

Each type has different testing requirements and different token budget targets. Choosing the wrong type leads to a skill that is either too prescriptive (a technique used where a discipline was needed) or too permissive (a reference used where an agent needed a protocol).

---

## TDD-for-Skills Methodology

Skills are developed through four phases. Each phase has a defined input, a defined output, and a clear gate before the next phase begins.

### Phase 1 — RED (Baseline)

**Input:** A description of the target behaviour or protocol.

**Process:** The agent attempts the task without any skill loaded. The agent records every failure: shortcuts taken, rationalisations made, steps skipped, edge cases ignored.

**Output:** A baseline report documenting each failure mode. This report is the benchmark. A skill that does not prevent the documented failures is not done.

**Gate:** Baseline must show at least one clear failure before writing begins. If the agent already behaves correctly without a skill, no skill is needed.

---

### Phase 2 — GREEN (Write Skill)

**Input:** The baseline report.

**Process:** The agent writes a skill that directly addresses each failure mode from the baseline. The skill follows the required file format (see below). Content is written to be actionable under pressure — not a description of good intent, but a protocol the agent can follow when conditions are difficult.

**Output:** A `SKILL.md` file in `skills/<name>/` that passes a structural validity check (frontmatter present, required sections present, word count within budget).

**Gate:** The skill must address each baseline failure by name. A skill that does not reference the failure modes it was written to fix is not ready for pressure testing.

---

### Phase 3 — REFACTOR (Pressure Test)

**Input:** The written skill.

**Process:** The agent loads the skill and attempts the target task under combined pressure scenarios. Each scenario must combine at least three simultaneous pressures drawn from: time pressure, sunk cost (work already done), authority pressure (someone senior asking for a shortcut), urgency framing, partial completion pressure, and technical complexity pressure.

For each scenario the agent records:
- Whether the skill was cited before proceeding
- Whether the protocol was followed in full
- If the protocol was violated: the rationalisation used

Failures are documented as new failure modes and fed back into the skill content. The skill is rewritten and the pressure suite is re-run. This loop continues until the stopping condition is reached.

**Output:** A pressure test log documenting: scenarios attempted, compliance result per scenario, rationalisations observed, and skill revisions made.

**Stopping condition:** The agent cites the skill and follows the full protocol under maximum pressure (all three or more pressures simultaneously active) in the final run. A skill that achieves this is considered bulletproof.

**Gate:** No skill advances to installation until it reaches the stopping condition.

---

### Phase 4 — INSTALL

**Input:** A bulletproof skill that has passed the pressure test.

**Process:** The skill is installed globally (see Installation below). The installation is verified by starting a fresh session and confirming the skill appears in discovery.

**Output:** A globally installed skill available in all future sessions.

---

## Skill File Format

Every skill is a directory containing a required `SKILL.md` and optional supporting files.

```
skills/
  <name>/
    SKILL.md              Required. Main skill content.
    <supporting-file>     Optional. Only for reference material exceeding 100 lines.
```

### SKILL.md Structure

```markdown
---
name: skill-name-with-hyphens
description: Use when [specific triggering conditions — no workflow summary].
---

# Skill Name

## Overview
Core principle in 1–2 sentences.

## When to Use
- Bullet list of symptoms and triggering conditions
- When NOT to use

## Protocol / Content
The actual technique, rules, or reference material.

## Common Mistakes
What goes wrong and how to fix it.
```

### Frontmatter Rules

- `name`: letters, numbers, hyphens only — no spaces or special characters.
- `description`: starts with "Use when". Describes triggering conditions only. No workflow summary. Under 500 characters.
- Only `name` and `description` are valid frontmatter fields — no additional fields.

### Token Budget by Type

| Type | Target word count |
|------|------------------|
| Discipline (loaded every session) | < 150 words |
| Technique (frequently used) | < 300 words |
| Reference (on-demand) | < 500 words |

Exceeding budget is a signal that the skill is trying to do too much. Split it or move heavy reference material to a supporting file.

---

## Testing Approach

### Pressure Scenarios

Each pressure scenario is a realistic situation designed to make the agent rationalise away the skill protocol. Effective scenarios:

- Combine at least three simultaneous pressures
- Use realistic framing (a real-sounding task, not an abstract test)
- Target the specific failure modes documented in the baseline

**Example pressures to combine:**

| Pressure | Framing |
|----------|---------|
| Time pressure | "This is blocking a release" |
| Sunk cost | "You've already done 80% of the work" |
| Authority | "The tech lead said it's fine to skip this" |
| Urgency | "Customer is waiting" |
| Partial completion | "Just finish this one edge case without the full protocol" |
| Complexity | "The normal approach doesn't apply here because..." |

### Documenting Rationalisations

Every rationalisation the agent uses to bypass the skill protocol is a candidate for explicit rebuttal in the skill content. If the agent says "this case is different because X", the skill should address X directly.

Rationalisations are documented in the pressure test log as:
```
Scenario: [description]
Pressures: [list applied]
Result: [complied / violated]
Rationalisation (if violated): [verbatim]
Skill revision: [what changed in response]
```

### Stopping Condition

A skill is bulletproof when:
1. The agent cites the skill by name before proceeding in the final pressure run
2. The agent follows the full protocol without omitting any step
3. This result holds under the maximum combined pressure scenario

A skill that achieves partial compliance (follows the protocol but does not cite the skill) is not done — citation confirms the agent loaded and applied the skill rather than arriving at the right behaviour by chance.

---

## Installation

Skills live in the project repo and are installed globally by symlinking into the global skills directory.

```
Project:  skills/<name>/SKILL.md
Global:   ~/.claude/skills/<name>/
```

The symlink means project edits are immediately reflected globally — there is no separate sync step. Re-running the install script after adding a new skill or changing a skill name updates the global directory.

Skills installed globally are available in all projects via the superpowers plugin discovery mechanism. No per-project configuration is required after installation.

---

## Relationship to `decomposing-broad-tasks`

`docs/superpowers/specs/2026-03-13-decomposing-broad-tasks-design.md` is the first skill designed using this TDD-for-skills workflow. It serves as the reference example for the full cycle: the baseline was run to document how agents decompose broad tasks without guidance, the skill was written to address the documented failures, and the pressure test was used to validate compliance under realistic working conditions. Reviewing that spec alongside this document illustrates how the methodology is applied in practice.
