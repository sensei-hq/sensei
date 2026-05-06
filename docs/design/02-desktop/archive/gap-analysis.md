# Gap Analysis — Sitemap vs Design Docs

> Compares `sitemap.md` (what's built + planned) against `ux-redesign.md`, `observatory-analysis.md`, and `views.md` (what was designed).

---

## 1. Sidebar Navigation

| Design (ux-redesign) | Sitemap | Gap |
|---|---|---|
| Active solutions (expandable with sub-pages) | Recent section (solutions + projects mixed) | **Simplification.** Design wanted expandable solution sub-nav in sidebar. Sitemap flattens to links — sub-pages via tabbed layout on the solution page instead. Keep sitemap approach. |
| Libraries section (standalone libs shared across 2+ solutions) | Libraries in global nav | **Missing: standalone library promotion.** Design auto-promotes libs used by 2+ solutions to sidebar. Sitemap treats libraries as a global docs browser. Need to reconcile — library docs page is useful, but the "shared dependency" concept is missing. |
| Side Projects / Ideas categories | Not in sitemap | **Dropped.** Design had Active/Side/Ideas categories with drag-to-recategorize. Sitemap has a flat Recent list. The categorization added complexity without clear value for v1. Keep dropped. |
| `+ Add solution` button in sidebar | Not in sitemap | **Missing.** Should be in sidebar or accessible from overview. |

**Decision needed:** Keep flat Recent list (sitemap) but add a solution create action somewhere visible.

---

## 2. Global Routes

| Design (ux-redesign) | Sitemap | Gap |
|---|---|---|
| `/overview` — redirects to last active solution | `/overview` — all projects list | **Different purpose.** Design wanted overview to jump to the active solution. Sitemap uses it as the project browser. Both are needed — the project browser is the "all repos" view from the design. |
| `/all` — flat repo list, import, assign | Marked for removal | **Conflict.** Design needs an "all repos" page for import/assignment. Sitemap's `/overview` serves this role. Rename in design, keep `/overview`. |
| `/acp` — ACP registry page | Marked for removal (covered by settings) | **Agreed.** ACP config in settings is sufficient. |
| `/libraries` — design removes this, inlines into repo cards | Sitemap keeps as global page | **Disagreement.** Design says "no separate libraries page — inline in Repos." But the current `/libraries` page is for external library *documentation* (e.g., index Svelte docs), not repo dependencies. These are different things. Keep `/libraries` for external doc browsing. |
| `/tools` — not mentioned in design | Sitemap keeps | **Missing from design.** The MCP tools explorer is useful for debugging. Keep. |
| `/sessions` — design scopes to solution only | Sitemap adds global sessions to nav | **Extension.** Design only has solution-scoped sessions. A global view across all projects is useful. Keep both. |
| `/benchmarks` — not in design | Sitemap drops from nav | **Agreed.** No backend, defer. |
| `/skills` (global catalog) | Sitemap has `/catalog` | **Rename.** Design has per-solution skill config (tiers, stages). Global catalog is for browsing/installing marketplace items. These are different views. `/catalog` = marketplace browser (global), `/s/{id}/skills` = solution skill config. |

---

## 3. Solution Routes

| Design (ux-redesign) | Sitemap | Gap |
|---|---|---|
| `/s/{id}` — Overview with connection diagram, index health, developer impact panel | `/s/{id}` — stats, cross-repo, graph preview, recent sessions | **Missing: connection diagram.** Design has a rich overview with topology diagram derived from infra files + cross-repo imports. Sitemap has basic stats. The connection diagram is a key differentiator — needs daemon support for infra detection. |
| `/s/{id}/repos` — expandable cards with inline Symbols/Docs/Index/Graph tabs | `/s/{id}/repos` — add/remove repos, role badges | **Missing: expandable inline detail.** Design collapses project detail into the repo card itself. Sitemap keeps repos page simple (management only) and uses `/s/{id}/p/{pid}` for detail. Design approach is more ambitious but the sitemap's separation is cleaner for now. |
| `/s/{id}/arch` — Structural + Deployment + Data Flow views | `/s/{id}/arch` — Structural + Doc drift | **Missing: Deployment view, Data Flow view.** Design has three sub-views. Sitemap has two. Deployment view needs infra file detection (daemon). Data flow needs API endpoint + DB table extraction. Both are Phase 2+. |
| `/s/{id}/trace` — Requirements → Design → Code → Tests chain | Sitemap marks for removal as "duplicate of arch" | **Wrong call.** Traceability is NOT the same as architecture. The design's trace view shows requirement coverage gaps across the chain. This is a distinct feature. **Restore `/s/{id}/trace` — but it depends on doc classification in the daemon (Phase 3 in design).** |
| `/s/{id}/sessions` | Same in both | Aligned. Both stub. |
| `/s/{id}/profiles` — solution-level mindsets, personas, rules | Not in sitemap | **Missing.** Design has a Profiles page showing active mindsets/personas/rules with agent promotion. This is where the observatory's "how am I doing" meets the coaching system. Add to sitemap as future. |
| `/s/{id}/sources` — repos + folders + future connectors | Sitemap marks for removal | **Premature removal.** Design distinguishes "repos" (management) from "sources" (extensible — git repos, folders, future connectors). For v1 they're the same, but the Sources concept matters when non-git sources arrive. Keep `/s/{id}/repos` for now, add Sources when needed. |

---

## 4. Project Routes

| Design (ux-redesign) | Sitemap | Gap |
|---|---|---|
| `/s/{id}/p/{pid}` — dashboard | Same | Aligned. |
| `/s/{id}/p/{pid}/sessions` — project sessions | Not in sitemap | **Missing.** Design has per-project session view. Add. |
| `/s/{id}/p/{pid}/code` — graph, complexity, dead code | Sitemap marks for removal (merge into project detail) | **Agreed for now.** Inline code analysis into the project detail page. Separate route later if it grows. |
| `/s/{id}/p/{pid}/profiles` — project-specific overrides | Not in sitemap | **Missing.** Design has per-project profile overrides. Future — depends on profile cascade. |
| `/s/{id}/p/{pid}/indexer` — indexing status per project | Not in sitemap | **Missing.** Design inlines this in the repo card's Index tab. Either approach works. |

---

## 5. Views (from views.md)

| View | In sitemap? | Gap |
|---|---|---|
| Quality dashboard (FTR trend, rework rate, tool adherence) | No | **Missing entirely.** This is the core observatory view. Needs daemon metrics API (`/api/metrics/:proj`). |
| Phase timeline (visual phase transitions with duration bars) | No | **Missing.** Needs daemon phases API (`/api/phases/:proj`). |
| Event log (filterable, expandable, correction highlighting) | No | **Missing.** Needs daemon events API (`/api/events/:proj`). |
| Pattern catalog (detected patterns, conformance, heatmap) | No | **Missing.** Needs pattern detection in daemon (`/api/patterns/:proj`). |
| Guided coaching (recommendations, action recipes, prompt preview) | No | **Missing.** Depends on quality dashboard + pattern data. |
| Active work (current phase, task, issue from workflow state) | No | **Missing.** Needs workflow state API (`/api/state/:proj`). |

**All six views from views.md are missing from the sitemap.** These are the observatory's analytical layer. They all depend on daemon APIs that don't exist yet.

---

## 6. Observatory Model (from observatory-analysis.md)

| Concept | In sitemap? | Gap |
|---|---|---|
| Three scope levels (Global > Solution > Project) | Partially — routes exist for all three | Aligned structurally, but metrics/dashboards missing at each level. |
| Active/Recent/Inactive/Archived project states | No — sidebar shows Recent only | **Missing.** No lifecycle management. Projects are either shown or not. |
| Session identity via CWD | Not addressed | Implementation detail — daemon side. |
| Profile cascade (global → solution → project) | No | **Missing from UI.** No profiles page at any level. |
| Capability registry (real/workaround/unavailable) | No | **Missing.** Daemon concept — UI would show which metrics are available vs blocked. |
| Developer Impact panel (FTR, tokens, cost, pattern reuses) | No | **Missing.** The signature observatory feature. |
| Agents (mindset wrapping, promotion, generic persona agent) | No | **Missing from UI.** Agent management not in any page. Design puts it in Profiles page. |
| Data capture events (turn, revision_requested, tool_used, etc.) | No | **Missing.** No event visualization. |

---

## Summary: What Needs to Change

### Sitemap corrections (wrong calls)

1. **Restore `/s/{id}/trace`** — traceability is not architecture. Different data, different purpose.
2. **Don't remove `/s/{id}/sources`** prematurely — rename to `/s/{id}/repos` for v1, but keep the extensibility concept in mind.

### Missing from sitemap (add as future phases)

3. **Quality dashboard** — the core observatory view. Where: solution overview or dedicated tab.
4. **Developer Impact panel** — FTR, tokens, cost. Where: solution overview.
5. **Event log** — filterable session events. Where: session detail page.
6. **Pattern catalog** — detected patterns + conformance. Where: new solution tab or inside arch.
7. **Profiles page** — `/s/{id}/profiles` for mindsets, personas, rules, agent promotion.
8. **Connection diagram** — infra-derived topology on solution overview.
9. **Deployment + Data Flow views** — additional tabs in `/s/{id}/arch`.
10. **Per-project sessions** — `/s/{id}/p/{pid}/sessions`.
11. **Guided coaching** — recommendations based on metric trends.

### Things the sitemap got right that the design overcomplicated

12. **Flat Recent sidebar** over Active/Side/Ideas categories — simpler, less conceptual overhead.
13. **Separate `/libraries` page** for external docs — design tried to inline everything into repo cards.
14. **Tabbed solution layout** over expandable sidebar sub-nav — cleaner navigation.
15. **`/catalog` as global marketplace browser** — design mixed this with per-solution skill config.
16. **Global `/sessions`** — useful cross-project view the design didn't include.

### Daemon APIs needed before UI work

| API | Blocks |
|-----|--------|
| `/api/metrics/:proj` | Quality dashboard, Developer Impact |
| `/api/events/:proj` | Event log, session detail |
| `/api/phases/:proj` | Phase timeline |
| `/api/patterns/:proj` | Pattern catalog |
| `/api/state/:proj` | Active work view |
| Infra file detection | Connection diagram, Deployment view |
| Doc classification | Traceability view |
| `log_event()` MCP tool | All event-based metrics |
