---
name: Implementation Backlog
description: Prioritized index of OPEN work — tracked as GitHub issues (sensei-hq/sensei)
date: 2026-04-28
---

# Implementation Backlog

Work is tracked as **GitHub issues** in [`sensei-hq/sensei`](https://github.com/sensei-hq/sensei/issues). This file is the prioritized index of **open** work. When an item ships, **close its issue and remove it here** — shipped history lives in git (`git log -- docs/backlog.md`) and in [`plan/decisions.md`](plan/decisions.md).

> The **capability roadmap** — implementation vs vision, ranked gaps G1–G10, phased —
> is [`plan/README.md`](plan/README.md). This backlog is the *issue tracker*; the plan
> is the *why/what-next*. New capability work should land as a G-gap in the plan and,
> when scoped, a filed issue here.

---

## Open GitHub issues (12)

### Epics
| Issue | Summary |
|-------|---------|
| [#91](https://github.com/sensei-hq/sensei/issues/91) | **Dōjō governance track** — admin site + policies + preferences + skills/agents |
| [#85](https://github.com/sensei-hq/sensei/issues/85) | **Track 3 — Project window** (per-screen, separate Tauri window) |

### Observatory · Instruments
| Issue | Summary |
|-------|---------|
| [#96](https://github.com/sensei-hq/sensei/issues/96) | Instruments: background-task visibility (scheduler state + logs UI) — frontend/backend shipped; e2e verify remaining |
| [#90](https://github.com/sensei-hq/sensei/issues/90) | Instruments Replay: usage verdict classifier (used / partial / ignored) — DDL + classifier + endpoints shipped; blocked on the Replay screen to close |
| [#43](https://github.com/sensei-hq/sensei/issues/43) | Observatory Configure section (design + build) |
| [#44](https://github.com/sensei-hq/sensei/issues/44) | J7 Extend & Customize screens (design + build) |
| [#45](https://github.com/sensei-hq/sensei/issues/45) | J5 Pattern knowledge catalog |
| [#46](https://github.com/sensei-hq/sensei/issues/46) | J9 Context-pack tool |

### UI / quality / site
| Issue | Summary |
|-------|---------|
| [#47](https://github.com/sensei-hq/sensei/issues/47) | Mockup component migration to Rokkit (steps 4–14) |
| [#81](https://github.com/sensei-hq/sensei/issues/81) | Update website `/sensei` route to the new mockup (focused sections, covers Dōjō) |

### Foundation / deferred
| Issue | Summary |
|-------|---------|
| [#39](https://github.com/sensei-hq/sensei/issues/39) | Bootstrap diagnostic logging + debug mode (`diagnostic_sessions`+`diagnostic_traces` model + log viewer + anonymized issue export) |
| [#50](https://github.com/sensei-hq/sensei/issues/50) | Extract `bootstrap` into a reusable library (deferred) |

---

## Open — not yet filed as issues

- **Codebase-wide silent-error audit** — find + fix every discarded error (`.ok()`, `let _ =`, empty catch, masking `unwrap_or_default`); log so it's inspectable. (Directed after the `node_kind` drop; that was one instance.) → plan WS D.
- **Stale / orphaned project cleanup** — scan reconcile already *tags* dead-but-ambiguous folders `stale` + empty projects `orphaned` (never auto-deletes). Needs list endpoints + a gated purge action + a housekeeping UI.
- **Activity-data GC** — periodic prune of `assistant_events`/`turns`/`sessions`/`transcript_turns` past a TTL, *only after* analysis has derived insights (structured-log TTL already shipped). GC counterpart to the analyzer + transcript backfill.
- **Analyzer/#65 follow-ups** — consolidation TOCTOU race (partial unique index on `(project_id, trigger_detail->>'signature')` + ON CONFLICT); subagent sidechains; embeddings on transcript turns.
- **Per-language calls-edges adapters** — #57 shipped Rust + a language-agnostic call-site contract; Python/Svelte/TS adapters still need to adopt it for `get_callers`/`get_callees`/`call_flow`.
- **Wizard → Preferences arch change** — split the wizard into a thin 5-stage first-run + a persistent editable Preferences surface (operationalises "value before setup"). No new backend. Verify current stage count first.
- **DORA delivery-performance module** — surface the DORA Four Keys (change lead time · deploy frequency · change fail rate · recovery time) + the generative-culture signal *alongside* FTR, derived from the code+activity graph + git history; correlate AI-pairing patterns with DORA movement (the "AI amplifies fundamentals" story). **Prereq: a deploy/release-signal detector** — sensei captures coding, not CI/CD today. Source: [`spec/governance/default-constitution.md`](spec/governance/default-constitution.md) + the Kasperowski *DORA-2025* thesis. → plan Phase 3.
- **Default governance bundle** — ship a curated starter constitution / guardrails / guidelines (DORA + XP/CD + Core Protocols, framed for AI-assisted work) as the **default** personal-Dōjō rules, so a fresh project inherits real rules instead of an empty file. Drafted: [`spec/governance/default-constitution.md`](spec/governance/default-constitution.md); seed `~/.sensei/rules.md` + the marketplace defaults.
- **Dōjō console backend — port `/v1` routes into the Worker** *(decision 2026-07-16: Path A)* — port `dojo-mind`'s `/v1/t/{tenant}/…` logic route-by-route into the CF Worker (cloud Supabase) rather than deploying `sensei-dojo` + proxying. 13 of ~14 routes still stubbed (triage → members/identities/policies → incidents/engagements + `[id]`/bind → audit/compliance → health); the TS clients + UIs already exist (`engagements/+server.ts` is the reference). Also build the **join/membership flow** (dead "Join" button today). Risk: a TS reimplementation forks dojo-mind's triage scoring / k-anonymity / seq-lock logic — keep them in sync.

---

## Relay (new vision — R1–R8)

See [`requirements/objectives.md#relay`](requirements/objectives.md#relay--supervising-long-runs-from-anywhere) · [`architecture/relay.md`](architecture/relay.md) · [`journeys/relay.md`](journeys/relay.md). File issues as scoped:

- **Relay screen specs** — `docs/spec/screen/relay-*.md` for the 14 mockups (currently 0 specced). Star screens: Dashboard, TaskDetail, Decisions, Coordinator, DojoRelayGates.
- **Coordinator** — supervise agent CLIs (Claude Code · Codex · OpenCode · Aider), run the active plan in auto mode, publish filtered status, raise gates. New Observatory rail item.
- **Zero-knowledge relay transport** — encrypted pairing round-trip + scoped/revocable permissions; filtered status only; daemon outbound-only; adopt Apache-2.0 **ACP** (not Zed's GPL agent crate).
- **Planner data model** — plans → phases → features · checkpoints · gates; plan authoring marks gate steps.
- **Mobile companion app** — the phone surfaces.
- **Team relay (Dōjō)** — gates fan into a shared on-call queue with attribution.

---

## Mockup gaps — ✅ RESOLVED 2026-07-14

The 2026-07-14 mockup-gap pass is closed. Details in [`spec/MOCKUP-INDEX.md`](spec/MOCKUP-INDEX.md).

| Gap | Resolution |
|---|---|
| Stale spec component refs | fixed (`Splash`, `ProjAboutPane`) |
| Duplicate splash | `splash-healthcheck.jsx` retired to `discarded/` |
| 3 solution-track screens | drawn (`solution-track.jsx`) + specs wired |
| Relay specs | 13 written (`relay-*.md`) |
| Dōjō console per-role | split into admin/maintainer/lead + **developer**; all specced |
| Extensions/skill editors | `extensions-browser`/`skill-editor` retired; `agent-editor`/`persona-editor` specced |
| Benchmark runner | specced (`benchmark-runner.md`) |
| Prune orphans + `lib/` reorg | done (folders: shared/setup/observatory/project/dojo/relay/data/discarded) |
| Empty/loading/error states | `ScreenState` helper added; screens take a `state` prop |
| _also_ | in-app Dōjō flows → Observatory ⑦; ecosystem architecture board (⑧); `project-atlas` specced |

---

## Website

| Item | Summary |
|------|---------|
| _(file issue)_ | **On-page SEO** — canonical, OpenGraph, Twitter Card tags + a generated `sitemap.xml` (root `<svelte:head>`); submit to Search Console. |
| _(file issue)_ | **Website redesign** — screenshots→flows + a "For teams · 結 Dōjō" section + a Teams nav; **reconcile the "0 external requests / local-first" promise with the opt-in networked Dōjō**; trim Dōjō copy to shipped reality. |
