# Personas

> A persona is a project-specific *user* archetype you validate work against. See also [mindsets](./mindsets.md) and [agents](./agents.md).

## What a persona is

A persona is one of the people who use your product, captured as a profile: who they are, what they're trying to do, what frustrates them, and the questions that tell you whether the work serves them. A healthcare app has patients, doctors, and admins. A developer tool has end users, plugin authors, and API consumers. Each wants different things and breaks in different ways.

Without persona awareness, the AI builds for an abstract "user" — and produces code that passes every test yet confuses the actual admin or surprises the actual API consumer. A persona makes that abstract user concrete so the AI (and you) can ask: *does this work for them?*

Personas answer **who the work is for**. That distinguishes them from [mindsets](./mindsets.md), which answer **how to approach the work** — see [Personas vs. mindsets](#personas-vs-mindsets) below.

## Where personas live

Personas are **per-project** and live in your repository:

```
.sensei/
  personas/
    end-user.md
    admin.md
    api-consumer.md
```

Each persona is a **markdown file** (not YAML) with structured frontmatter plus freeform body content. The frontmatter is machine-readable; the body holds the rich material — journey maps, scenarios, real frustrations — that a structured schema can't capture.

```markdown
---
name: End User
description: Non-technical user accessing the dashboard
goals:
  - See quality trends
  - Understand what went wrong
  - Get actionable advice
pain_points:
  - Too much jargon
  - No guidance on next steps
validates:
  - Is the language plain?
  - Is there a clear call to action?
  - Does it load in <2s?
---

# End User

Freeform content: journey maps, examples, scenarios, acceptance stories.
```

| Field | Purpose |
|-------|---------|
| `name` | Display name of the persona |
| `description` | One sentence: who they are |
| `goals` | What they're trying to accomplish |
| `pain_points` | What frustrates them — what to watch for |
| `validates` | The checklist: questions that decide whether work serves this persona |

The `validates` list is the load-bearing part. It turns "is this good for the end user?" into concrete, checkable criteria.

## How personas are authored

Use the `/sensei:persona` command:

- **`/sensei:persona list`** — show defined personas, their descriptions and goals, and which (if any) is active.
- **`/sensei:persona add <name>`** — create one. Sensei asks three questions conversationally — *Who is this persona, in one sentence? What are their top three goals? What frustrates them, what should we watch for?* — and writes `.sensei/personas/<name>.md`.
- **`/sensei:persona switch <name>`** — make a persona active for the session, so the AI views decisions through their eyes.
- **`/sensei:persona validate`** — evaluate the current work against the active persona's `validates` criteria (or all personas if none is active).

You can also edit the markdown directly — the file is the source of truth.

## How personas are used

1. **At session start.** Sensei injects a summary of your project personas — each one's name, description, and `validates` criteria — into the session context. From the first turn, the AI knows who it's building for.

2. **During design and review.** When a decision affects users, the AI considers each persona's goals and pain points. A choice that's fine for a developer may confuse an end user; surfacing that *during* design is cheaper than discovering it after.

3. **For validation, via the persona-reviewer agent.** The generic `persona-reviewer` ([agent](./agents.md)) loads every persona from `.sensei/personas/` and walks the changed code from each one's perspective — checking each `validates` criterion, flagging triggered pain points, and surfacing *cross-persona conflicts* (persona A needs X, persona B needs Y). Invoke it with `/sensei:agent use sensei-persona-reviewer`, optionally naming one persona to focus on.

## Evidence grounding

Personas are not invented in a vacuum. The strongest ones are **grounded in real session failures** — the moments where the assistant produced something that worked technically but failed a real user. When sessions cluster around a user-experience correction ("the API returned a 500 instead of a 422"), that's evidence: it tells you a persona is missing or its criteria are too thin. You add or sharpen the persona (e.g. give "API Consumer" a *validates* criterion about correct status codes), and link the sessions that inspired it.

This keeps personas honest. Each one points back to observed failures rather than to a designer's guess about who might use the product. As Sensei observes more sessions, it can recommend new personas where coverage is thin ("new module *payments* has no persona validating it") — closing the loop between what went wrong and the archetype that would have caught it.

## Personas vs. mindsets

They are complementary, not competing — and personas **supplement** mindsets, they never replace them.

| | Mindset | Persona |
|---|---------|---------|
| **Answers** | How should I approach this work? | Who am I building for? |
| **Scope** | Universal — ships with the plugin | Project-specific — defined in your repo |
| **Examples** | Analyst, Developer, Tester, Security, UX | End User, Admin, API Consumer |
| **Source** | `marketplace/plugins/sensei/agents/*.md` | `.sensei/personas/*.md` |
| **Fixed questions?** | Yes — each mindset has its own | No — questions come from your `validates` lists |

The core mindsets (analyst → developer → acceptance tester) apply to *every* task. Specialist mindsets apply by domain. Personas add a project-specific user perspective *on top* of all of them. When you validate, consider each persona independently.

## Why personas tie to FTR

Sensei's hero metric is **FTR (First-Time-Right)**. Mindsets catch *implementation* failures; personas catch the **"works for me but not for the user"** class — code that's correct by every automated measure yet wrong for the people it's for. Those failures otherwise surface late, as corrections in review or complaints in production. Role-playing a real user before declaring done turns a future correction into a caught-early fix, which is exactly what raises FTR.

---

**Related:** [mindsets.md](./mindsets.md) · [agents.md](./agents.md) · [concepts/README.md](./README.md)
