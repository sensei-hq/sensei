# 律 · Pipeline · Governance

**Owner files:**
- Rules storage: `sensei.rules` + `sensei.rule_ladders`
- Read endpoints: `crates/senseid/src/api/handlers/rules.rs`
- MCP tool: `crates/mcp/src/tools/get_rules.rs`
- Resolver: `crates/senseid/src/rules/resolver.rs`
- Global governance: `~/.sensei/rules.md` (per-user global rules,
  loaded from disk on daemon boot)

## Purpose

Governance is *how the pair (developer + assistants) agrees on the
rules of engagement*. It exists at two scopes:

- **Personal Sensei** — a single user's rules for their own work.
  Loaded from `~/.sensei/rules.md`, augmented by rules the daemon
  learned (memories that got promoted to rule status).
- **Dōjō** (the SaaS extension, see
  [[pipeline/dojo-lifecycle]]) — the same primitives but shared
  across an org, with triage + approval.

Rules are not a flat list. The journey map §3.1 rejects a strict
hierarchy in favour of **priority ladders**: each concern
(security, architecture, testing, style, …) is its own ladder,
each rung has a priority tag (`P0 | P1 | P2 | P3`) and an impact
label. When two rules conflict, the higher rung wins regardless of
scope. Scope (company / team / project / repo / stack) rides along
as a tag, not as the structure.

Kanji is 律 — *rule / discipline*.

## Data invariants

### Ladders

- `sensei.rule_ladders` — one row per concern:
  - `id` uuid, `name` text (`Security & guards`, `Architecture`,
    `Testing`, `Style & conventions`, …), `kanji` text
    (`守 / 構 / 験 / 形`), `description` text.
- `sensei.rules` — one row per rung:
  - `id` uuid, `ladder_id` uuid, `priority` enum
    (`P0 | P1 | P2 | P3`), `impact` enum
    (`critical | high | med | low`), `text` text (the rule
    statement itself), `scope` jsonb (`{ level: 'user' | 'project'
    | 'org' | 'stack', project_id?, org_id?, stack? }`),
    `source` (`file:~/.sensei/rules.md` | `promoted:memory:{id}` |
    `dojo:{org_id}` | `user_added`), `state` (`active | archived`),
    `created_at`, `state_changed_at`.
- **Mandatory rules** — rules with a `mandatory: true` flag in the
  source file (or promoted with that flag) cannot be overridden by
  a more-specific scope. See `~/.sensei/rules.md` for the source
  of truth on personal-scope mandatories.

### Conflict resolution

The resolver runs on every `get_rules(scope)` MCP call:

1. Gather all rules matching the scope (project → user → org →
   stack, plus applicable global rules).
2. Group by ladder.
3. Within a ladder, sort by priority (`P0` > `P1` > `P2` > `P3`),
   then by impact (`critical` > … > `low`).
4. When two rules with the same priority + impact have
   contradictory statements, prefer the **tighter scope**
   (project > user > org > stack). Same rule as the
   [[pipeline/memory]] scope contract.
5. Mandatory rules cannot be overridden — they surface even if a
   tighter scope has a contradiction; the tighter scope's rule
   is flagged as a violation candidate.

### Rules from memory (promotion path)

The [[pipeline/memory]] promotion ladder feeds rules:

- A `battle-tested` memory with `category = anti_pattern` or
  `pattern` can be promoted to a rule with a chosen ladder +
  priority.
- The rule row's `source` records `promoted:memory:{id}` so the
  provenance is traceable.
- Retiring the source memory retires the rule (or moves it to
  archived, depending on user choice).

### Personal-scope global rules

`~/.sensei/rules.md` is the personal global rules file. On daemon
boot:

- Parse the file (markdown with `## Ladder / ### Rung` structure).
- Detect `mandatory:` flags on rungs.
- Upsert into `sensei.rules` with `scope.level = 'user'`,
  `source = 'file:~/.sensei/rules.md'`.
- On file change (watched by [[pipeline/capture]] root-watcher),
  reload.

## Signals produced

| Signal | Consumer |
|---|---|
| `get_rules(scope)` → ordered rule list | MCP; assistant loads at session start |
| Ladder view (per concern, per project) | Preferences → Rules panel |
| Rule violations (against captured behaviour) | Insights Now column violation cards |
| Recommendation candidates when a project would benefit from a rule the user hasn't added | [[pipeline/insights]] source #3 (persona/skill gaps → rule gaps) |
| Rule-derived guards | Dōjō artifacts (see [[pipeline/dojo-lifecycle]] artifact type #4) |

## Done gate

- On daemon boot, `~/.sensei/rules.md` is parsed and every rung
  ends up in `sensei.rules` with the right ladder + priority +
  scope + source.
- `get_rules(project=X)` returns the effective merged list for
  project X respecting the ladder ordering and scope precedence.
- Editing `~/.sensei/rules.md` while the daemon runs triggers a
  reload within the root-watcher debounce window.
- Promoted memories appear as rules with the correct `source`
  tag; retiring the memory archives the rule.
- Mandatory rules cannot be overridden by tighter scopes; a
  violating tighter-scope rule surfaces as a violation card.

Optional check:
```
mcp_call get_rules --project=sensei \
  | jq '.rules | group_by(.ladder) | map({ladder: .[0].ladder, ranked: [.[] | {priority, text}]})'

# is the personal rules file being watched?
touch ~/.sensei/rules.md
# then within a few seconds:
psql -A -t -c "select max(state_changed_at) from sensei.rules where source = 'file:~/.sensei/rules.md'" -d sensei
```

## Wrong gate

- **`get_rules` returns rules in insertion order, not priority.**
  Resolver isn't applying the ladder ordering.
- **A P2 project-scope rule wins over a P0 user-scope rule.**
  Priority comparison inverted.
- **A `mandatory` rule from `~/.sensei/rules.md` can be
  overridden by a per-project rule.** Mandatory bit not honored.
- **`~/.sensei/rules.md` edits require a daemon restart.**
  Watcher isn't hot-reloading.
- **Promoted memory that's later archived leaves the rule
  active.** Provenance link broken.
- **A rule cross-project rule leaks scope** (a project-scope rule
  shows in a different project's `get_rules`). Scope filter bug.
- **Ladder categorisation collapse** — all promoted memories land
  in a default `Style` ladder because the source memory doesn't
  carry a ladder hint.

## Related

- [[pipeline/memory]] — promotion path from memory → rule
- [[pipeline/insights]] — persona/skill-gap → rule-gap recommendation
- [[pipeline/dojo-lifecycle]] — org / collective governance layer
- [[pipeline/capture]] — watches `~/.sensei/rules.md` for changes
- [[screen/preferences]] — Rules panel
- [[project_governance_plane_design]] (memory) — earlier design notes
