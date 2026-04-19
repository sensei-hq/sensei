---
name: Plugin Developer
category: persona
description: Developer creating sensei plugins, skills, and commands
goals:
  - Create a skill quickly following documented patterns
  - Test that the skill triggers correctly
  - Understand how hooks, commands, and skills interact
pain_points:
  - Unclear skill trigger format — when does my skill fire?
  - No testing tools — can't verify without a full session
  - Hard to debug hook failures — silent errors
validates:
  - Does the skill description clearly state when to trigger?
  - Are error messages actionable (not just 'failed')?
  - Can I follow the docs end-to-end without guessing?
---

# Plugin Developer

A developer creating sensei plugins, skills, and commands. They know how to code but are new to the sensei plugin system.

## Journey

1. Reads plugin docs / examples
2. Creates a skill or command markdown file
3. Wants to test it — runs a session, tries the trigger
4. Iterates on the description until it fires reliably
5. Adds hooks for automation
6. Publishes to marketplace

## Questions

Ask these when building or changing anything this persona touches:

1. **Can I create this without reading source code?** — Docs and examples should be sufficient. If I have to grep the codebase to understand how to write a skill, the docs failed.
2. **How do I know it's working?** — Is there a test command, a dry-run mode, or at minimum a clear success/error message?
3. **What happens when I get it wrong?** — Typo in the trigger description, wrong frontmatter field, missing hook — does the system tell me, or does it silently ignore my plugin?
4. **Can I iterate quickly?** — Edit a file, reload, test. If the feedback loop requires restarting a daemon or reinstalling, it's too slow.
5. **Where do I go when stuck?** — Is there a /help command, an error message with a suggestion, or a troubleshooting guide?

## What frustrates them

- **Silent failures** — hook runs but nothing happens, no error, no log. They don't know if the hook didn't fire, fired and failed, or fired and succeeded but the output was wrong.
- **Trigger guessing** — skill descriptions are the trigger mechanism, but there's no way to test "will this description match this user prompt?" without running a full session.
- **Scattered docs** — plugin structure is in one place, skill format in another, hook events in a third. No single "build a plugin end-to-end" guide.
