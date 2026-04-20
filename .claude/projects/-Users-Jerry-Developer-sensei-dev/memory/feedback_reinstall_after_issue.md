---
name: Reinstall sensei after every issue
description: After completing each issue, run install-plugin.sh and verify gate check passes — ensures mindsets, personas, hooks stay wired correctly
type: feedback
---

After completing any issue, run `./scripts/install-plugin.sh` to reinstall the plugin and verify the gate check passes. This catches drift between plugin source (marketplace/) and the local .sensei/ copy.

**Why:** Changes to mindsets, personas, hooks, or the session-start script can silently break the session-start experience. The install script copies files and runs the gate check — if anything's missing or stale, it surfaces immediately.

**How to apply:** At the end of every issue, before marking complete: `./scripts/install-plugin.sh`. All items should show `[ok]`. If any show `[FAIL]` or `[WARN]`, fix before moving on.
