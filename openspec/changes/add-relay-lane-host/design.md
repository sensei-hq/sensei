## Context
CONTRACT §3l: each seen tool has `kind: assistant | editor`; transcripts attribute to the assistant;
an editor is the host. Today `assistant_family` mixes both.

## Open Questions
- Q1: Scope — ship the lane `host` (small; unblocks the tag) first and the transcript
  classification as its own change, or both together?
- Q2: How does a session know its host? Self-report at register (`host`), or detect from the
  environment (e.g. `TERM_PROGRAM`, the MCP client name) — and trust which when they disagree?
