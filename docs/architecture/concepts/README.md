# Concepts

The vocabulary you need to understand how Sensei steers an AI coding session. Three ideas do most of the work: **mindsets**, **personas**, and **agents**. They are easy to confuse, so this folder defines each one precisely and shows how they fit together.

| Doc | Concept | One-line definition |
|-----|---------|---------------------|
| [mindsets.md](./mindsets.md) | **Mindset** | A thinking lens the AI "wears" to approach work — analyst, developer, tester, and specialists. |
| [personas.md](./personas.md) | **Persona** | A project-specific *user* archetype you validate work against. |
| [agents.md](./agents.md) | **Agent** | The Claude Code subagent mechanism that *implements* mindsets and runs autonomous tasks. |
| [governance.md](./governance.md) | **Governance** | The layered rule hierarchy (scope × enforcement), promotion, and the hive-mind shared brain that decides which rules a session starts with. |

---

## The one-paragraph version

A **mindset** is a role the AI adopts — "think like an analyst," "think like a security reviewer." A **persona** is one of *your users* — "the non-technical admin," "the API consumer" — that the AI role-plays to check whether the work actually serves the people it's for. An **agent** is the underlying construct that makes both real: each mindset ships as a Claude Code subagent, and a generic persona-reviewer agent loads your personas on demand. Mindset = how the AI thinks; persona = who it builds for; agent = the engine that runs both.

---

## How they relate

```
                       ┌─────────────────────────────────────────────┐
                       │  AGENT  (Claude Code subagent mechanism)      │
                       │  isolated context · scoped tools · a report  │
                       └─────────────────────────────────────────────┘
                          ▲                                   ▲
            implements    │                                   │   implements
                          │                                   │
        ┌─────────────────┴───────────────┐        ┌──────────┴──────────────┐
        │  MINDSET  (a thinking lens)      │        │  persona-reviewer        │
        │  shipped 1:1 as an agent file    │        │  one generic agent that  │
        │                                  │        │  loads PERSONAS on demand │
        │  Core (in sequence):             │        └──────────┬──────────────┘
        │   Analyst → Developer → Tester   │                   │ reads
        │  Specialists (by domain):        │                   ▼
        │   UX · Security · Perf · DevOps  │        ┌─────────────────────────┐
        └──────────────────────────────────┘        │  PERSONA  (a user role)  │
                                                     │  .sensei/personas/*.md   │
                                                     │  project-defined,        │
                                                     │  evidence-grounded       │
                                                     └─────────────────────────┘
```

- **Mindsets** are *built-in* and universal. They ship inside the Sensei plugin as agent files (`marketplace/plugins/sensei/agents/*.md`) and apply to every project.
- **Personas** are *project-defined* and specific. They live in your repo at `.sensei/personas/*.md` and describe the humans who use *your* product.
- **Agents** are the *mechanism*. Every mindset is realized as one agent file; personas are realized through the single generic `persona-reviewer` agent.

---

## Where each lives

| Thing | Source of truth | Scope | Authored by |
|-------|-----------------|-------|-------------|
| Mindset | `marketplace/plugins/sensei/agents/<name>.md` | Universal (ships with plugin) | Sensei (you can add custom agents) |
| Persona | `.sensei/personas/<name>.md` | Per-project | You, grounded in real session failures |
| Agent | A `.md` file with frontmatter (`name`, `description`, `tools`, `model`) | Plugin-level or project-level (`.sensei/agents/`) | Sensei + you |

---

## Why this matters: FTR

Sensei's hero metric is **FTR — First-Time-Right**: a session where the assistant produces correct code without corrections. Mindsets and personas exist to raise it.

- **Mindsets** force *structured thinking*. Analyst before developer before tester means the AI understands the problem before it designs, and validates before it claims done — so it gets the code right the first time instead of after three corrections.
- **Personas** catch the *"works for me but not for the user"* class of failure. Code can pass every test and still confuse the admin or surprise the API consumer. Role-playing a real user surfaces that before you do.

Together they reduce corrections, which is exactly what FTR measures. See each doc for the detail.
