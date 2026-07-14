# 弟 · Dōjō · Developer flow

**Segment:** Dōjō (SaaS)
**Route:** in-app (embedded in the Observatory) — not a separate route
**Source mockup:** [`lib/dojo-inapp.jsx`](../../mockups/Sensei/lib/dojo-inapp.jsx)
**Source design:** [`Sensei Dōjō Journey Map.html`](../../mockups/Sensei/Sensei%20D%C5%8Djo%20Journey%20Map.html)

## Purpose

The developer's full Dōjō journey lives inside the Observatory
— no separate console. The user discovers a Dōjō, authenticates,
binds projects, produces findings, shares upstream, watches
what happened, and receives approved practice downstream.

This spec is the **journey narrative** mapping the developer-
role stages from the Journey Map to the existing screens.

Kanji is 弟 — *disciple*.

## Stages → screens

| Stage | Kanji | Purpose | Landing screen |
|---|---|---|---|
| Discover | 出 | Learn my org runs a Dōjō | Detected on first run ([[screen/first-run-scan]]) OR via Preferences → Profile → Connection |
| Authenticate | 鍵 | Prove who I am | [[screen/observatory-dojo-connections]] add-connection flow |
| Bind project → org | 識 | Route this project's findings correctly | [[screen/project-about]] Dōjō binding chip |
| A finding forms | 芽 | Real reusable lesson emerges | Auto in [[screen/observatory-memories]] as memory candidate |
| Share upstream | 共 | Contribute with right attribution | [[screen/observatory-share-review]] |
| Watch it travel | 旅 | See what happened to what I shared | [[screen/observatory-share-review]] (batch history) + notification banner on approval |
| Receive downstream | 贈 | Get approved org knowledge in Today / Upgrades | [[screen/observatory-upgrades]] + [[screen/observatory-today]] downstream lane |

## Data invariants

- No new tables — this spec is the narrative wiring across
  existing screens (see [[pipeline/dojo-lifecycle]]).
- Notification on approval: `sensei.dojo_notifications` row
  with `stage: approved` + link to the landing artifact.

## Done gate

- Every stage above has a landing screen that a first-time user
  can find without a wizard.
- The auto-discover step (first-run) works when a company Dōjō
  is broadcasting; renders a "join?" prompt.
- Sharing upstream and receiving downstream both work end-to-end
  on Jerry's local + a scratch `global-dojo` instance.

## Wrong gate

- **A stage has no reachable landing screen** — journey has a
  gap.
- **Approved artifacts don't surface a notification.** User
  doesn't know their contribution landed.
- **Auto-discover fires without a Dōjō broadcasting.** Should
  be silent when nothing's there.

## Related

- [[pipeline/dojo-lifecycle]] — mechanics
- (mockup: Sensei Dōjō Journey Map.html) — narrative source
- Every screen mentioned in the Stages table above
