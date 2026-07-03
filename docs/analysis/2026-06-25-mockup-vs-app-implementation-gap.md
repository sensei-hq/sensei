---
title: Mockup vs App Frontend Implementation — Gap Analysis
description: Detailed comparison between the redesigned mockup screens (docs/mockups/Sensei) and the Svelte app frontend implementation (app/src/routes/) to identify layout gaps, missing features, and implementation differences.
type: analysis
status: completed
created: 2026-06-25
references:
  - docs/mockups/Sensei/
  - app/src/routes/
---

# Mockup ↔ App Frontend Implementation Gap Analysis

This document provides a page-by-page, section-by-section comparison between the visual designs and interactive scripts in [docs/mockups/Sensei/](file:///Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/Sensei/) and the Svelte-based frontend implementation inside [app/src/routes/](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/).

The goal of this audit is to highlight UI discrepancies, completely missing screens, stubbed/mocked pages, and visualization differences, enabling developers to align the frontend with the Zen-Sumi design system.

---

## Verdict in One Line

While the onboarding wizard is largely complete (except for model fallback assignments), the core **Observatory** and **Project** daily views are highly simplified, stubbed, or missing rich interactive layouts (such as logs timeline, doc traceability diffs, memory anatomy, project groupings, and the entire **Dōjō Console** federation suite).

---

## Screen-by-Screen Summary Table

| Screen/Section | Mockup Reference | App Implementation Path | Status | Key Discrepancy |
|---|---|---|---|---|
| **Onboarding: Welcome** | `01`/`02` Splash | [welcome/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/welcome/+page.svelte) | ✅ Aligned | Identical copy and structure. |
| **Onboarding: Profile** | `preferences` | [preferences/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/preferences/+page.svelte) | ✅ Aligned | Identical toggle states. |
| **Onboarding: Assistants** | `acps` | [assistants/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/assistants/+page.svelte) | ✅ Aligned | Configures connected assistant tools. |
| **Onboarding: Folders** | `folders` | [roots/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/roots/+page.svelte) | ✅ Aligned | Configures watched paths. |
| **Onboarding: Scan** | `scan` | [scan/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/scan/+page.svelte) | ✅ Aligned | Shows recursive folder indexing. |
| **Onboarding: Projects** | `projects` | [projects/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/projects/+page.svelte) | ✅ Aligned | Repos configuration & folder roles. |
| **Onboarding: Libraries** | `libraries` | [libraries/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/libraries/+page.svelte) | ✅ Aligned | Lib wrapping toggle. |
| **Onboarding: Instruments**| `registry` | [instruments/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/instruments/+page.svelte) | ✅ Aligned | MCP server installation. |
| **Onboarding: Routers** | `inference` | [inference/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/inference/+page.svelte) | ✅ Aligned | API key configuration. |
| **Onboarding: Assignments**| `assignments` | *None* | ❌ Missing | **Fallback prioritizer per role is completely absent.** |
| **Onboarding: Enter** | `done` | [done/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/done/+page.svelte) | ✅ Aligned | Summary of setup completion. |
| **Today Dashboard** | `06-today` | [+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/+page.svelte) | ⚠️ Diff | Uses line graph sparkline instead of daily bar strip (`ObsFtrStrip`). Missing corrections column in sessions. |
| **Observatory: Projects** | `07-projects` | [projects/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/projects/+page.svelte) | ⚠️ Simplified | Flat grid of buttons. Missing FTR rates, session counts, active/recent sections. |
| **Observatory: Sessions** | `09-sessions` | [sessions/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/sessions/+page.svelte) | ⚠️ Simplified | Flat list of sessions. Missing timeline, logs viewer, search, and details. |
| **Observatory: Insights** | `10-insights` | [insights/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/insights/+page.svelte) | ❌ Stub | Empty state mockup rendering `MemoryList` with no loader or props. |
| **Observatory: Learnings** | `11-memory` | [learnings/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/learnings/+page.svelte) | ⚠️ Simplified | Missing rich Memory Anatomy tabs (What/Because/Consequence). |
| **Observatory: Libraries** | `12-libraries` | [libraries/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/libraries/+page.svelte) | ⚠️ Aligned | Basic usage listing. Missing drift-to-code details. |
| **Observatory: Instruments**| `13-instruments`| [instruments/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/instruments/+page.svelte) | ⚠️ Stub | Playground & stats are live. Replay tab is empty stub. |
| **Observatory: Upgrades** | `14-upgrades` | [upgrades/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/upgrades/+page.svelte) | ❌ Stub | Renders only an empty state mock. |
| **Observatory: Impact** | `15-impact` | [impact/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/impact/+page.svelte) | ❌ Stub | Renders only an empty state mock. |
| **Observatory: Settings** | `16-collective` | [settings/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/settings/+page.svelte) | ⚠️ Stub | General configuration is read-only. Inference configuration is placeholder. Missing Collective/Federation settings. |
| **Project: Overview** | Project Overview | [overview/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(project)/project/[id]/overview/+page.svelte) | ✅ Aligned | Shows FTR delta, sparklines, signals, hotspots. |
| **Project: Sessions** | Project Sessions | [sessions/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(project)/project/[id]/sessions/+page.svelte) | ⚠️ Simplified | Simple text list of sessions. Missing timelines and diffs. |
| **Project: Memories** | Project Memories | [memories/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(project)/project/[id]/memories/+page.svelte) | ⚠️ Simplified | Simple list. Missing Memory Anatomy view and synthetic examples. |
| **Project: Traceability** | Project Trace | [traceability/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(project)/project/[id]/traceability/+page.svelte) | ⚠️ Simplified | Flat text list of drift statuses. Missing signature diffs. |
| **Project: Patterns** | Project Patterns | [patterns/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(project)/project/[id]/patterns/+page.svelte) | ⚠️ Simplified | Simple list of followed/anti-patterns. Missing FTR delta metrics. |
| **Project: Impact** | Project Impact | [impact/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(project)/project/[id]/impact/+page.svelte) | ✅ Aligned | Shows before/after FTR delta metrics and verdict list. |
| **Project: About** | Project Settings | [about/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/about/+page.svelte) | ✅ Aligned | Renders name, client, repos, and stack tags. |
| **Dōjō Console** | Dojo | *None* | ❌ Missing | **The entire Dōjō Console, journey mapping, and federation client are absent.** |

---

## Detailed Page-by-Page Comparison

### 1. Onboarding & Setup Flow

*   **Welcome (`/setup/welcome`)**
    *   *Mockup*: Renders greeting "A teacher does not write the code" and details basic observe/teach/local principles.
    *   *App*: Matches mockup perfectly in [welcome/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/welcome/+page.svelte).
*   **Preferences / Profile (`/setup/preferences`)**
    *   *Mockup*: Setting display name, shared learnings cadence, and behavioral correction aggressiveness.
    *   *App*: Matches mockup perfectly in [preferences/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/preferences/+page.svelte).
*   **Assistants (`/setup/assistants`)**
    *   *Mockup*: Detects and configures extensions/assistants (Claude Code, Cursor, Copilot).
    *   *App*: Matches mockup in [assistants/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/assistants/+page.svelte).
*   **Roots / Folders (`/setup/roots`)**
    *   *Mockup*: Watch roots configuration.
    *   *App*: Matches mockup in [roots/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/roots/+page.svelte).
*   **Scan (`/setup/scan`)**
    *   *Mockup*: Multi-folder scanner surfacing repos and code graph progress.
    *   *App*: Matches mockup in [scan/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/scan/+page.svelte).
*   **Projects (`/setup/projects`)**
    *   *Mockup*: Configures sibling directories, name, stack tags, and folder roles (frontend, backend, library).
    *   *App*: Matches mockup in [projects/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/projects/+page.svelte).
*   **Libraries (`/setup/libraries`)**
    *   *Mockup*: Lists third-party dependencies, toggling wrappers.
    *   *App*: Matches mockup in [libraries/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/libraries/+page.svelte).
*   **Instruments (`/setup/instruments`)**
    *   *Mockup*: Suggests and configures MCP servers.
    *   *App*: Matches mockup in [instruments/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/instruments/+page.svelte).
*   **Inference (`/setup/inference`)**
    *   *Mockup*: Provider list and API key configurations.
    *   *App*: Matches mockup in [inference/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/inference/+page.svelte).
*   **Model Assignments (`assignments`) — ❌ MISSING**
    *   *Mockup*: Defined in [wiz-assignments.jsx](file:///Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/Sensei/lib/wiz-assignments.jsx). Displays a split layout (left: reasoning roles, right: prioritize model chips + fallback configuration).
    *   *App*: **No assignments route or screen exists.** In Svelte [stages.ts](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/stages.ts), the Assignments stage is missing entirely.
*   **Enter / Done (`/setup/done`)**
    *   *Mockup*: Renders configured summary.
    *   *App*: Matches mockup in [done/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(config)/setup/done/+page.svelte).

---

### 2. Observatory (Collective Window)

*   **Today / Home Dashboard (`/`)**
    *   *Discrepancy (Visualization)*: The mockup [observatory-today.jsx](file:///Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/Sensei/lib/observatory-today.jsx) defines `ObsFtrStrip` as a 14-day daily bar graph highlighting individual daily FTR ratios. The Svelte [+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/+page.svelte) implementation renders this ratio as a classic sparkline (smooth path line graph).
    *   *Discrepancy (Data columns)*: The recent sessions table in the mockup includes a `Corrections` count column (e.g., `first-try` or `3x`). The Svelte sessions list in [RecentSessions.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/RecentSessions.svelte) omits this column, only displaying project, task name, duration, and start time.
*   **Projects List (`/projects`)**
    *   *Discrepancy (Simplification)*: The mockup shows segmented project sections (Active, Recent, Archived) where each card displays FTR%, weekly session counts, and warning indicator dots. The Svelte [projects/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/projects/+page.svelte) renders a flat, unsegmented grid of buttons displaying only the project name, client name, maturity, and a link arrow.
*   **Sessions Log (`/sessions`)**
    *   *Discrepancy (Simplification)*: The mockup [sessions-zen.jsx](file:///Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/Sensei/lib/sessions-zen.jsx) contains interactive timeline logs, turns-rework counters, diffs, and detailed session analysis drawers. The Svelte [sessions/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/sessions/+page.svelte) is a simple list showing outcome and started time with standard filter buttons.
*   **Insights Triage (`/insights`)**
    *   *Discrepancy (Mocked)*: The mockup triage displays Now/Soon/Settled triage recommendations list. The Svelte [insights/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/insights/+page.svelte) only renders the generic [MemoryList.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/lib/components/MemoryList.svelte) component but **passes no data loader or props**, rendering it perpetually empty.
*   **Learnings (`/learnings`)**
    *   *Discrepancy (Simplification)*: The Svelte [learnings/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/learnings/+page.svelte) and [MemoryDetail.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/learnings/MemoryDetail.svelte) show lists of Active/Archive/Triage items and basic fields (content, metrics, simple list of evidence/examples). However, they lack the mockup's structured "Memory Anatomy" tab splits (What / Because / Consequence), scope-ladder edit controls, and inline code snippet displays.
*   **Libraries (`/libraries`)**
    *   *Discrepancy (Simplification)*: The Svelte [libraries/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/libraries/+page.svelte) implements searching, usage-by-folder stats, and active repo imports count. It lacks the mockup's document drift code-to-symbol mismatch diffs.
*   **Instruments (`/instruments`)**
    *   *Discrepancy (Mocked)*: The Svelte [instruments/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/instruments/+page.svelte) features a working Playground and Insights page. However, the **Replay tab is a stub** rendering `EmptyState` ("Session replay - Tool calls from your assistant sessions will appear here").
*   **Upgrades (`/upgrades`) — ❌ STUB**
    *   *Discrepancy (Mocked)*: The Svelte [upgrades/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/upgrades/+page.svelte) is a stub that only displays `EmptyState` ("no upgrades waiting"). The mockup [upgrades.jsx](file:///Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/Sensei/lib/upgrades.jsx) contains interactive lists of recommended skills, lints, and agents with installer drawers.
*   **Change Impact (`/impact`) — ❌ STUB**
    *   *Discrepancy (Mocked)*: The Svelte [impact/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/impact/+page.svelte) is a stub that only displays `EmptyState` ("no measurements yet"). The mockup [impact.jsx](file:///Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/Sensei/lib/impact.jsx) includes detailed MOE (Mixture of Experts) verdicts, delta corrections, and consensus reviews.

---

### 3. Project Window (Scoped Views)

Every sub-route under [project/[id]/](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(project)/project/[id]/) shares the left project sidebar:

*   **Project Overview (`/overview`)**
    *   *Status*: Aligned. Correctly handles FTR daily rates, sparkline paths, recent sessions, hotspots, and adopted teachings in [overview/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(project)/project/[id]/overview/+page.svelte).
*   **Project Sessions (`/sessions`)**
    *   *Discrepancy (Simplification)*: Renders a simple flat list of sessions with outcome, FTR dot, and date. It lacks detailed timelines or rework tracking.
*   **Project Memories (`/memories`)**
    *   *Discrepancy (Simplification)*: Shows a basic list of memories containing title, type, and strength. It does not provide the detail drawer, code references, or full "Memory Anatomy" cards.
*   **Project Traceability (`/traceability`)**
    *   *Discrepancy (Simplification)*: Displays a basic list of drifted doc files with a status indicator dot, but lacks expected-vs-actual signature diffs or code preview capabilities.
*   **Project Patterns (`/patterns`)**
    *   *Discrepancy (Simplification)*: Lists followed and anti-patterns with a basic lifecycle tag, but omits delta FTR percentage metrics.
*   **Project Impact (`/impact`)**
    *   *Status*: Aligned. Shows before/after FTR delta metrics and verdict list.
*   **Project About (`/about`)**
    *   *Status*: Aligned. Displays project goal, client name, repositories path list, and stack tags.

---

### 4. Settings Page

The main Settings view in [settings/+page.svelte](file:///Users/Jerry/Developer/sensei-hq/sensei/app/src/routes/(observatory)/settings/+page.svelte) has four tabs:

*   **General**:
    *   *Discrepancy*: The Svelte settings page renders general config as a read-only list of key-value pairs. It does not allow users to modify display names, behavior nudge rules, or telemetries (unlike the onboarding preferences page).
*   **Assistants**:
    *   *Status*: Aligned. Correctly lists detected assistants and their configured statuses.
*   **Inference — ❌ STUB**
    *   *Discrepancy*: Displays a placeholder message: *"Inference configuration will be available once model assignments are supported."*
*   **Extensions**:
    *   *Discrepancy*: Renders a basic list of extension names, types, and on/off status text. It does not have actions to update, configure, toggle, or install extensions.
*   **Collective/Federation Settings — ❌ MISSING**
    *   *Discrepancy*: The mockup has a dedicated settings page (`16-collective`, `collective-settings.jsx`) to manage sharing settings, review queues, category exclusions, and contribution metrics. **This is completely missing in the Svelte app settings.**

---

### 5. Dojo & Federation Governance — ❌ DEFERRED / MISSING

*   **Dōjō Console & Journey Map**
    *   *Mockup*: Defines [Sensei Dōjō Console.html](file:///Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/Sensei/Sensei%20Dōjō%20Console.html) and [Sensei Dōjō Journey Map.html](file:///Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/Sensei/Sensei%20Dōjō%20Journey%20Map.html) with detailed multi-org membership, project-to-org binding, upstream sharing redaction previews, downstream conflict scopes (org/team/global/personal rules), and maintainer triage queue interfaces.
    *   *App*: **No Dōjō routes, sections, sidebars, or pages exist.** The Svelte app does not support any federated governance or shared teams views. *(Note: This aligns with the Rev-3 scope decision to prioritize standalone Sensei features first, deferring Dojo/hive integrations).*
