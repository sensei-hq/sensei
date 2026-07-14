# 貌 · Persona editor

**Segment:** 02 · Preferences / Extensions
**Route:** `/settings/extensions/personas/[id]`  (drill-down from the Extensions browser at `/settings/extensions`, kind filter `貌 Personas`)
**Source mockup:** [`lib/observatory/agent-persona-editors.jsx`](../../mockups/Sensei/lib/observatory/agent-persona-editors.jsx) → `PersonaEditor`
**Data:** reads/writes a persona file at `.sensei/personas/<name>.md` (YAML frontmatter: `name`, `description`, `goals`, `pain_points`, `validates`; freeform body). The **evidence trail** reads live from sensei's memory store (`sensei.memories`) linked by rule → session.
**App file:** _greenfield_
**Daemon files:** _greenfield_ — no HTTP handler for persona files exists; only the CLI references `.sensei/personas/` (`crates/cli/src/main.rs`). The `/sensei:persona` command authors these files today.
**Status:** greenfield — the persona concept, file layout, and the evidence-grounding loop are fully specified ([[concepts/personas]]), but nothing serves or edits them from the app. The evidence trail depends on the memory store being populated.

## Purpose

Edit one project **persona** — a project-specific *user archetype* you validate
work against ([[concepts/personas]]). A persona answers **who the work is for**;
this editor captures its stance (description), its **rules** (short imperatives
the persona embodies — the editable form of the persona's `goals` / `pain_points`
/ `validates`), and grounds each rule in an **evidence trail** pulled from
sensei's memory: the real sessions where that rule shaped sensei's response.

The mockup subtitle names the stance: *"Persona editor · the hat sensei wears."*
The three-column layout is: **triggers + assembled context** (when the hat goes
on, and what it costs in tokens) → **rules** (the imperatives) → **evidence
trail** for the selected rule (the receipts). Evidence grounding is what keeps a
persona honest — each rule points back to observed sessions, not a designer's
guess ([[concepts/personas]] §Evidence grounding).

Kanji is 貌 — *countenance / the face worn*.

## Data invariants

- The persona is a **markdown file** at `.sensei/personas/<name>.md` — machine-
  readable frontmatter (`name`, `description`, `goals`, `pain_points`,
  `validates`) plus a freeform body (journeys, scenarios). **The file is the
  source of truth**; editing here and editing the file must converge.
- `validates` is the **load-bearing** field — it turns "is this good for this
  user?" into concrete, checkable questions. In the editor, the rules list is the
  editable projection of that discipline; each rule is a short imperative
  ("Name the tradeoff. Always name the tradeoff.").
- **Triggers** are ANDed clauses (`kind op value`) describing when sensei dons
  this hat — e.g. `session-tag is "review"`, `file-pattern matches "**/api/**"`.
  A trigger is a structured predicate, never free prose.
- **Evidence is pulled live from the memory store**, not authored by hand. Each
  evidence row links a `memoryId` → the `sessionId` where the rule fired, with a
  snippet. A rule's `evidenceCount` must equal the number of memory rows that
  cite it. A rule with zero evidence shows the explicit empty state, never a
  fabricated citation.
- **Assembled context** is a preview of what sensei actually loads when this hat
  is active: `activeRules`, `memoryRefsLoaded`, `tokenEstimate`, and a
  `systemSnippet`. `tokenEstimate` is real (it's the token budget the persona
  costs), so a heavy persona is visible before it's enabled.
- Personas **supplement** mindsets, never replace them ([[concepts/personas]]);
  the editor covers *stance* ("what & why"), not method — the "What & why"
  textarea is explicitly labelled "covers stance, not method".

## Signals shown

Hero stats (mockup `AgMini` row):

| Element | Value | Meaning | Example |
|---|---|---|---|
| rules | `P.rules.length` | count of imperatives | `5` |
| evidence cited | `P.evidence.length` | memory rows backing the rules (mono) | `4` |
| tokens | `P.assembled.tokenEstimate` | context cost when the hat is on (accent) | `2,240` |

