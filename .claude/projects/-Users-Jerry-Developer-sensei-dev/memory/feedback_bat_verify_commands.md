---
name: BAT must verify commands are actually callable
description: After any plugin/catalog change, test that slash commands actually work — don't just check files exist
type: feedback
---

After any change to commands, skills, or catalog.json, verify that every command on disk is registered in catalog.json. A command file that exists but isn't in the catalog is invisible to Claude Code.

**Why:** Missed this when retiring skills (#93) — 19 commands existed on disk but only 9 were in the catalog. The brainstorm command failed at runtime because it was never registered. BAT should have caught this by testing `/sensei:brainstorm` after the install.

**How to apply:** The install-plugin.sh gate check should verify catalog completeness. After any catalog change, run the check AND try invoking a command to confirm it works end-to-end.
