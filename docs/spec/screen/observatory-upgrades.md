# 贈 · Observatory · Upgrades

**Segment:** 03 · Observatory — daily use
**Route:** `/upgrades`
**Source mockup:** [`lib/dojo/dojo-inapp.jsx`](../../mockups/Sensei/lib/dojo/dojo-inapp.jsx) → `InappDownstream`
**App file:** `app/src/routes/(observatory)/upgrades/+page.svelte`

## Purpose

Upgrades is the **downstream lane** — approved practice arriving
from Dōjō or from the global Collective. Every artifact that lands
here has already been triaged and approved elsewhere; the user's
job is to review it, apply (accept into local rules / skills), or
mute. This is the destination the journey map §3.8 §.15 wires
into the loop.

Every incoming artifact carries **attribution** and a **scope tag**
(company / team / project / stack), so the user can immediately
tell where it came from and where it applies.

Kanji is 贈 — *gift*.

## Data invariants

- `GET /api/upgrades` returns:
  ```json
  {
    "artifacts": [
      { "id": "…", "type": "principle|pattern|prompt|guard|skill|agent",
        "title": "…", "body": "…", "scope": {…},
        "attribution": { "mode": "named|anonymous", "author": "…", "org": "…", "anonymous_id": "…" },
        "received_at": iso, "state": "pending|applied|muted|pinned" }, …
    ],
    "unread_count": N
  }
  ```
- Reads from `sensei.dojo_downstream_inbox` and the collective
  inbox (see [[pipeline/dojo-lifecycle]]).
- Every artifact title / body comes through
  [[pipeline/narration-cache]] with `kind = upgrade_{type}` when the
  model is available.

## Signals shown

| Element | Value |
|---|---|
| Header | `贈 · N new upgrades` |
| Source filter | `Dōjō · {org}` / `Dōjō · {client}` / `Collective` / all |
| Type filter | principle / pattern / prompt / guard / skill / agent |
| Artifact card | type kanji · title · body · attribution · scope chip · received-at |
| Actions | Apply · Mute · Pin (one-decision-default: Apply) |
| Pin behaviour | pinned artifacts outrank local alternatives on conflict |
| Mute behaviour | muted artifacts don't populate the local rule / skill surface |

## Done gate

- Every artifact in the downstream queue shows here with
  attribution + scope + received-at.
- Applying an artifact installs it locally per its type:
  - `type = principle | pattern | rule` → new row in
    `sensei.rules` with `source: dojo:{org}:{artifact_id}`
  - `type = skill | agent | prompt` → installed under the
    assistant plugin surface (verify with
    `list_library_skills` for skills)
  - `type = guard` → added to the CI/lint check config
- Mute / Pin overrides persist across daemon restart and are
  respected by consumers (rules resolver, skill loader).
- Client-work artifacts are credited `anonymous` and carry no
  repo identifiers (every shared artifact is source-dereferenced,
  always-on).
- New arrivals bump `unread_count` and land at the top of the
  list; clicking Apply decrements `unread_count` by 1 within
  500ms.

Optional check:
```
curl -s http://localhost:7744/api/upgrades \
  | jq '{unread: .unread_count, n_artifacts: (.artifacts | length)}'
```

## Wrong gate

- **A muted artifact still shows up in `get_rules` results.**
  Local mute override not consulted.
- **A `client` artifact still has a source repo url.**
  Confidentiality regression.
- **Applied artifacts don't install as their type dictates.**
  Type router broken.
- **`unread_count` never decrements.** Read-mark not persisting.
- **A pinned artifact loses to a local rule on conflict.** Pin
  override not applied.

## Related

- [[pipeline/dojo-lifecycle]] — the loop this consumes
- [[pipeline/governance]] — where rules end up locally
- [[pipeline/mcp-surface]] — where skills / agents end up locally
- [[screen/observatory-today]] — surfaces new upgrades as a lane
- [[screen/observatory-share-review]] — the peer upstream surface
