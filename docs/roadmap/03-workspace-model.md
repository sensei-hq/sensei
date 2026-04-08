# Workspace Model — Projects, Ideas, Phases, and Cards

---

## Core Principle: Flexible, Not Forced

The workspace does not impose a workflow. It provides **structures you can use** when
they help, and gets out of the way when they do not.

A developer can:
- Drop a raw idea note and never touch it again
- Work entirely in the Implementation phase without ever creating a requirements card
- Use only the prompt bar and built-in commands without organizing anything into cards
- Run a full requirements → design → implementation flow for a complex feature

All of these are valid. The tool tracks what exists without judging what is missing.

---

## Two Top-Level Containers

### Project

A **Project** is tied to a git repository. It has:
- A local path (or remote URL if not yet cloned)
- A detected language/stack
- A maturity level (derived from the state of its cards and code)
- One or more phases (optional — can be empty)
- A symbol graph (populated by `sensei index`)

### Idea

An **Idea** is pre-code. No repo yet. Just thinking. It has:
- A title and description
- Cards in any phases the developer chooses
- A maturity level
- An optional link to a Project (when it graduates to code)

Ideas and Projects are peers in the workspace. The developer sees both in the same list,
sorted and filtered however they prefer.

---

## Maturity States

Every Project and Idea has a maturity level. This is derived automatically from signals,
but the developer can override it.

| Level | Label | What it means |
|---|---|---|
| 0 | Seed | Title only, no content, no code |
| 1 | Sprout | Some notes or cards exist, no implementation |
| 2 | Growing | Requirements or design cards exist |
| 3 | Active | Implementation underway, code indexed |
| 4 | Stable | Core implementation done, tests present |
| 5 | Mature | Documentation, test coverage, no open gaps |

The maturity level surfaces in the project list as a visual indicator — not a grade,
just a signal for where the project is in its lifecycle. Useful when managing many
projects to see at a glance which ones need attention.

---

## Managing Many Projects

Developers working on multiple repos and ideas need a way to navigate without being
overwhelmed.

### Views

| View | What it shows |
|---|---|
| Recent | Last 5–10 touched (by any interaction: open, index, prompt, card edit) |
| Active | Projects with implementation underway (maturity 3+, recent session) |
| Ideas | All Ideas, sorted by last touched |
| All | Full list, searchable, filterable by stack / maturity / phase completion |
| Archived | Hidden from other views — work that is paused or abandoned |

### Indicators in the list

Each item in any view shows:
- Name and stack icon
- Maturity badge
- Last active (relative timestamp)
- Open cards count (cards without a resolution)
- Token cost last session (if any)

This gives enough information to decide where to focus without opening anything.

---

## Phases

A **Phase** is a named container for cards within a project or idea. Phases are optional.
If you do not create any, a project has a single default "Work" phase.

### Default phase set (suggested, not required)

| Phase | Purpose |
|---|---|
| Exploration | Raw ideas, questions, unknowns — fluid thinking |
| Requirements | What the system must do |
| Analysis | What exists, what is missing, what are the constraints |
| Design | How the system will work |
| Implementation | Tasks, work items, code-level decisions |
| Review | Post-implementation: gaps found, lessons, follow-ups |

The developer can rename, reorder, add, or remove any phase. They can also create
custom phases (e.g. "Research", "Spike", "Security Review").

The **Exploration phase** is specifically designed for fluid, unstructured thinking —
the stage where you are not sure what you are building yet. Cards here have no required
fields. They are a scratchpad.

### Phase completion signal

A phase is considered "complete" when all cards in it have a resolution (accepted,
deferred, or superseded). This is surfaced as a visual indicator, not enforced as a gate.
You can proceed to any phase at any time.

---

## Cards

A **Card** is the atomic unit of thinking in a phase. Every document, note, requirement,
decision, task, or finding is a card.

### Card anatomy

