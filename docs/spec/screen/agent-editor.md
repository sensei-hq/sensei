# 者 · Agent editor

**Segment:** 02 · Preferences / Extensions
**Route:** `/settings/extensions/agents/[id]`  (drill-down from the Extensions browser at `/settings/extensions`, kind filter `者 Agents`)
**Source mockup:** [`lib/observatory/agent-persona-editors.jsx`](../../mockups/Sensei/lib/observatory/agent-persona-editors.jsx) → `AgentEditor`
**Data:** reads/writes a subagent definition file — built-in agents at `marketplace/plugins/sensei/agents/*.md`, custom ones at `.sensei/agents/<name>.md` (YAML frontmatter + body). Replay fixtures read historical sessions from `sensei.sessions` / `sensei.hook_events`.
**App file:** _greenfield_
**Daemon files:** _greenfield_ — no HTTP handler for agent definitions exists; only the CLI references `.sensei/agents/` (`crates/cli/src/main.rs`).
**Status:** greenfield — the concept is documented ([[concepts/agents]]) and the file layout is defined, but nothing serves or edits agent definitions from the app yet. Replay-against-history is a net-new capability (no runner exists).

## Purpose

Edit one subagent — its **autonomy ceiling**, its **tool envelope**, and how it
*would have behaved* on real past sessions before you turn it loose. An agent is
the Claude Code subagent construct: it runs multi-step work with tools and picks
its own path *within an autonomy ceiling* ([[concepts/agents]]). This screen is
where the user sets that ceiling and earns trust in it.

The load-bearing idea is the **replay test panel** (right column). Rather than
trust a new agent definition blind, sensei reruns it against historical session
fixtures and shows pass / diverged with a reason. "Run all replays" is the gate
before publishing a new version — the same discipline the mockup's Save-adjacent
`{passing}/{total} replays passing` stat advertises.

Kanji is 者 — *the one who does* (agent).

## Data invariants

- The agent definition is a **markdown file** with YAML frontmatter
  (`name`, `description`, `tools`, `model`, `color`) plus a body of
  `## Mindset` / `## Procedure` / `## Report Format`, per [[concepts/agents]].
  Built-ins live in `marketplace/plugins/sensei/agents/*.md`; project-custom
  agents live in `.sensei/agents/<name>.md`. The file is the source of truth.
- **Autonomy ceiling** is one of four ordered levels (the mockup's
  `autonomyLevels`, low → high): `observe → suggest → confirm → autonomous`.
  Tool access **scales with the level** — the powers list is derived from the
  selected level, not edited independently, so a user can never grant `open PR`
  while the ceiling is `observe`.
- The **tool envelope** is a per-tool allow/deny list; **each allowed tool
  carries a rationale string** ("Needs to read every file in the module."). A
  tool cannot be `allowed: true` with an empty rationale.
- **Replay fixtures** are references to real past sessions, not synthetic
  prompts. Each fixture records the expected (`correctOutcome`) and the last
  result (`passed`, `steps`, `durationMs`, `toolCalls`, optional `divergence`).
  A diverged result must carry a `divergence` reason.
- `version` is monotonic; **Run all replays is the pre-publish gate** — a new
  version should not ship while fixtures are red without an explicit override.
- Autonomy and tool grants are a **guardrail, not a formality** — an agent
  scoped read-only can never write, mirroring the tool-scoping rule in
  [[concepts/agents]].

## Signals shown

Hero stats (mockup `AgMini` row):

| Element | Value | Meaning | Example |
|---|---|---|---|
| version | `v{A.version}` | current definition version (mono) | `v0.7.4` |
| replays passing | `{passing}/{total}` | fixtures green over total (accent) | `2/3` |
| Save | button | writes the definition file | — |

Left column:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Template | one of `A.templates` | starting shape for a new agent | `Multi-file refactor` · `Test author` · `SQL migration` · `Blank · custom` |
| Autonomy level card ×4 | `lvl.label` + `level {i+1}` + `lvl.rule` | pick the ceiling; selected card inverts to `bg-ink` | `観 Observe` · `勧 Suggest` · `確 Confirm each step` · `走 Autonomous` |
| Powers at this level | `cur.powers` chips | what the selected level unlocks | `read`, `write with confirm`, `git stage with confirm` (for Confirm) |
| Tool envelope row | ✓/✗ · `t.label` · `t.rationale` · on/off | each callable tool + why | `fs-read` · "Needs to read every file in the module." · on |

