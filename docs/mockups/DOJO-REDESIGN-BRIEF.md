# Dōjō web app — redesign brief (work-first IA)

> Hand this to the designer (claude.ai/design). It reorganizes the Dōjō web app around a
> personal-first IA and asks for a **modular** artboard set (a shared component kit first, then
> screens composed from it) so the components map cleanly to Svelte later. The route/IA reference
> is [`../design/dojo-web.md`](../design/dojo-web.md); the user-facing intent is
> [`../features/dojo/README.md`](../features/dojo/README.md).

## Build modular

Design **one shared component kit first**, then compose **every** screen from it — no per-screen
hand-rolling. Keep the existing conventions: `lib/tokens.css` (Zen-Sumi named tokens, 8-stop type
scale, 4px spacing, kanji accents), reuse/extend `lib/dojo/dojo-shared.jsx`, lowercase "sensei",
sentence case, **no emoji**, mobile-first responsive (`<md` phone / `md:+` desktop).

## The IA

- Land on **your work** (the primary surface): projects you're working on · a "needs you" band ·
  live runs. A dōjō is **optional**; membership / management / config are secondary.
- A **"my dōjōs"** list shows each org + **your role** (empty state when none).
- Click an org → **org context at `/org/[slug]`** with its **own nav pane**: projects in its
  jurisdiction · the **constitution ladder** (governance) · role-scoped surfaces.
- Click a project → **preview which ladder levels built its constitution** (conflicts settled,
  locks shown).
- Two planes: **governance** (rules + ladder) and **relay** (watch · approve · decide · chat).

## Naming

- The rules-adopt screen is **"rule packs"** — not "library" (that word already means dependency
  docs in sensei).
- Roles are **additive**: **developer** (read-mostly) · **maintainer** (governance) · **lead**
  (clients) · **admin** (member roles + policies).

## Shared kit — design these FIRST, reuse everywhere

- **AppShell** = TopBar (brand · **OrgSwitcher** popover · search · avatar) + **NavPane** (grouped
  items + version footer) + main content. **Personal and org contexts use the SAME shell** — only
  the nav items + a context header differ.
- **Rows / primitives:** **MyDojoRow** (org + role) · **ProjectRow** (name · classification ·
  phase) · **EmptyState** · **SectionHead** · **Banner** · Chip · KanjiToken · StatBadge · Spark ·
  EnsoRing · ConfidenceBar.
- **Governance:** **LadderRung** (scope + its rules + locks) · **RuleRow** (include · level pills ·
  ★ non-negotiable) · **ConflictCard** (topic · winner · why) · **StanceDial** (autonomy / sharing
  / review).
- **Relay:** **RunCard** · **GateCard** (command + approve/deny) · **NeedsYouBand** ·
  **DecisionCard** · **ChatThread**.

## Screens — compose only from the kit (desktop + mobile each)

**Personal**
1. **your work** (home) — needs-you · your projects · live runs · the my-dōjōs list.
2. **your rules** — rule packs (adopt) + your effective personal constitution.
3. **project constitution preview** — the ladder resolved for one project.
4. **relay** — watch / approve / decide / chat.

**Org — `/org/[slug]`**
5. **org home** — the projects in the org's jurisdiction.
6. **constitution ladder** — author governance per scope along the ladder.
7. **project → effective-constitution preview** — which ladder levels built it.
8. **role surfaces** — members/roles/policies (admin) · triage/approvals/knowledge (maintainer) ·
   clients/engagements/audit (lead).

## Deliver

One **shared-components** artboard + one artboard per screen (desktop + mobile), each screen built
**only** from the kit. Flag any new primitive you had to add so it can be folded back into the kit.

## The ladder (for the preview + governance screens)

Rules resolve broad → specific: **company → client → personal → project → stack**. The more
specific scope refines the broader — except a **non-negotiable (★)** rule, which locks and can't be
relaxed beneath it. Confidentiality resolves first. On a client engagement both the client's and
the employer's constitutions apply: the **owning org wins ordinary conflicts**, while the
employer's non-negotiables stay a floor the owner can tighten but never relax.
