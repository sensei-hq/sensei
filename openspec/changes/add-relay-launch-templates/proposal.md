# Change: Launch templates in the AddMember picker (#513)

## Why
The Crew view's AddMember picker lists the assistants and editors this machine
has seen, and picking one only rewords the note under the paste line
(`app/src/lib/relay/AddMember.svelte:4-7`). It gives no way to start the
picked tool: the person knows *what* to paste but not *where* — which terminal
command opens the assistant, or which step opens the editor's agent chat. #455
deferred this; `GET /relay/catalogue` was left ready for it.

## What Changes
- **Seeded templates.** `sensei.assistant_catalogue` gains `launch_template`
  (nullable text): one command string per tool with a `{repo}` placeholder —
  `cd "{repo}" && claude` for terminal assistants, an "open the agent chat"
  step for editors. Supporting a tool's launch stays a seed row; tools whose
  invocation is not known keep `null` and render no block (a wrong command is
  worse than none).
- **Resolved read.** `GET /relay/catalogue` takes `workspace_id` (and
  `crew_id` when adding to a crew) and returns each item's `launch`: the
  template with `{repo}` resolved to the crew's repo, else the workspace's
  first repo; `null` when the tool has no template or no repo resolves.
- **Own copyable block.** AddMember renders the picked tool's `launch` under
  the paste line as its own block with Copy, reusing the paste-line styling.
  Nothing picked, or `launch: null`, renders no block.

## Decisions (user, 2026-11-23)
- Single command-string column, not structured per-kind steps.
- Repo resolution: the crew's repo when adding to a crew, else the
  workspace's first repo.
- The template renders as its own copyable block, not folded into the note.
