---
name: Visualization & Dashboard
description: Visual interfaces for code graphs, complexity heatmaps, symbol exploration, traceability, and workspace overview
date: 2026-04-17
status: idea
sources: features/12-code-explorer.md, features/10-system-intelligence.md, gap-analysis.md
---

# Visualization & Dashboard

## Problem

The desktop app exists (Tauri + SvelteKit) but key dashboard pages are missing. Users can't visualize their codebase structure, explore symbols, see quality trends, or navigate the graph. The data exists in the daemon — it just needs to be surfaced.

## Current state

- Desktop Tauri app: implemented (SvelteKit webview + Rust backend)
- Project list view: implemented
- Card system / phase visualization: not in dashboard
- Symbol explorer: planned, not built
- Code graph visualization: planned, not built
- Complexity heatmap: planned, not built
- Traceability viewer: planned (traceability.json generated but page missing)
- Quality metrics dashboard: planned, not built
- Workspace multi-repo visualization: planned, not built

## What this idea covers

- **Symbol explorer**: searchable, filterable table of all indexed symbols with navigation to source
- **Code graph**: interactive dependency graph — zoom into modules, packages, or individual functions
- **Complexity heatmap**: per-file/function cyclomatic complexity visualization to identify hotspots
- **Traceability viewer**: design doc ↔ code coverage map; highlight undocumented code and stale docs
- **Quality dashboard**: FTR trends, session analytics, turn counts over time
- **Workspace overview**: multi-repo system view showing repo roles, health scores, cross-repo contracts
- **Phase/card visualization**: show active workflow phase, current task, progress through plan

## Open questions

- What's the UX priority? Symbol explorer and quality dashboard seem highest value.
- Should visualizations be desktop-only or also accessible via CLI/web?
- Interactive graph rendering: D3? Cytoscape? Something lighter?
- How does the graph filter UX work? (Cascading level dropdowns: L1:Repo, L2:Code/Docs, L3:Package/Category — from memory)
