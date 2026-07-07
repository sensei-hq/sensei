# 結 · Pipeline · Dōjō lifecycle

**Owner files (proposed):**
- Membership: `crates/senseid/src/dojo/memberships.rs`
- Routing: `crates/senseid/src/dojo/routing.rs`
- Triage queue: `crates/senseid/src/dojo/triage.rs`
- Attribution / dereference: `crates/senseid/src/dojo/attribution.rs`
- Federation transport: `crates/hive-mind/` + `crates/hive-protocol/`

## Purpose

The Dōjō is the **company-hosted hive-mind** that sits between the
individual developer and the global Sensei Collective. Personal
Sensei works fine without a Dōjō. When a Dōjō exists, it becomes:

- The **upstream destination** for memories, patterns, rules,
  prompts, skills, and agents a developer wants to share with
  their org (or client).
- The **downstream source** of approved practice the org expects
  everyone to inherit.
- The **governance layer** — nothing is org-wide without triage
  and a named approval.

This pipeline covers the loop from the personal side. Console-side
UX (queue, approve, admin) is specified in the Dōjō screen docs.

Kanji is 結 — *connection / knot*.

## Data invariants

### Memberships

A developer belongs to zero or many Dōjōs. `sensei.dojo_memberships`:

- `id` uuid, `user_id`, `dojo_id`, `dojo_url`, `role`
  (`contributor | maintainer | admin`), `kind` (`employer |
  client | community | personal`), `authenticated_via`
  (`sso | github_oauth | device_code`), `attribution_default`
  (`named | anonymous | dereferenced`).

Every **project** binds to **exactly one** membership through
`sensei.projects.dojo_membership_id`. Auto-bound at project detect
based on the folder's git remote + heuristics (org owner match,
existing memberships), or user-picked from the Project → About
pane.

Rule: **client takes precedence.** A project bound to a client
membership routes findings to the client's Dōjō before the
employer's, and the client's confidentiality policy governs.

### Artifact types

Six primitives ride the loop:

1. **Guiding principle** (`理`) — durable engineering value.
2. **Pattern** (`紋`) — constructive shape promoted to rule; also
   anti-patterns.
3. **Prompt** (`問`) — vetted prompt template or persona.
4. **Guard** (`守`) — lint / check / safety rail.
5. **Skill** (`技`) — packaged capability an assistant can pick up.
6. **Agent** (`使`) — configured agent with tools and scope.

Each has a canonical DDL shape in `sensei.dojo_artifacts` (jsonb
payload keyed by type). Upstream shape and downstream shape are
identical — the round-trip preserves the payload.

### Attribution

Depends on origin:

| Origin | Attribution | What's stripped | Where it can go |
|---|---|---|---|
| Personal · open source | Public credit | Nothing | You · communities · any org |
| Personal · closed source | Named to you, within chosen org | Source & specifics — only the generalized lesson travels | Private by default · opt-in to an org |
| Employer work | Named to you, org-internal | Stripped only if it leaves the org | Employer Dōjō |
| Client work | **Source-dereferenced** | Source reference (repo, identifiers, pointer). Learning + cause + context travel. | Shareable anywhere — source dropped, not lesson |

Dereference for client work is automatic. No per-item review
required; the universal strip is trusted.

## Lifecycle

Upstream, in five stages:

1. **Contribute** — developer marks a memory / pattern / guard to
   share, scoped to where it's true. On the personal side this is
   the Memories "widen scope" action (see [[pipeline/memory]]).
2. **Accumulate** — contributions pool on the Dōjō server,
   clustered and deduped against existing knowledge. Shape:
   `dojo_triage_queue` with `signature` for dedup.
3. **Triage** — maintainer sees candidates scored, with conflicts
   surfaced and near-duplicates merged. This is
   [[screen/dojo-maintainer-console]] queue view.
4. **Approve** — a named approval publishes at the chosen scope,
   with attribution and a regression note.
5. **Distribute** — approved practice lands automatically in every
   matching scope's Today / Upgrades lane downstream. The
   in-app consumer is [[screen/observatory-upgrades]] +
   [[screen/observatory-today]] (Today gains a downstream lane).

