---
name: Overview needs search, add, exclude, and live progress
description: Overview page must have search/filter, add-repo button, exclude/remove with exclude list surfaced in settings
type: feedback
---

Overview needs to be the primary action surface, not just a display:

1. **Search/filter** — filter the project list by name
2. **Add button** — add repos or scan folders (same as /all's scan input)
3. **Exclude/remove** — mark repos to exclude from indexing/watching. Excluded repos go to an exclude list.
4. **Exclude list in Settings** — user can view/modify excluded repos from Settings page
5. **Live indexing progress** — per-repo progress bars showing files processed/total, current file
6. **States visible** — queued, indexing, indexed, failed per repo

**Why:** The user expects one page to manage everything. /all had these actions but Overview didn't. Two separate pages fragments the experience.

**How to apply:** Merge /all's functionality into Overview. /all can redirect to /overview.
