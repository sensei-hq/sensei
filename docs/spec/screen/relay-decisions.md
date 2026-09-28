# 択 · Relay · Decisions

**Segment:** 05 · Relay (mobile companion · planner)
**Route:** Relay iOS app · Decisions (no web route)
**Source mockup:** [`lib/relay/relay-planner.jsx`](../../mockups/Sensei/lib/relay/relay-planner.jsx) → `RelayDecisions`
**Data:** _greenfield_ — `GET /api/relay/decisions` (coordinator-published gate prompts): the open `decide`-gates across all projects, each an `eyebrow` (`{project} · phase {n}`), a `question`, an ordered list of 3–4 `options` (`[label, hint]`), and a free-reply slot. `POST /api/relay/decisions/{id}` carries back the chosen option **or** the typed reply.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + planner model not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

The human-in-the-loop inbox for **decide-gates** — the way sensei asks. When
a plan reaches a step that needs a judgement call, the coordinator raises a
decision: a single clear question, **3–4 options** each with a short hint,
plus a **type-your-own** reply for when none fits. Answering is
**non-blocking** — the subhead says so (`Answer when you can — other tracks
keep moving`), and unanswered decisions don't halt the other projects. The
user picks (or types), then sends the batch; the send button counts how many
are answered vs left (`Send 1 answer · 1 left`).

Kanji is 択 — *taku / choose*, the act of selecting.

## Data invariants

The decide-gate model this screen renders:

- **A decide-gate = one question + 3–4 options + a free reply.** This is
  the fixed shape (R4). Fewer than 2 options, or no free-reply slot, is an
  invalid gate — a decision the human can't actually answer.
- **Every option is `[label, hint]`.** The hint is optional (`''`), a
  short qualifier sensei attaches (`simplest`, `revocable`,
  `recommended`, `42 accounts`). A `recommended` hint marks the
  coordinator's suggestion but never pre-selects.
- **Selection is exclusive per decision.** At most one option `chosen`, or
  the free reply filled — not both. An unanswered decision has
  `chosen = -1` and an empty reply; it is valid to leave and send later.
- **Non-blocking.** Decisions belong to different projects/phases and are
  answered independently. Sending answers resolves the chosen gates and
  the coordinator resumes those tracks (R2); the rest stay open.
- **Filtered status only.** The `question` and `options` are plan-level
  choices (`Which session strategy?`), never code. The relay carries the
  prompt and the reply — never the transcript that produced it (R5).
- **Attribution (team).** On a team relay the answer carries who decided
  and lands in the Dōjō record (R8); solo, attribution is the paired user.

## Signals shown

Hero:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Back | ← | return to Projects | — |
| Waiting count | `{n} waiting` (accent, mono) | open decisions across projects | `2 waiting` |
| Title | `Decisions` | screen name | `Decisions` |
| Subhead | non-blocking reassurance | answering is optional/deferred (R2) | `Answer when you can — other tracks keep moving.` |

Per-decision card (`DecisionQ`):

| Element | Value | Meaning | Example |
|---|---|---|---|
| Eyebrow | `{project} · phase {n}` | which plan + phase raised it | `lumen-auth · phase 2` |
| Question | `question` | the single judgement call | `Which session strategy should sensei use?` |
| Option | `label` + `· hint` | one of 3–4 choices; hint is muted | `Hybrid — JWT + refresh store · recommended` |
| Selected option | filled accent radio + check | the chosen option | (option 3 selected) |
| Divider | `OR` | separates options from free reply | `OR` |
| Free reply | typed text or placeholder | type-your-own answer | `Type your answer…` |

Send bar:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Send button | `Send {answered} answer · {left} left` | commit answered decisions; count of remaining | `Send 1 answer · 1 left` |

Worked example (the two mockup decisions):

- `lumen-auth · phase 2` — *Which session strategy should sensei use?* —
  options: **Stateless JWT** (simplest) · **Server sessions** (revocable) ·
  **Hybrid — JWT + refresh store** (recommended) — chosen: option 3.
- `billing-svc · phase 3` — *Roll invoicing v2 out to which cohort first?*
  — options: **Internal only** · **5% of users** · **Beta list** (42
  accounts) — chosen: none yet (free reply empty).
- Send bar: `Send 1 answer · 1 left` (1 of 2 answered).

## Done gate

- Each card shows one question, its 3–4 options with hints, an `OR`
  divider, and a free-reply slot — the full R4 shape, every time.
- Exactly one option can be selected per decision, or the free reply
  filled; selecting an option and typing are mutually exclusive.
- The waiting count in the hero equals the number of unanswered decisions;
  the send button reads `Send {answered} answer · {left} left` and matches.
- Leaving a decision unanswered does not block or grey out the others
  (R2) — the subhead's promise holds.
- A `recommended` hint is shown but never auto-selects the option.
- Question + options are plan-level text only — no code, no diff, no
  transcript crosses the relay (R5).
- Dark mode: the selected-option accent fill and the free-reply well stay
  readable and distinct from unselected options.

## Wrong gate

- **A decision with no options and no free reply.** The human is asked to
  decide but given nothing to choose and nowhere to type — the gate is a
  dead end (R4 violated).
- **Only one option, or five+.** The shape is 3–4 options plus a free
  reply; one option isn't a decision, and a long list should have been a
  free-reply prompt.
- **Answering one decision blocks the others** (spinner/disabled on the
  whole screen) — the run is no longer non-blocking (R2).
- **`recommended` is pre-selected on load.** The coordinator's suggestion
  became a silent auto-answer; the human never actually decided.
- **The question or an option contains code / a diff.** Zero-knowledge is
  broken — only the plan-level choice may cross the relay (R5).
- **Send commits unanswered decisions** (or the count disagrees with what's
  selected) — the human sends a choice they didn't make.
- **Team gate lands with no attribution** — a decided gate reaches the
  Dōjō record without who-decided (R8).

## Related

- [[architecture/relay]] — gates: approve (exact command) vs decide (3–4 options + free reply)
- [[journeys/relay]] — the run & supervise round-trip · team on-call queue
- [[screen/relay-projects]] — a `gate` verdict card leads here
- [[screen/relay-plan]] — the plan item that raised the decision
- [[pipeline/narration-cache]] — mentor-voice for questions + hints
