# Change: A lane names the editor it runs in — the host tag (#396; classification is #455)

## Why
Transcripts are processed for assistants and editors alike (`assistant_family` mixes `claude-code`,
`cursor`, `zed`), but an editor runs an assistant behind the scenes: work and transcripts belong to
the assistant (model); the editor is only the host. The member card's host tag ("in Cursor", built
and benched) stays empty because the daemon never reports a lane's editor.

## What Changes
- Register accepts an optional `host` (the editor the session runs in), self-reported by the
  session; the lane stores and serves it; the member card shows "in <editor>" with its title.
- `/sensei:register` and the register guidance tell an editor-hosted session to pass `host`.
- Not here (user, 2026-11-16): classifying assistants vs editors in transcripts, attribution, the
  AddMember picker, and detecting the host from the environment — follow-up #455.

## Impact
- Specs: `relay-crew` (+ the transcript attribution it relies on).
- Code: senseid register (host), lane DDL + now/crew reads, `@sensei/state` crew selectors
  (`editorTag` already exists), the register command and guidance.
