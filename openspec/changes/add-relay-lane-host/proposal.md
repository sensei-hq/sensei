# Change: Tell assistants from editors; a lane names the editor it runs in (#396)

## Why
Transcripts are processed for assistants and editors alike (`assistant_family` mixes `claude-code`,
`cursor`, `zed`), but an editor runs an assistant behind the scenes: work and transcripts belong to
the assistant (model); the editor is only the host. The member card's host tag ("in Cursor", built
and benched) stays empty because the daemon never reports a lane's editor.

## What Changes
- A seen tool is classified `kind: assistant | editor` (CONTRACT §3l); sessions and transcripts carry
  the `host` editor when one was involved, and attribute work to the assistant (model).
- Register accepts an optional `host` (the editor the session runs in); the lane stores and serves
  it; the member card shows "in <editor>" with its explaining title.
- AddMember's picker lists **Assistants** and **Editors** from what this machine has seen (the
  catalogue's first cut: names and when last seen; launch templates later).

## Impact
- Specs: `relay-crew` (+ the transcript attribution it relies on).
- Code: senseid register (host), lane DDL + now/crew reads, session/transcript classification,
  `@sensei/state` crew selectors (`editorTag` already exists), AddMember picker.
