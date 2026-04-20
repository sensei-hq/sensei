---
name: Solution model — multi-repo logical grouping
description: Solutions are logical wrappers around multiple repos (and future external sources) — auto-detected from subtrees or manually curated. The scope hierarchy is Global > Solution > Project.
type: project
---

Solutions group related repositories into a logical unit. A solution represents a product or system that spans multiple repos.

**Why:** Real products are multi-repo — API, frontend, mobile, docs, infra. The developer thinks in terms of the solution ("Acme Platform") not individual repos. Cross-repo connections (API calls frontend, docs describe API) only make sense within a solution context.

**How to apply:**
- Three-level scope: Global > Solution > Project
- Solution pages show cross-repo metrics, merged graphs, shared libraries
- Project pages show single-repo data
- Global pages are solution/project agnostic
- Solutions can include future external sources (Confluence, Jira, wiki) — design data connectors as pluggable