```
┌───────────────────────────────────────────────────────┐
│  [Phase badge]   Card title                           │
│                                                       │
│  Body (markdown — free-form prose or structured)      │
│                                                       │
│  Links:                                               │
│    → Card: [Req-04] User can log in with Google       │
│    → Symbol: auth.ts#signIn                           │
│    → File: src/routes/(app)/login/+page.svelte        │
│    → Decision: [ADR-03] Use Supabase Auth             │
│                                                       │
│  Tags: #auth #login #oauth                            │
│  Status: open | accepted | deferred | superseded      │
│  Created: 2026-04-08   Last touched: 2026-04-08       │
└───────────────────────────────────────────────────────┘
```

Links are typed and bidirectional. Creating a link from Card A to Symbol X also
creates a reverse edge in the graph (Symbol X is referenced by Card A).

### Card lifecycle

- **Open** — active, needs attention
- **Accepted** — content is confirmed and in use
- **Deferred** — acknowledged but not actioned now
- **Superseded** — replaced by another card (link to successor required)

Cards are never deleted — they are superseded or archived. This preserves the decision
trail.

---

## The Prompt Interface

The prompt bar is always available regardless of what is open. It operates in context:

| What is open | Prompt context |
|---|---|
| Nothing (workspace view) | Full workspace — queries span all projects |
| A project | Project scope — code graph + all cards for that project |
| A phase | Phase scope — cards in this phase + linked symbols |
| A card | Card scope — this card + linked cards + linked symbols |

Every response cites its sources. If the answer comes from a card, it links to that card.
If it comes from a code symbol, it links to the file and line. If it comes from a library
doc, it cites the library and section.

### Prompt modes

| Mode | Trigger | Behaviour |
|---|---|---|
| Ask | Default | Answers questions from the graph |
| Create | "create card..." | Creates a card in the current phase |
| Link | "link this to..." | Creates a typed link from the current card |
| Command | `/command` | Runs a built-in command (see below) |

---

## Built-In Commands

These are structured, deterministic operations. Not freeform prompts — they always
produce the same type of output given the same inputs.

| Command | Scope | What it does |
|---|---|---|
| `/gap-analysis` | Project | Cards in Requirements with no Implementation descendants |
| `/analyze-repo` | Project | Runs indexer, generates Analysis phase cards from symbol graph |
| `/trace [card-id]` | Card | Full chain: requirement → design → implementation → symbols |
| `/find-orphans` | Project | Symbols with no card references; cards with no symbol links |
| `/phase-summary` | Phase | Count and status of cards; open items; coverage estimate |
| `/design-review` | Project | Design cards checked against Requirements — conflicts, gaps |
| `/token-estimate [task]` | Card/Phase | Estimates Claude API cost to implement the described work |
| `/decision-log` | Project | All accepted Decision cards in chronological order |
| `/library-status` | Project | Libraries used, indexed status, drift warnings |
| `/session-recap` | Session | What changed, what cards were addressed, FTR score |

Commands can be scoped with arguments: `/gap-analysis --phase=Requirements --since=last-week`

---

## The Catalog View

The Catalog is a secondary view alongside the phase pipeline. It shows cards organized
by **type** rather than by phase:

- All Requirements cards across all phases
- All Design decisions
- All open items
- All superseded cards (archived view)
- All cards referencing a specific file or symbol

The Catalog is useful for cross-cutting questions: "show me all requirements that
reference the auth module" or "show me all open items across the whole project".

---

## Fluid Experimentation Mode

When an idea is genuinely unclear — a spike, a research thread, an experiment — the
developer can work in the **Exploration phase** without any structure.

In Exploration:
- Cards have no required fields — a title is enough
- No phase completion tracking
- The prompt bar works as a scratchpad: ask questions, capture thoughts, link to anything
- Outputs from experimentation can be promoted to cards in other phases when ready

Exploration is also the recommended starting point for any new feature or idea where
the requirements are not yet clear. Think on paper first, structure later.
