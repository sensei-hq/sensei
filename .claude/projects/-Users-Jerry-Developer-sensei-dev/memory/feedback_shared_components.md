---
name: Modularize before building — no duplicate implementations
description: Before building any new UI, check if a similar component exists. Extract shared components. Never duplicate add-folder, repo-list, progress-bar, or any other pattern.
type: feedback
---

When building UI, always check for existing implementations first. If something similar exists, extract the shared parts into a component before building the new page.

**Why:** The setup wizard, /all page, and Overview all need add-folder, repo-list-with-progress, and exclude/select. Building each separately creates three implementations that drift apart.

**How to apply:** Before writing any page-level code, identify the reusable parts and extract them into `$lib/components/`. Pages compose these components — they don't re-implement them. If you find yourself copying code from another page, stop and extract.