### Downstream — the return path

- Approved artifacts stream to every consumer whose scope matches
  the artifact's scope tag.
- The consumer's Sensei applies the artifact according to type:
  - Guiding principle / pattern → new row in
    `sensei.rules` with `source = dojo:{org_id}:{artifact_id}`.
  - Skill / agent / prompt → installed under the assistant plugin
    surface.
  - Guard → added to the CI/lint check surface.
- The user retains **mute** and **pin** overrides — they can
  suppress a downstream artifact without leaving the Dōjō, or pin
  it above ambiguous local alternatives.

### Federation transport

Sensei ↔ Dōjō traffic runs over the `hive-mind` federation
protocol. Persistent connection to `dojo_url`; retry with backoff
on drop. Payloads signed by the developer's device key
(configured at membership creation).

## Signals produced

| Signal | Consumer |
|---|---|
| Pending-upstream contributions per developer | [[screen/observatory-share-review]] |
| Triage queue rows per maintainer scope | [[screen/dojo-maintainer-console]] |
| Downstream inbox per developer | [[screen/observatory-upgrades]] |
| Attribution decisions | Audit trail in `dojo_events` |
| Distribution notifications | Today downstream lane |

## Done gate

- A developer with a `client` membership can share a memory
  scoped to their project. The Dōjō receives it with the source
  dereferenced — the memory, its cause, and its context arrive
  but the `project_id` / `session_ids` / raw identifiers are
  stripped.
- A maintainer's queue at `screen/dojo-maintainer-console` shows
  every contribution routed to their owned scopes.
- Approval publishes the artifact and every matching consumer
  receives it downstream within one federation heartbeat.
- Mute / pin overrides on a downstream artifact respect user
  intent — muted artifacts don't populate the local rule /
  skill surface; pinned ones win against ambiguous local
  alternatives.
- Client-project artifacts are automatically dereferenced with no
  per-item review step. Post-approval audit trail carries the
  dereference confirmation.
- Federation transport is resilient — a Dōjō outage delays
  contributions but does not lose them; queued-locally
  contributions replay when the connection restores.

Optional check:
```
# What memberships does this user carry?
curl -s http://localhost:7744/api/dojo/memberships | jq '.[] | {kind, dojo_url, role}'

# What's in my upstream queue?
curl -s http://localhost:7744/api/dojo/queue?direction=upstream | jq 'length'

# Any downstream artifacts arrived in the last hour?
curl -s http://localhost:7744/api/dojo/queue?direction=downstream \
  | jq '[.[] | select(.received_at > (now - 3600))] | length'
```

## Wrong gate

- **A client-project memory got published with its source repo
  identifiers intact.** Automatic dereference didn't run — this
  is the confidentiality gate; nothing else matters more.
- **Two memberships try to receive the same client project's
  artifacts.** Precedence rule violated — client wins uniquely.
- **A maintainer sees contributions from a scope they don't own.**
  Queue filter bug.
- **A distributed artifact lands in a consumer whose scope doesn't
  match.** Distribution query is broadcasting too widely.
- **Muted downstream artifacts still populate `sensei.rules` /
  skills locally.** Local mute override not consulted.
- **Federation drop loses in-flight upstream contributions.**
  Queue isn't durable across the drop.
- **A rejected contribution can be re-submitted verbatim.**
  Signature dedup should suppress until materially different.

## Related

- [[pipeline/memory]] — the promotion ladder feeds contributions
- [[pipeline/insights]] — recommendations for widening scope
- [[pipeline/governance]] — where downstream rules land
- [[pipeline/mcp-surface]] — where downstream skills / agents land
- [[screen/observatory-share-review]] — personal upstream review
- [[screen/observatory-upgrades]] — personal downstream lane
- [[screen/dojo-developer-flow]] — full developer journey inside a Dōjō
- [[screen/dojo-maintainer-console]] — triage / approve surface
- [[screen/dojo-admin-console]] — server & membership admin
- [[screen/dojo-client-lead-console]] — engagement definition, audit
- [[project_governance_plane_design]] (memory) — earlier design notes