Autonomy levels (exact mockup content):

| id | Kanji | Label | Rule | Powers |
|---|---|---|---|---|
| `observe` | 観 | Observe | Watches and reasons. No suggestions surfaced. | read-only access |
| `suggest` | 勧 | Suggest | Surfaces proposals as memories. User pulls them in manually. | read-only access · writes to memories |
| `confirm` | 確 | Confirm each step | Executes one step at a time, prompting the user before each. | read · write with confirm · git stage with confirm |
| `autonomous` | 走 | Autonomous | Runs to completion within its tool envelope. Reports at the end. | read · write · git stage · open PR |

Right column — replay test panel:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Fixture row | dot + `f.label` + `f.when` + `{steps} steps` | one past session; green dot = passed, amber = diverged | `lumen-app · 2025-10-04 boundary-thrash` · 3 days ago · 14 steps |
| Fixture verdict | `passed` / `diverged` | outcome of the last replay | `passed` |
| Fixture detail grid | Expected outcome · Steps · Duration · Tool calls | run stats | `Single-PR cleanup…` · 14 · 8.4s · 22 |
| Why it diverged | `result.divergence` (amber left-border) | shown only for diverged fixtures | "Agent stopped at step 9 — couldn't decide between 1 PR or 3 PRs without user input. Autonomy level was 'observe'." |
| Replay → / View trace / + add fixture | buttons | rerun one fixture, inspect its trace, add a new one | — |
| Run all replays | button `Run {n} replays →` | pre-publish gate across every fixture | `Run 3 replays →` |

## Done gate

- The four autonomy cards render in order `observe → suggest → confirm →
  autonomous`; selecting one inverts it and updates the **Powers at this level**
  chips to that level's `powers` — powers are derived, never independently
  editable.
- Every allowed tool row shows its rationale; toggling a tool off greys it and
  flips the on/off label. A tool cannot be on with a blank rationale.
- Replay fixture list shows one row per historical session with the correct
  green/amber dot; clicking a row loads its detail grid, and a diverged fixture
  shows the "Why it diverged" block with its reason.
- "Run all replays" reruns every fixture against the *current* definition and
  updates the `{passing}/{total}` hero stat.
- Save writes the agent's markdown file (frontmatter + body); built-ins write to
  the plugin path, custom agents to `.sensei/agents/`.
- Dark mode: inverted (selected) autonomy card and the amber divergence block
  stay readable.

## Wrong gate

- **Powers list doesn't change when the autonomy level changes.** The powers
  are hard-coded instead of derived from the selected level — a user could then
  read `open PR` under an `observe` ceiling.
- **A tool is allowed with no rationale.** The rationale-required invariant
  regressed; the envelope is no longer self-documenting.
- **Replay fixtures are synthetic prompts, not real past sessions.** The panel
  claims "How would the agent behave on past sessions?" — fabricating fixtures
  breaks that promise. Fixtures must reference real `sessions`.
- **A diverged fixture shows no reason.** `divergence` is required whenever
  `passed=false`; a red dot with no explanation is a dead end.
- **Run all replays reports green while a fixture is red.** The pre-publish gate
  is lying; the `{passing}/{total}` stat and the per-fixture verdicts disagree.
- **Save lets you publish a new version with red replays and no override.** The
  gate is bypassable silently.
- **Autonomy ceiling and tool envelope disagree** — e.g. `autonomous` selected
  but `git`/`fs-write` forced off, or `observe` selected but write powers shown.
  The ceiling caps the envelope, not the other way around.

## Related

- [[concepts/agents]] — what an agent is, frontmatter, tool scoping, autonomy
- [[concepts/mindsets]] — the seven built-in mindset agents (agent files too)
- [[screen/persona-editor]] — sibling editor in the same mockup file
- [[screen/observatory-instruments-replay]] — the replay/trace surface this reuses conceptually
- [[screen/preferences]] — Extensions is a Preferences pane; this is a drill-down
