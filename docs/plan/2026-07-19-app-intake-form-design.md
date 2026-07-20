---
name: Sensei-app intake form — design
date: 2026-07-19
status: design — approved in brainstorm 2026-07-19; plan next
spec: docs/plan/operating-model.md §3.3 (front door — intake conversation → recommend-and-confirm)
phase: Operating-model Phase 2 — front-door follow-up #3 (item 3b of §9→auto-select→nudge/app)
---

# Sensei-app intake form — design

The **second** Sensei surface for the front door. The first is the CLI/agent `/sensei:intake`
(conversational). This is the per-user **app** surface: describe a work chunk in freeform, the app
classifies it + recommends a playbook, and on confirm **records the run** (`playbook_run`) — the
structured counterpart to the conversation. Reuses the front-door backend already shipped
(`/api/playbook/guide`, `/api/playbook/recommend`, `classify_chunk`, `is_trusted`); one small
backward-compatible daemon change (a `preview` flag).

## Decisions (brainstorm 2026-07-19)

1. **Build against the Zen-Sumi catalog + the existing wizard pattern** — no separate mockup cycle.
   Mirror `app/src/routes/(config)/setup` + `WizardRail.svelte` + canonical tokens
   (`docs/architecture/frontend-svelte-guidelines.md`). Visual verification is flagged for Jerry
   (Tauri app — cannot be eyeballed autonomously); `svelte-check` + unit tests gate it structurally.
2. **Freeform → classify → recommend** interaction (not axis pickers, not hybrid). One text box; the
   daemon's `classify_chunk` derives the axes; the card shows what it inferred.
3. **Persist on confirm.** The form records the chosen playbook. To avoid the CLI's current
   double-insert (recommend leg inserts `confirmed=false`, confirm leg inserts `confirmed=true`), the
   form's recommend leg runs in **preview** mode (no insert) and the confirm leg persists exactly one
   row. Requires a `preview` flag on `recommend_playbook` (below).

## Placement

New route `app/src/routes/(observatory)/intake/+page.svelte` + an "Intake" entry in the observatory
nav (the front door — listed first). The observatory group is the per-user home; intake is where a
chunk of work starts, so it leads.

## Flow (freeform → classify → recommend → confirm-persist)

1. **On load:** `GET /api/playbook/guide` → use `frame` as the intro copy above the text box (grounds
   the user in what "a chunk of work" means).
2. **Describe:** a textarea ("Describe the work chunk"). Submit → `POST /api/playbook/recommend
   { chunk, preview: true }`. The daemon classifies (gateway + heuristic fallback) and recommends
   **without persisting**. Response carries: `playbook`, `rationale`, the classified `lifecycle`,
   `intent`, `risk`, `opening_tone`, `when_to_use`, `auto_select`, `trust {n, ftr}`.
3. **Recommendation card:** the playbook **title** (looked up from the guide's `playbooks[]` by the
   response's `playbook` id) + one-line `rationale` + the `opening_tone` line + the **inferred axes**
   (lifecycle / intent / risk chips, so the user sees what it read) + a "trusted — would auto-select
   (FTR X over N)" badge when `auto_select` is true.
4. **Confirm:** a Confirm button → `POST /api/playbook/recommend
   { lifecycle, intent, risk, confirm: true, session_id?, feature? }`. This **reuses the classified
   axes** from step 2 (no re-classify, no second gateway call, deterministic) → persists exactly one
   confirmed `playbook_run`. The card flips to a "recorded" state.
   - **Auto-select:** when step 2 returns `auto_select: true`, the form fires the confirm
     automatically and announces "auto-selected **<playbook>** — reliable for this kind of chunk
     (FTR X over N)", mirroring the CLI. High-risk never auto-selects (the daemon only sets
     `auto_select` for low-risk), so high-risk always shows the explicit Confirm button.

## Backend change (small, backward-compatible)

`recommend_playbook` (`crates/senseid/src/api/handlers/playbook.rs`) gains a `preview: bool` body
field:

- **`preview == true`:** skip `insert_playbook_run` entirely (classify + recommend + trust only).
  No row is written; the response is unchanged otherwise.
- **`preview` absent / false:** current behavior (persist a run with `confirmed = parse_confirm(...)`).

The response **also** returns the classified axes so the form can display them and drive the confirm
call: add `"lifecycle"`, `"intent"`, `"risk"` (the `as_str()` labels) to the existing
`serde_json::json!({...})`. This is additive — the CLI and existing callers are unaffected.

## Session

The confirm leg passes the app's active sensei session id if the app has one, else `null`
(`insert_playbook_run` already accepts `Option<Uuid>`). Session-less runs are **recorded** (they show
in the decision log / future history view) but are **not** FTR-attributed — §9 learning still comes
from live coding sessions (the CLI path). Creating or associating a session from the form is
deferred (see Scope).

## Data (already shipped — no change)

- `GET /api/playbook/guide` → `{ frame, axes[], playbooks[] }` (frame + per-axis prompts + catalog).
- `POST /api/playbook/recommend` → recommendation JSON (see step 2), now with `preview` + the axes.

## Units & interfaces (isolation)

| Unit | Responsibility | Interface | Depends on |
|---|---|---|---|
| `recommend_playbook` `preview` + axes | preview mode (no insert) + expose classified axes | handler → response | existing handler, `insert_playbook_run` |
| `intake.svelte.ts` | fetch guide; recommend(preview); confirm; hold state | Svelte 5 runes module | `api.ts` fetch pattern |
| `(observatory)/intake/+page.svelte` | render frame + textarea + recommendation card | route component | `intake.svelte.ts`, tokens |
| observatory nav entry | link to `/intake` | nav markup | existing nav |

## Testing

- **Rust (`recommend_playbook`):** `preview: true` → no `playbook_run` row inserted + response
  carries `lifecycle`/`intent`/`risk`; `confirm: true` (no preview) → exactly one `confirmed=true`
  row. (DB test mirrors the existing playbook handler tests; `sensei_test`.)
- **`intake.svelte.ts` (unit, mocked fetch):** preview submit → parses the recommendation into card
  state; confirm → POSTs the reused axes with `confirm:true`; `auto_select:true` → confirm fires
  without a manual click.
- **`svelte-check`** clean; Svelte MCP autofixer over the `.svelte` files.
- **Visual:** flagged for Jerry via `make app-dev` (Tauri) — the app UI does not render in bare Vite.

## Scope / deferred

**In:** the `preview` flag + axes in the `recommend_playbook` response; the `/intake` route + nav
entry; `intake.svelte.ts`; the freeform → classify → recommend → confirm-persist flow with
auto-select honored.

**Out (deferred):** app-created/associated sessions for FTR attribution; a per-user intake **history**
view; editable inferred axes before confirm (v2 — the user overriding the classifier); the axis-picker
/ hybrid entry; changing the CLI's two-call persistence behavior.
