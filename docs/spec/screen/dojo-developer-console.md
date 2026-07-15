# 群 · Dōjō · Developer console

**Segment:** Dōjō · console (SaaS or self-hosted)
**Route:** `console/developer` (the contributor seat)
**Source mockup:** [`lib/dojo/dojo-developer.jsx`](../../mockups/Sensei/lib/dojo/dojo-developer.jsx) → `DojoDeveloperConsole` (panels: My teams · My contributions · For me)
**Data:** `dojo.memberships` (per user), the contribute outbox / `federated_memories` ledger (per-destination status), the downstream inbox
**App file:** _dojo web app (`dojo/`), developer view — greenfield_
**Daemon files:** `crates/dojo-mind` (memberships, contribute, routing — exist; a per-user read view is greenfield)
**Status:** greenfield console view; the dojo-mind data model exists but is external-blocked (no live Dōjō server) — see [architecture/dojo](../../architecture/dojo.md)

## Purpose

Every user logs into the Dōjō, even a solo developer. The team/org value is
primary, but one developer belongs to **many** Dōjōs (employer · clients ·
communities · personal). This is their personal, **read-mostly** seat: where do I
belong, what have I sent up, and what came back down. Contribute / approve /
publish stay with maintainers and admins — this is the contributor view.

## Data invariants

- One login resolves **all** of a user's `dojo.memberships`, each with its `kind`
  (employer · client · community · personal) and derived role.
- A project routes **only** to the membership it's bound to — a finding never
  crosses into an unrelated Dōjō.
- On **client** memberships, contributions are **source-dereferenced
  automatically** — the lesson travels, the client + repo never do (theme 5,
  DJ2/DJ4).
- "My contributions" reflects the real per-destination status from the contribute
  ledger (pending / approved / declined / distributed); nothing is fabricated.
- "For me" shows only knowledge **approved and distributed down** to this user
  (pull, never push).

## Signals shown

| Element | Value | Meaning | Example |
|---|---|---|---|
| Membership card | kind + role + follows | one row per Dōjō the user belongs to | `acme · employer · Contributor · follows Web·Auth·Payments` |
| `active` chip | on the current membership | which Dōjō this project is bound to | `globex · active` |
| Client-dereference note | fixed banner | on client memberships the source is stripped | 客 "the lesson travels, the client and repo never do" |
| My contributions row | title + per-destination status | what I sent upstream and where it stands | `refresh-token store → acme · approved · initech · pending` |
| For me row | approved teaching | distributed down to me | `Prefer additive migrations · from acme/platform` |

## Done gate

- The user's real memberships list (employer/client/community/personal) with role
  + follows; the bound project shows `active`.
- My contributions shows real per-destination status from the ledger.
- For me shows only approved-and-distributed teachings.
- Client memberships visibly mark the source-dereference guarantee.

## Wrong gate

- A contribution shows a client name or repo path (dereference failed — DJ4).
- A finding appears under a membership it wasn't bound to (routing leak).
- Contribute/approve controls appear here (this is the read-mostly contributor
  seat, not maintainer).
- "For me" shows unapproved or pushed (not pulled) content.

## Related

- Objectives [DJ1–DJ5](../../requirements/objectives.md#dōjō--the-cross-cutting-team-layer) · [architecture/dojo](../../architecture/dojo.md) · [journeys/dojo](../../journeys/dojo.md)
- Sibling consoles: [[screen/dojo-maintainer-console]] · [[screen/dojo-admin-console]] · [[screen/dojo-lead-console]]
- In-app developer flows (Observatory): [[screen/dojo-developer-flow]] · [[screen/observatory-dojo-connections]]
