# Dōjō Consoles · T2

**Plane:** team · **Status:** [roadmap/04-team.md](../../../../roadmap/04-team.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
org admin ([persona 4](../../../03-personas.md#4--org-admin--the-plane-owner))

## Purpose

The Dōjō Consoles are the **four role-scoped surfaces** (contributor · maintainer ·
lead · admin, D1) that make the Dōjō usable for each person's job. They are **additive
grants**, not a single admin/not switch — a maintainer (governance) needn't be an
admin (roles/policies), and vice-versa. The personal landing ("your work") is
always primary; the consoles are what you open *inside* an org context. The job
is **each role sees exactly what it can act on** — no more, no less. The user
feels this as *I open my org and the surfaces match my role — I don't trip over
things I can't touch.*

The maintainer's console is not a viewer — it is where the constitution is
**authored**: the stance, the rules, the shared skills / agents / commands, and
a workspace's learnings, written once at a **scope** and composed down the ladder
for everyone below. That half of the console is specified in
[The governance authoring surface](#the-governance-authoring-surface).

> **Status honesty.** The data layer exists (memberships, roles, `dojo.member_role`
> enum, role ladder in `dojo-auth.ts`), the personal screens (`/you/*`, Relay,
> Library, Preview) are built and tested. The **org context `/org/[slug]` exists**
> (`dojo/src/routes/(app)/org/[slug]/+page.svelte` + `/projects` + `/[section]`)
> and the **four role-scoped surfaces within it are built and role-gated**
> (`dojo/src/lib/nav.ts` `navForOrg`; gap T-2 resolved, #307 A3 invites). The
> org-switcher popover is built (top bar: "Relay · you — all Dōjōs" + one row per
> membership + "Your Dōjōs"). Its "＋ Create or join" entry is retired by D11/D20 — a
> dōjō comes from a git integration — but `dojo/src/lib/components/kit/OrgSwitcher.svelte`
> still shows it: a known leftover to remove.

## In scope / out of scope

- In: the four role definitions and their additive grants:
  - **Contributor** (default) — read-mostly: see workspaces in jurisdiction, preview
    a workspace's effective constitution, view members. A device-code sign-in maps to
    contributor; there is no "read-only" role.
  - **Maintainer** — **manage governance at org level**: author rules/stance on
    the ladder, triage/approve findings, manage knowledge.
  - **Lead** — a rung between maintainer and admin, never derived from the forge.
    Since D34 (no Clients section) it unlocks nothing a maintainer lacks.
  - **Admin** — **change member roles + access policies**: members, identities/SSO,
    scopes & precedence, billing.
  - The **org context `/org/[slug]` exists** (`+page.svelte` + `/projects` +
    `/[section]`) with its own nav pane (workspaces, governance,
    role surfaces); the **four role-scoped surfaces within it are built and role-gated** (T-2 resolved).
  - **The governance authoring surface** (`/org/[slug]/governance`, maintainer):
    the **scope ladder** (free personal ladder vs the paid company ladder), the
    **stance** axes, the **five authored artifact kinds** (rules · skills ·
    agents · commands · workspace learnings), *defined-here vs inherited*
    rendering with explicit override labels, the **constitution library** entry
    point, the **onboarding preview**, and **playbook-learning proposal
    review** — specified in full below.
  - The personal screens that are already built: `/you/projects`, `/you/inbox`,
    `/you/relay`, `/library`, `/preview`, `/me`. The **inbox** carries Relay for
    contributors (Q10, amended 2026-10-21): tickets assigned to you from the tracker
    mirror sit under the relay session that carries their key; tickets with no session
    yet are listed below.
  - The **viewer shell** (T6) is a non-role surface: a viewer is not a member and gets
    none of the role consoles.
- Out: the federation backbone (T1), workspace planning against trackers (T3),
  billing/metering (T4).

## Surfaces

- **App (target routes)** — `/org/[slug]` (org home: workspaces in jurisdiction);
  `/org/[slug]/governance` (ladder config — maintainer);
  `/org/[slug]/projects/[id]` (workspace preview: which ladder levels built the
  constitution);
  `/org/[slug]/members`, `/identities`, `/policies`, `/billing` (admin);
  `/org/[slug]/triage`, `/approvals`, `/knowledge` (maintainer);
  `/org/[slug]/audit` (admin) — who did what in the dōjō. **No Clients section
  (D34):** a client is just another dōjō, so there are no engagements, incidents or
  client audit screens; the confidentiality ledger D19 kept apart from the admin
  audit is gone with them.
- **App (built personal)** — `/` (personal landing: your work + My Dōjōs);
  `/you/projects`, `/you/inbox`, `/you/relay`, `/library`, `/preview`, `/me`.
- **Mockup (authoring source of record)** — `mockups/Sensei/lib/dojo/dojo-governance.jsx`
  (`GV_SCOPES`, `STANCE`, `GV_DATA`, `GvSection` / `GvRow`, `PlaybookReview`) —
  visual source, never edited.
- **App (build contracts, partial)** — `spec/dojo/screens/org-ladder.md`
  (rules editor + pack adoption at the org scope) · `spec/dojo/screens/rule-packs.md`
  (`/you/packs` personal adoption) — element→data contracts for two of the
  library/rules affordances; the stance, inheritance and onboarding surfaces
  have no build contract yet.
- **Daemon** — `dojo-auth.ts` (`resolveTenantAccess`, role/access-floor logic);
  `governance.rs` (resolution, cached 5 min); `get_rules` / `get_layered_context`
  MCP tools.
- **Data** — `dojo.memberships` (user_id, org_id, role, kind), `dojo.member_role`
  enum (`contributor < maintainer < lead < admin`), `dojo.shared_rules`,
  `dojo.tenants`, `dojo.audit_log`, `dojo.audit_events`; plus the governance
  ladder the authoring surface writes against: `sensei.scopes` (levels
  `general` … `repository`, `level` = most-specific-wins), `sensei.namespaces`
  (instances — a repo is a member of a **set** of namespaces, there is no parent
  tree), `sensei.rule_packs` + `sensei.rule_pack_adoptions` (library adoption).

## Data invariants

- **Lead.** Grants are **additive** — the union of a user's roles is what they
  see. A maintainer + admin sees both governance and roles/policies.
- **Lead.** **Per-person lists ("By developer") show only where the reader is a
  maintainer or above on the repositories behind them** (D6). Teams are reporting groups:
  a team caption never binds a workspace (D7, D13).
- **Lead.** The personal landing is **always reachable** — the org-switcher
  popover has "Relay · you — all Dōjōs" pinned at top.
- **Lead.** Role floor: `contributor < maintainer < lead < admin` (`dojo.member_role`; canonical since 2026-10-21).
  A role reveals its surfaces; the union is what you see.
- **Lead.** Client binding **excludes employer** — the owning org wins ordinary
  conflicts; the employer's mandatory rules remain a floor.
- **Lead.** **You author against a scope, not a document.** Every authored
  artifact carries the scope it was written at. A dōjō's governance levels are
  **dōjō → workspace → repository** (D25); scopes compose from self up to the
  dōjō. A **stack** — a tech stack's rules and packs (React, Rust, …) — is not a
  level: it is a rule pack (`stack:<slug>`) **applied at a level**, and sensei
  adopts it **at each repository where the stack is detected** (D29, D33), so its
  rules apply there like any rule written at the repository. Teams are not a
  governance level — they are reporting only (D13). Client is not a governance
  level either (D32): client work is the dōjō it lives in plus the workspace
  classification `client`, and client-only standards are rules at that dōjō or
  workspace. An individual's own rules live in sensei, not in the dōjō.
- **Lead.** **Composition is most-specific-wins**, and it is not the first line
  of resolution: confidentiality resolves first, then a mandatory (★) lock that
  no narrower scope may relax, then specificity — T1's three-step order.
- **Lead.** **Company vs Client is derived from membership**, never chosen:
  your employer's Dōjō renders *Company*, an org you're engaged with renders
  *Client*. It is a caption on the dōjō, not a rung — either way the dōjō sits
  on the Dōjō rung (D32).
- **Lead.** **Anonymization is not authorable.** The stance data model carries
  an `anon` value, but the surface exposes no stop for it — client work is
  stripped by T1's universal strip or it does not leave.
- **Lead.** **A collision is surfaced, never hidden.** A local artifact that
  shadows an inherited one is shown *as* overriding; the inherited one stays
  visible and marked. Composition counts are **deduplicated by normalized
  text** across the chain, so a rule written at two levels counts once.

## The governance authoring surface

Source of record: [`mockups/Sensei/lib/dojo/dojo-governance.jsx`](../../../../mockups/Sensei/lib/dojo/dojo-governance.jsx)
(visual source, never edited). One screen, three jobs: pick a **scope**, set its
**stance**, fill it with **artifacts** — then read back what everyone below
inherits. Header: 掟 · *Govern · define* — "Governance & shared knowledge".

### The ladder you author against

| Rail group | Kanji | Example | Marker |
|---|---|---|---|
| Personal | 己 | My workspaces | free |
| Dōjō | 社 | Company · Globex | **paid** · *your employer* or *client · not your employer* |
| Workspace | 件 | personal-site · lumen-auth · billing-svc | *(none)* |
| Repository | 庫 | lumen-auth-api · billing-svc-web | *(none)* |

- **The dōjō ladder is dōjō → workspace → repository, most specific wins** (D25):
  a rule at the dōjō applies everywhere in it, a workspace rule to that group of
  repositories, a repository rule to one repository. In `sensei.scopes` the rungs
  are `general` 0 · `user` 10 · `organization` 20 (the dōjō — labelled *Dōjō*) ·
  `workspace` 60 · `repository` 70. There is **no Team rung** — teams are
  reporting only (D13) — **no Client rung** (D32) and **no Stack rung** (D29).
- **Personal is not a dōjō level.** The Personal group shows an individual's own
  rules in sensei (the `user` rung); they stay theirs and are
  never authored by the dōjō. The rail marks the **Dōjō** paid;
  **Workspace and Repository carry no marker**.
- **Company vs Client is derived per-user from membership** — never chosen. It
  captions the dōjō; an engaged org's dōjō is still a Dōjō rung, not a Client one.
- **Composition**: walk `parent` from the scope to the root — so a repository
  composes as *lumen-auth-api → lumen-auth → Company*. A stack is a rule pack
  (`stack:<slug>`, area `tech_stack`) adopted at **each repository where the stack is
  detected** (say React at `lumen-auth-api`), so it contributes its rules at the
  repository rung (D29, D33); it is never a rung of its own. Any pack adopted at the
  dōjō or a workspace contributes from that level down.
- The rail badge counts **rules + skills only** at each scope — an empty scope
  reads blank rather than counting agents and commands.
- Desktop: rail grouped by kind. Mobile: the same scopes as a wrapping pill row.

> The `parent` links are the mockup's **preview chain**. Rule resolution has no
> tree: a repo binds to a *set* of `sensei.namespaces` and `sensei.scopes.level`
> orders them most-specific-first — a `parent_id` was tried and dropped because
> a repo needs both a personal and an org source at once
> ([`design/governance.md`](../../../../design/governance.md) §Scopes). The
> preview chain must therefore be **derivable from real namespace membership**;
> see Open gaps.

### Stance — three dials and a fixed floor

| Dial | Kanji | Question | Stops (0 → 2) | Consequence, low → high |
|---|---|---|---|---|
| Agent autonomy | 自 | how far sensei goes before it gates to a human | Gate often · Balanced · Run free | every prod-touching or irreversible step pauses → only explicitly-flagged commands gate |
| Sharing | 共 | what leaves this scope, upward | Manual · Opt-in · Automatic | nothing leaves unless shared by hand → lessons past the confidence bar leave on a cadence |
| Review | 検 | approvals required before a lesson publishes | 1 approval · 2 approvals · Consensus | one named maintainer → all scope owners sign off |
| Anonymization | 盾 | *not a dial* | — | fixed: "Client anonymization · always on" |

- Each dial shows **the consequence of the stop selected**, not generic help text.
- **An unauthored scope defaults to the middle stop on every dial** — *Balanced ·
  Opt-in · 2 approvals*. A locally changed stop wins over the stored one.
- **Stance resolves most-specific-first**: a repository's stance beats its
  workspace's, which beats its dōjō's, which beats the default (`most_specific_scoped_stance_beats_default_and_lower_rungs`).
- **A scope can also be *strict*.** Under a strict stance **every dial fails
  closed** — every autonomous action is refused unless explicitly permitted
  (`strict_fails_closed_on_every_dial`). The authoring mock has no strict control,
  so whether it is derived from the stops or needs one is an Open gap.
- **Anonymization has no stops.** The data model carries `anon`, and the mock
  prints the 盾 line unconditionally in the stance header (even at personal
  scope) — a mockup simplification; the requirement is the *floor*, not the label.
- **Stance ≠ sprint gates.** Autonomy is *approval for a risky agent step*; the
  code-health gates in [T3](../03-dojo-workspace-planning/README.md) are
  code-derived and delta-scoped. Two mechanisms, one word.
- Worked examples — *Company*: **Gate often** · **Automatic** · **Consensus**.
  *lumen-auth*: **Gate often** · **Opt-in** · **Consensus**.

### The five artifact kinds

| Kanji | Section | Add | Counts | Notes |
|---|---|---|---|---|
| 掟 | Rules · guards · principles | Add rule | here · inherited | glyph + text + type chip |
| 技 | Shared skills | Add skill | here · inherited | local rows carry an *edit* affordance |
| 問 | Agents & personas | Add agent | here · inherited | as above |
| 令 | Commands | Add command | here · inherited | as above |
| 憶 | Memory & learnings · this workspace | Add learning | here only | **workspace scopes only**; each row carries a tag (`3rd time`, `settled`, `gotcha`) |

- **Four rule types here**: `guard` 守 · `principle` 理 · `pattern` 紋 · `anti` 禁
  (chipped *anti-pattern*). That is **not** the six-family vocabulary the rule
  editor fixes `rule_type` to (`RULE_FAMILIES` = 守 guard · 紋 pattern · 理
  principle · 検 review · 技 stack · 盾 shield) — see Open gaps.
- Glyphs collide across surfaces: 技 is *skills* here but *stack* in the editor;
  盾 is *anonymization* here but *shield* there; 検 is the *review* dial here but
  the *review* family there. **Kanji are brand marks, never the data.**
- A section header reads `N here · M inherited`; empty copy renders only when
  **both** are zero, so a scope that authored nothing but inherits five still
  lists them.
- The memory section is gated on `kind === "Project"` — a dōjō or repository
  scope renders no memory section at all.

### Defined here, inherited, overridden

- Inherited rows render dimmed with a faint glyph, attributed `↑ {source scope}`.
- A **local** item whose normalized text matches an inherited one is labelled
  `overrides ↑ {source}` in accent; the **inherited** copy is struck through and
  labelled `overridden here` in danger. **Both stay listed** — the collision is
  shown, not resolved by hiding.
- Attribution is **first-wins along the chain**: nearer scopes are walked before
  ancestors; an adopted pack's rules are attributed to the level that adopted it.

### What a new developer inherits

- A summary card counts the **composed** set — rules · skills · agents ·
  commands · (learnings, workspace only) — deduplicated across the whole chain, so
  the number is *unique artifacts*, not a sum of lists.
- It names the chain: *"Composed down the ladder — this scope plus **lumen-auth ·
  Company**"*, with the packs applied at each scope (the Rust and React stack packs
  sit on the repository, where those stacks are detected — D33), and states delivery: *"Delivered to their
  Observatory the moment they connect."*
- **Preview onboarding** expands to a per-scope breakdown (one row per
  contributing scope) and closes with *"Composed by specificity — a more
  specific scope's rule wins on collision."*

### The constitution library

- Header **Add from library** (蔵) and an inline banner: *"Don't start from a
  blank page — pull proven principles, patterns, compliance controls and stack
  reviewers from the constitution library. **Prevention is cheaper than
  rework.**"* → **Browse library →**.
- That is the adoption write for `sensei.rule_packs` at *this scope's* namespace
  — the same table the personal `/you/packs` screen writes at the user's
  namespace (`spec/dojo/screens/rule-packs.md` · `org-ladder.md`).
- It is also the **missing surface for the ponytail pack**: seeded but never
  adopted, so its opt-in is unreachable today (M13's C-2 follow-on).

### Playbook learning — accept makes it a rule

- Section 覚 · *Playbook learning · proposed rules* counts **pending** proposals;
  intro 察: *"Sensei attributes first-turn resolution back to each combo and
  proposes a re-weighting. Accept to make it a rule at this scope; it then
  cascades to everyone below."*
- A proposal is `lifecycle · intent · risk` → **prefer** *X* over *Y*, with its
  evidence: the reason, the projected `FTR 0.71 → 0.90`, and `· 14 runs`. Risk
  `high` renders in warning tone.
- **Accept** → marked `✓ rule at {scope}` and leaves the pending count;
  **Dismiss** → dimmed, with **dismissed · undo** returning it to pending.
- This is the **governance half** of the front-door learning loop; the
  developer's own per-combo FTR stays in the app's *Intake · History*.
  `list_playbook_rule_proposals` / `accept_playbook_rule` are already named MCP
  surfaces in M13 — the missing piece is this maintainer-facing review.
- Worked example — *stable · enhancement · low*: prefer **Change-flow** over
  **Get stuff done**, `FTR 0.71 → 0.90 · 14 runs` — "Impact-first runs landed
  cleaner than lean-plan on stable enhancements."

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Org context nav | workspaces / governance / members / triage | maintainer sees governance + triage |
| Role badge | the user's role in this org | `maintainer` |
| Scope rail | the ladder, grouped by kind, paid-marked, with a rules+skills count | `社 Company · paid · 4` · `社 Globex · paid · client` |
| Scope chip | which scope you are authoring | `件 lumen-auth · Project` |
| Stance dial | the selected stop **and** its consequence | `Sharing → Automatic: lessons past the confidence bar are sent up on a cadence` |
| Artifact section | what is authored here vs inherited | `2 here · 5 inherited` |
| Override label | a local artifact shadowing an inherited one | `overrides ↑ Company` |
| Overridden label | an inherited artifact shadowed locally | struck-through text · `overridden here` |
| Inheritance summary | composed, deduplicated counts | `6 rules · 3 skills · 2 agents · 4 commands · 3 learnings` |
| Playbook proposal | a proposed re-weighting + its evidence | `stable · enhancement · low → prefer Change-flow over Get stuff done — FTR 0.71 → 0.90 · 14 runs` |
| Triage queue | findings awaiting review | 3 pending (2 auto-approved, 1 human) |
| Audit trail | immutable log | `promote rule#r-12 by u-3 at 2026-10-13T14:22:00Z` |
| My Dōjōs section | orgs + role on personal landing | `Sensei-HQ · maintainer` |

## Done gate

- A maintainer opens `/org/[slug]/governance`, picks a scope on the ladder, sets
  the three stance dials (each showing the consequence of the stop selected),
  and authors a rule, a shared skill and a command at that scope; each appears
  under `N here`, and after cache expiry the scope's rules appear in `get_rules`
  for the scopes below it.
- The same scope renders inherited artifacts dimmed and attributed
  `↑ {source}`; a local rule that shadows an inherited one reads
  `overrides ↑ {source}` while the inherited copy is struck through and reads
  `overridden here` — neither is hidden.
- The inheritance card shows composed, **deduplicated** counts and names the
  chain; **Preview onboarding** breaks that down per contributing scope and ends
  with "composed by specificity".
- A maintainer browses the constitution library and pulls a pack into the current
  scope — adopted at that scope's namespace, not the user's.
- A maintainer accepts a playbook proposal; it becomes a rule at that scope and
  leaves the pending count. Dismissing one stays recoverable via undo.
- Anonymization is **not** editable at any scope — the client floor holds.
- An admin opens `/org/[slug]/members` and changes a user's role; the access
  floor updates immediately in `resolveTenantAccess`.
- An admin opens `/org/[slug]/audit` and sees the immutable audit trail; no
  role sees an Engagements, Incidents or Client audit section (D34).
- A contributor opens `/org/[slug]` and sees workspaces in jurisdiction; clicking
  one drills to `/org/[slug]/projects/[id]` previewing which ladder levels built
  the constitution.
- The personal landing (`/`) shows "your work" (workspaces, needs-you, relay) as
  primary and "My Dōjōs" with role badges as secondary.

```bash
# Worker API (requires auth)
curl -s -H "Authorization: Bearer $TOKEN" \
  "https://dojo.sensei-hq.com/v1/t/org/sensei-hq/members" | jq '.[] | {user_id, role}'
```

## Wrong gate

- **A maintainer sees the admin surfaces (members/roles/policies)** — role-gating
  regressed.
- **An admin cannot change a member's role** — `resolveTenantAccess` regressed.
- **A contributor can author governance** — role-gating regressed.
- **A shadowed artifact disappears instead of reading `overridden here`** —
  collision display regressed (hiding beats surfacing).
- **The inheritance count is a plain sum that double-counts an artifact written
  at two levels** — composition dedup regressed.
- **A maintainer can edit client anonymization** — confidentiality is not
  authorable.
- **An accepted playbook proposal never becomes a rule at that scope** — the
  loop's accept path regressed.
- **A pack applied at a level does not reach the scopes below it, or a detected
  stack's pack is missing from its repository** — applying a pack regressed (D29,
  D33).
- **An unauthored scope shows a stance other than the middle stop** — default
  drift.
- **The personal landing shows the management overview instead of "your work"**
  — work-first regressed.
- **Clicking an org in My Dōjōs goes to `/console/teams` instead of
  `/org/[slug]`** — org context regressed.
- **A lead's audit trail is mutable or missing events** — audit regressed.

## Scenarios

- [`scenarios/dojo-consoles.feature`](scenarios/dojo-consoles.feature) — the
  four role consoles and their surfaces (pinned for built personal screens,
  `NO TEST` / `# gap: T-2` for org context).
- [`scenarios/dojo-consoles-gaps.feature`](scenarios/dojo-consoles-gaps.feature) —
  the org context role surfaces (built; `NO TEST` until pinned).
- [`scenarios/dojo-governance-authoring.feature`](scenarios/dojo-governance-authoring.feature) —
  the authoring surface: ladder, stance, artifact kinds, inherit/override
  rendering, onboarding preview, library, playbook proposals (`NO TEST` /
  `# gap: dojo-authoring-surface`, epic [#91](https://github.com/sensei-hq/sensei/issues/91)).

## Open gaps

- [T-2](../../../../roadmap/06-gaps.md) — join/membership flow. **Resolved**:
  invites (#307 A3) and GitHub-org provisioning create memberships; the role
  surfaces are built and role-gated.
- [T-1](../../../../roadmap/06-gaps.md) — T3's tracker adapters: the daemon-side
  Jira adapter exists (PR #274); Azure Boards and GitHub Issues do not.
- **"＋ Create or join"** in `OrgSwitcher.svelte` — retired by D11/D20, still
  rendered; remove it.
- Epic [#91](https://github.com/sensei-hq/sensei/issues/91) — the authoring
  surface is **specified here but unbuilt**; the org-context route it lives on is
  built, and the rules editor's save is still presentational
  (`D-4`, reorg §12.4 — the dojo2 ladder / stance / rule-pack sections are
  fixture-only).
- **Route to reconcile** — this spec and
  [`design/dojo-web.md`](../../../../design/dojo-web.md) §3 put authoring at
  `/org/[slug]/governance`; the build contract
  [`spec/dojo/screens/org-ladder.md`](../../../../spec/dojo/screens/org-ladder.md)
  builds section `ladder` at `/org/[slug]/ladder`.
- **Who authors** — the mockup and `design/governance.md` §Budgets & controls say
  *admin*; this spec follows `design/dojo-web.md`'s recorded split (*maintainers
  own governance · admins own roles + policies*).
- **Rule-type vocabulary** — the authoring mock's four types (`guard · principle
  · pattern · anti`) ≠ `RULE_FAMILIES`' six (守紋理検技盾), and
  `dojo.shared_rules.rule_type` is an unconstrained `text`. One vocabulary must
  win before either editor is built.
- **Preview chain vs the data model** — the mock composes through `parent` links;
  the schema has no parent (a repo is a *set* of namespaces). The preview must be
  derived from namespace membership.
- **Library reachability** — the ponytail pack is seeded but nothing adopts it
  (M13's C-2 follow-on); "Browse library" is the surface that would close it.
- **The ladder is dōjō → workspace → repository — resolved (#307 R6, #318).**
  `sensei.scopes` seeds only `general · user · organization · workspace ·
  repository`; `team`, `client` and `technology` are retired (D25, D32, D29), and
  the deploy refuses — never deletes — if a namespace still sits on a retired rung.
  Each technology namespace became its stack pack, adopted at each repository where
  the stack is detected (D33). `gov_scope` accepts
  `general|user|organization|workspace|repository` (`crates/mcp`), and the dōjō's
  `constitution-map.ts` / `org-ladder-view.ts` group rules *Dōjō · Workspace ·
  Repository*; the library's levels (`library-data.ts`) are `dojo · workspace ·
  repository`.
- **Strict stance has no control** — M13 pins `strict_fails_closed_on_every_dial`,
  but the authoring mock exposes only the three 3-stop dials; *strict* must be
  either derived from the stops or given a control of its own.

## Related

- [design/dojo-web.md](../../../../design/dojo-web.md) — sitemap, routes & IA
  (roles table, org context nav, target routes) and §4 — the ladder, the
  enforcement axis, and the three governance surfaces (Library · Authoring ·
  Preview).
- [design/governance.md](../../../../design/governance.md) — scopes, namespaces
  & precedence; the identity/membership routing; the stance concept (§Budgets &
  controls).
- [design/governance-track-depth-review.md](../../../../design/governance-track-depth-review.md)
  — the depth review that named this surface the track's one real gap (closed
  2026-10-14).
- [design/instruction-delivery-model.md](../../../../design/instruction-delivery-model.md)
  — how authored skills / agents / commands actually reach and *stick* in a
  session.
- [spec/dojo/screens/org-ladder.md](../../../../spec/dojo/screens/org-ladder.md) ·
  [`rule-packs.md`](../../../../spec/dojo/screens/rule-packs.md) — the build-level
  element→data contracts for the rules editor and pack adoption.
- [journeys/dojo.md](../../../../journeys/dojo.md) — journey map for each role.
- [mockups/Sensei/lib/dojo/](../../../../mockups/Sensei/lib/dojo/) — visual
  source (never edited): `dojo-saas.jsx`, `dojo-governance.jsx`,
  `dojo-preview.jsx`, `dojo-billing.jsx`.
- [prd/02-objectives.md §Dōjō](../../../02-objectives.md) · [prd/03-personas.md §2,3,4,5](../../../03-personas.md).
- Crates: `crates/senseid/src/dojo/`; Worker: `dojo/src/routes/(app)/org/*`,
  `dojo/src/routes/(app)/you/*`, `dojo/src/lib/server/dojo-auth.ts`.