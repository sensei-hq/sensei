# Review — 2026-10-27: every screen, for the walk

Five read-only, adversarial reviews (code + the live daemon, GET only + the mockups), one checklist per
screen. Go screen by screen; each section says what the screen is for, what it shows, which endpoints it
calls and whether their answers match, how it differs from the mockup or spec, and what to try.

| Review | Screens | Checklist items |
|---|---|---|
| [main window](2026-10-27-main-window.md) | Today, Workspaces, Sessions, Impact, Insights, Instruments, Libraries, Learnings, Consolidation, Logs, Sources, Traceability, Upgrades, Share review, Dōjō connections & sharing, Help, the rail and command palette | 150 |
| [workspace window](2026-10-27-workspace-window.md) | shell, Overview, About, Intake, Metrics (+ detail), Sessions, Memories, Patterns, Impact, Traceability, Instruments, Integrations, Libraries, Tickets, the diagrams (Code map, Structure, Cycles, Layers, Neighbourhood, Schema) | 137 |
| [settings & setup](2026-10-27-settings-setup.md) | the Settings window (11 pages), Setup (5 stages), Health, Logs, Upgrade, identities | 36 |
| [dōjō](2026-10-27-dojo.md) | every dōjō route, the `/v1` sync routes, roles | 70 |
| [roadmap vs state](2026-10-27-roadmap-vs-state.md) | each roadmap area and the five core targets, with evidence | — |

## Before the walk: what touches safety, money or the truth of the data

These are the findings to decide on first; none was introduced today, and none is made worse by v0.21.1.

**Security / privacy (dōjō and sharing)**
1. Any dōjō member can rewrite workspaces they cannot read — `attach_run_to_workspace`,
   `file_repository_workspace` gate on membership only ([dōjō](2026-10-27-dojo.md), top issue 5). The
   definer-function test covers a non-member; it needs a low-role member case too.
2. Seat binding looks up a namespace across tenants and swallows seat-write errors (dōjō, 8) — money-facing.
3. The last admin can be demoted; the self-guard compares a principal id with an auth id (dōjō, 3).
4. Share review publishes without reading the sharing preference — "Nothing leaves the machine" is not
   enforced on that path; small batches publish without confirmation (main window, 2).
5. The Playground labels writes and paid model calls as queries, with no confirmation (main window, 3).

**Data shown that is not true (the never-fabricate rule)**
6. Impact invents a 0% baseline (`COALESCE(baseline_ftr, 0)`), so every verdict reads "+75pp" (main 1,
   workspace 3).
7. Missing first-try rate drawn as 0 — Today's strip, Workspaces' "needs attention" (main 4).
8. A failed upgrade reads "Up to date" (settings 2); failed metric reads read as "no data yet"
   (workspace 6); the dōjō org home says "no workspaces … nothing is hidden" when its read fails, and it
   fails for every contributor and maintainer (dōjō 1).
9. A failing sync is invisible end to end — no sync_state row, the app stays green, the dōjō has no
   heartbeat (dōjō 2). This is why production looked empty.

**Blocking use**
10. Layers freezes the page (workspace 1 — a likely cause and a three-step bisect are in the review).
11. Overview can take minutes to appear (it waits on `/hotspots`); workspace Instruments 40–69 s, then zeros.

## Fixed today, after the reviews ran

- The settings window no longer doubles the app's global work (a second credential-renewal poller, menu
  events twice) and a setting saved there reaches the other windows (settings 4, 13) — 3aa6ff9f.
- The stale `dojo.upsert_repository_metrics` (a definer function production still had) is dropped — 5b955bd5.
- The settings review's "Libraries is probably empty in the new window" does not hold: the section
  refreshes its own data, and the settings e2e specs pass in the new window.
