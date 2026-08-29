# 覚 · Observatory · Memories

**Segment:** 03 · Observatory — daily use
**Route:** `/memories`
**Source mockup:** [`lib/observatory/learnings-anatomy-v2.jsx`](../../mockups/Sensei/lib/observatory/learnings-anatomy-v2.jsx) → `LearningsAnatomyV2`
**App file:** `app/src/routes/(observatory)/memories/+page.svelte`
**Framing:** the human curation surface for [[pipeline/memory]] — the primary consumer is the LLM at session start.

## Purpose

Every session sensei watches produces evidence. That evidence
becomes memories — statements like *"use dbd for database schema
migrations"* or *"terse instructions preferred"*. The **primary
consumer** of a memory is not the human reading this screen — it's
the **LLM agent** at the start of the next relevant session, which
calls `get_memories(project=…)` and loads what applies (see
[[pipeline/memory]]).

This screen is the **human curation surface** for that pool. The
user comes here to:

1. **Understand** what memories the assistants are seeing — the
   anatomy view lays out What / Why / How / Where per memory.
2. **Widen or narrow scope.** A memory formed in one project may
   be useful across all your projects (`user`), across your org
   (`org`), or globally (`collective`). *"Use dbd"* might be a
   user-wide preference. *"The auth module uses refresh tokens"*
   stays project-specific. This is the **promotion ladder**.
3. **Reinforce or archive.** When a memory has become noise
   (skipped repeatedly) or wrong (contradicted), retire it.
4. **Edit.** Sharpen the What / Because text so the assistant
   reads a cleaner statement.

The default view is calm — a rail of memories sorted by strength
descending, one at a time in the stage. The user's actions
directly affect what the LLM sees in every subsequent session on
this project.

Kanji is 覚 — *awareness / to remember*.

## Data invariants

- `GET /api/memories` returns:
  ```json
  {
    "memories": [
      {
        "id": "…", "what": "…", "because": "…",
        "state": "proposed|reinforced|battle-tested|challenged|archived",
        "strength": 0.0..1.0,
        "category": "preference|convention|anti_pattern|pattern|correctness",
        "scope": { "level": "project|user|org|module|stack|global",
                   "project_id": "…"?, "user_id": "…"?, "org_id": "…"?,
                   "modules": ["…"]?, "stack": ["…"]? },
        "references": { "sessions": ["…"], "corrections": ["…"], "reinforced_in": ["…"] },
        "violated": integer,
        "reinforced": integer,
        "loaded_last_7d": integer,
        "followed_last_7d": integer,
        "skipped_last_7d": integer,
        "created_at": iso, "last_seen_at": iso,
        "signature": "…"
      }, …
    ],
    "counts": {
      "proposed": N, "reinforced": N, "battle-tested": N,
      "challenged": N, "archived": N
    }
  }
  ```
- The `loaded_last_7d` / `followed_last_7d` / `skipped_last_7d`
  fields come from the `activity.memory_loads` +
  `activity.memory_use_reports` telemetry (see
  [[pipeline/memory]] "Feedback"). They're what make this screen
  informative — not just "here are your memories" but "here's how
  the LLM actually treats each one".
- `inferHow(memory)` — the surface classification (skill / agent /
  command / inline rule / lint) — should move server-side so the
  glyph and target file are stable across every UI that renders
  memories.
- Every memory's `what` and `because` come through
  [[pipeline/narration-cache]] — templated fallback otherwise.

## Signals shown

### Header + toolbar (unchanged from prior draft)

| Element | Value |
|---|---|
| Header title | "Every memory has the same anatomy." |
| Health chart | 5-bar chart: proposed / reinforced / battle-tested / challenged / archived |
| Project filter | pills + count |
| Search input | filters `what` + `because` case-insensitively |
| Count line | `{filtered} of {total} memories` |

### Rail (side list)

| Element | Value |
|---|---|
| Rail item | surface glyph + `what` line (truncated) |
| Active state | left-border accent + muted background |
| Rail sort | `strength` desc; secondary by `state` (battle-tested first) |

### Stage — the anatomy view

Same shape for every memory — What / Why / How / Where — plus a
new **Usage** strip that surfaces the LLM feedback.

| Block | Content |
|---|---|
| Title (H2) | `memory.what` — model-generated |
| Meta strip | strength bar · state chip · category chip · scope-level chip |
| **Usage strip** (new) | `loaded {N} times · followed {M} · skipped {K}` in the last 7d |
| Why | `memory.because` — model-generated |
| How | surface kind (skill / agent / command / rule / lint) + target file (deterministic) |
| Where | scope-level chip + project/user/org name + module list + stack list |
| Provenance | link to session/correction rows that formed or reinforced this |