Column 1 — Triggers + assembled context:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Trigger card | `t.label` + `{kind} {op} "{value}"` | ANDed clause; when sensei dons the hat | `Code review session` · `session-tag is "review"` |
| What & why | textarea = `P.description` | persona stance (not method) | "Optimises for boundaries, change blast-radius, and long-term legibility…" |
| Assembled context | Active rules · Memory refs loaded · Token estimate + `systemSnippet` | the live context preview | `5` · `12` · `2,240` |

Column 2 — Rules:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Rule card | `r.id` (accent) + `{n} citations` + `r.text` + `last fired {when}` | one imperative; selected card highlights | `R1` · `18 citations` · "When a change crosses a module boundary, ask why before how." · last fired 2 days ago |
| + add rule | dashed button | append a new imperative | — |

Column 3 — Evidence trail (scoped to the selected rule):

| Element | Value | Meaning | Example |
|---|---|---|---|
| Trail header | `Evidence trail · {rule.id}` + the rule text | which rule these receipts back | `Evidence trail · R1` |
| Evidence card | `e.memoryId` (accent) + `e.when` + `e.snippet` + `sessionId` link + `view memory →` | a real session where the rule shaped the response | `mem-1207` · 2 days ago · "…sensei pushed back: 'why does shell need this canvas type at all?' Outcome: a smaller fix." · `sess-9924` |
| Empty state | "No evidence cited for this rule yet." | rule has zero backing memories | shown for R4 (evidenceCount 6 but no seeded rows) |

## Done gate

- The three columns render: triggers + assembled context, rules, evidence trail.
- Selecting a rule scopes column 3 to that rule's evidence; each evidence row
  shows a `memoryId`, a snippet, and links to its `sessionId` and the memory.
- A rule with no backing memory shows the explicit empty state — never a
  fabricated citation.
- Each rule's `{n} citations` count matches the number of evidence rows that
  cite it (pulled from the memory store, not stored on the rule).
- "What & why" edits the persona `description`; the textarea is labelled as
  covering stance, not method.
- Assembled-context `tokenEstimate` reflects the real context cost and updates
  as rules/memory refs change.
- Save writes `.sensei/personas/<name>.md` (frontmatter + body); re-reading the
  file round-trips the same content.
- Evidence is read live from the memory store; it is not editable in place (it's
  a receipt, not a field).

## Wrong gate

- **Evidence rows are hand-authored, not pulled from memory.** The panel claims
  "Pulled live from sensei's memory store" — inventing citations breaks the
  evidence-grounding contract ([[concepts/personas]]). Each row must reference a
  real `memories` row and its `sessionId`.
- **A rule shows citations but the evidence panel is empty (or vice versa).**
  `evidenceCount` and the actual memory rows disagree; the count is stale or
  faked.
- **The editor edits method instead of stance.** Personas cover who + why, not
  how; if the editor starts capturing procedures it has drifted into mindset
  territory.
- **Triggers stored as free text.** Triggers are structured `kind op value`
  predicates; a prose blob can't be evaluated to don the hat.
- **`validates` criteria are lost.** The rules projection must preserve the
  persona's checkable criteria — dropping them guts the load-bearing part.
- **Token estimate is fabricated or zero** while rules and memory refs are
  loaded. A user can't judge whether a persona is worth its context cost.
- **Save diverges from the file.** Editing here and editing
  `.sensei/personas/<name>.md` by hand produce different personas — the file is
  supposed to be the single source of truth.

## Related

- [[concepts/personas]] — what a persona is, the file layout, evidence grounding
- [[concepts/agents]] — the generic `sensei-persona-reviewer` that consumes these
- [[screen/agent-editor]] — sibling editor in the same mockup file
- [[screen/observatory-memories]] — the memory store the evidence trail reads from
- [[screen/preferences]] — Extensions is a Preferences pane; this is a drill-down
