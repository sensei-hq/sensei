# 守 · Dōjō · Client / engagement lead console

**Segment:** Dōjō (SaaS) — console
**Route:** `dojo.sensei-hq.org/{origin}/{org}/console/client-lead`
**Source mockup:** [`lib/dojo-console.jsx`](../../mockups/Sensei/lib/dojo-console.jsx)

## Purpose

Client / engagement leads guard confidentiality on client work
inside a Dōjō. They register client engagements, ensure the
universal dereference strip holds, and hold the audit trail
for compliance.

Five stages:

| Stage | Kanji | Purpose |
|---|---|---|
| Define engagement | 客 | Register a client so its work routes correctly |
| Anonymize, always | 盾 | Share the lesson, never the source |
| No per-item review | 信 | Trust the universal strip |
| Audit trail | 録 | Prove confidentiality held |
| Incident handling | 警 | Contain a near-leak fast |

Kanji is 守 — *guard*.

## Data invariants

- Reads from `dojo.engagements`, `dojo.artifacts` (with
  `dereferenced: true` filter), `dojo.audit_events`,
  `dojo.incidents`.
- Universal strip is enforced at [[pipeline/dojo-lifecycle]]
  attribution step — client-lead cannot per-item override,
  matching the journey map's design.
- Incidents log rows with severity + resolution.

## Signals shown

| Element | Value |
|---|---|
| Engagements list | client · project bindings · start / end date · policy overrides |
| Artifact audit view | filterable by engagement — every artifact ever shared with strip status |
| Strip verification | on hover, shows what fields were stripped |
| Incident dashboard | open incidents · severity · owner · SLA |
| Compliance report | exportable audit trail for a period |

## Done gate

- Engagements can be created and bound to projects.
- Universal strip renders in the audit view for every client-
  work artifact (no exceptions).
- Incident creation with severity + owner works.
- Audit trail export produces a CSV / PDF suitable for
  compliance evidence.

## Wrong gate

- **A client-work artifact appears in the audit without strip
  info.** Enforcement broken.
- **Client-lead can override the strip.** Journey map violated.
- **Incident open past SLA without an alert.**
- **Audit export includes columns not covered by the strip.**
  Export leaks source references.

## Related

- [[pipeline/dojo-lifecycle]] — universal strip
- [[pipeline/governance]] — compliance packs (HIPAA / PCI / SOC2)
- [[screen/dojo-maintainer-console]] · [[screen/dojo-admin-console]]
