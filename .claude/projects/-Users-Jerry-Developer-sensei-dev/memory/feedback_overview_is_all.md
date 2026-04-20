---
name: Overview must be /all evolved, not a separate read-only page
description: The Overview page should combine project management (add folders, scan, indexing progress) with the metrics dashboard — not be a static display separate from /all
type: feedback
---

The /all page already has working: add folder, scan, indexing progress with states (queued, indexing, done). The Overview was built as a separate read-only metrics page that doesn't show progress or allow actions.

**Why:** The user expects one home page that does everything. Two pages (/all for management, /overview for metrics) fragments the experience. The user should never have to go to a different page to trigger a scan or see indexing progress.

**How to apply:** Merge /all's scan/progress/management functionality into the Overview page. The Overview becomes the single entry point: metrics at top, project list with live indexing progress below, add folder action available. /all can redirect to /overview.