### Actions

Three primary buttons + an overflow menu.

**Primary trio (default depends on state):**

| State | Recommended primary | Others |
|---|---|---|
| `proposed` | Reinforce | Challenge, Archive |
| `reinforced` | Widen scope | Reinforce, Archive |
| `battle-tested` | Widen scope | Narrow, Archive |
| `challenged` | Reinforce | Archive, Edit |
| `skipped_repeatedly` (derived, not a formal state) | Archive | Edit, Widen |

**Widen-scope submenu** (from `Widen scope` button):

    project ─→ user ─→ org ─→ collective (global)

Each hop enters the appropriate governance queue (see
[[pipeline/governance]]) — user is instant, org enters the org's
Dōjō triage, collective enters the global triage. Attribution
rules apply automatically.

**Overflow menu:**
- Edit (What / Because)
- Narrow scope
- Copy signature
- Promote pattern from this memory
- Trace: why sensei formed this

## Done gate

- On Jerry's live data the anatomy stage renders one memory at a
  time with all four blocks (What / Why / How / Where) populated.
- The Usage strip shows integers matching
  `activity.memory_loads` and
  `activity.memory_use_reports` for the last 7 days.
- The rail is sorted by strength desc; the health chart matches
  the counts breakdown exactly.
- Search matches both `what` and `because`; project filter narrows
  correctly.
- The recommended primary verb depends on state (per table above).
  Clicking `Widen scope` opens the ladder submenu.
- `Widen to user` writes an immediate scope change (no triage);
  `Widen to org` enqueues a Dōjō triage record; `Widen to
  collective` enqueues a global triage record. See
  [[pipeline/dojo-lifecycle]] for the org/global paths.
- Narrowing drops the wider-scope reinforcement history (per the
  memory-pipeline invariant).
- Every memory's What/Because text comes from
  [[pipeline/narration-cache]]; fallback templates are visibly
  different from model output (labelled fallback in wire).
- Dark mode: the muted-open row and the accent border are
  distinguishable.

Optional check:
```
# Do the Usage counts on the stage match the raw telemetry?
psql -A -t -c "select memory_id, count(*) from activity.memory_loads
                where loaded_at > now() - interval '7 days'
                group by memory_id order by count(*) desc limit 5" -d sensei

# Does widening a memory scope actually take effect for the assistant?
curl -X POST http://localhost:7744/api/memories/$MEM_ID/scope \
     -H 'Content-Type: application/json' \
     -d '{"level":"user"}'
# Then in a new session:
mcp_call get_memories --project=other-project | jq '.memories | any(.id == $MEM_ID)'
# expected: true, because we widened it
```

## Wrong gate

- **Usage strip always reads 0 · 0 · 0.** Telemetry tables aren't
  being written (see [[pipeline/memory]] feedback path) OR the
  screen is reading a static fixture.
- **Widen scope silently succeeds but next session's
  `get_memories` doesn't return it in the wider scope.** Scope
  resolution regressed — the retrieval side isn't reading the new
  `scope_level`.
- **A `battle-tested` memory offers `Challenge` as its default
  action.** State-to-verb table wrong.
- **Widening to `collective` skipped the attribution/dereference
  step for client-project work.** Confidentiality regression —
  see [[pipeline/governance]] attribution table.
- **Rail entry glyph inconsistent between page loads.** `inferHow`
  isn't deterministic — move server-side.
- **All memories say the same What / Because copy.** Insight-copy
  cache-key collision.
- **Memory whose `scope.project_id` != current project appears
  under this project's filter.** Filter bug (a hard
  confidentiality regression if org-scoped Dōjō is active).
- **Widening a memory takes the old strength with it.** Wider
  scope needs fresh reinforcement to earn `battle-tested`.
- **`Archive` on a memory removes it from the LLM retrieval AND
  the anatomy view.** Archived memories should stay visible in
  the anatomy view (under an Archived filter) — hidden from the
  LLM only.

## Related

- [[pipeline/memory]] — the pipeline this screen curates
- [[pipeline/narration-cache]] — What / Because text
- [[pipeline/governance]] — the promotion ladder rules
- [[pipeline/dojo-lifecycle]] — org / collective widening path
- [[screen/observatory-insights]] — the triage that feeds new
  proposed memories into this screen
- [[screen/observatory-today]] — the adopted-lane surface (a
  subset of these memories)
- [[screen/project-memories]] — project-scoped variant with
  ready-to-share entry point
