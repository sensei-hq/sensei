---
name: Overview (metrics) and Workspace (repos) are separate pages
description: Overview = metrics dashboard with actionable insights. Workspace = repo management (add, exclude, solutions, indexing progress). Setup redirects to workspace during indexing, then to overview once stable.
type: feedback
---

Current /overview page is actually a workspace (manage repos, scan folders, solution management). The observatory design has these as separate concerns:

- **Overview** = metrics dashboard (FTR, sessions, rework, tool adherence + actionable insights)
- **Workspace** = repo management (add folders, indexing progress, solution grouping, exclude)

Setup should redirect to workspace (see indexing progress) then user navigates to overview (see metrics) once projects are indexed.

**How to apply:** Rename current /overview to /workspace. Build a new /overview that shows metrics + insights. Both accessible from sidebar.
