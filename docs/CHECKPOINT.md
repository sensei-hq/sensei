# Checkpoint

**Slice:** developer retrospective — session facets across ACPs (spec `docs/spec/2026-08-26-session-facets-and-retrospective-report.md`)

## Done

- **VS Code layer-2 parsing** — `GitHub.copilot-chat/transcripts/` is the Copilot CLI event format; `parse_events()` extracted and reused. Closed the `n/a` columns (rajkumar: 42h active, 0.6% of 10,930 calls failed).
- **Mechanical signals** — languages, commits/pushes, human reply time, derived from tool arguments in all three ACPs. Verified by independent recount (Balaji: TypeScript 12,358, C# 7,769, 580 commits — exact match).
- **Facet layer** — one local-ollama call per session → fixed-shape grounded record; report sections are group-bys, remedies a lookup table. **131 of 149 sessions** covered across the five users. Nothing left the machine.
- **Tables created and live** — `activity.session_facets`, `activity.session_facet_tags`, `sensei.goal_outcome`, `sensei.facet_tag_kind`. Applied via `dbd reconcile`; `dbd diff --exit-code` clean.
- **#123 A1 fixed** (both copies) — untrusted journal index no longer allocates; guard-removed mutation confirmed both tests fail.
- **Disk** — 17 G reclaimed (116 → 133 GiB free). Cleared the preflight NO-GO blocking #123 B2.

- **`crates/transcript-formats` extracted** — one VS Code journal reader for the daemon and the tool. Closes #124. All six reports byte-identical after the swap.
- **#123 A1–A4 fixed**, each verified by mutation. A2 took the tool's path depth *and* the daemon's richer URI handling — neither copy alone was right.
- **Three pre-existing suite flakes fixed** (#123 comment) — one class: `assemble_context` blends global scope and keeps top-N by strength, so other suites' memories crowd fixtures out. Confirmed pre-existing against a `3061931b` worktree. Two clean full runs since: 2407 passed, 0 failed.

## Next

1. `#123` **A5** — `SENSEI_VSCODE_SAMPLE` tests report `ok` while executing nothing. More visible now that the shared crate has real fixture tests.
2. `#123` **A6** — three denominators for "failure rate" ("2.0% of 5,000 tool calls" where 2.0% is 4/200).
3. Wire the daemon's process analyzer to populate the new facet tables (spec D1: same gated pass, not a second one).
4. Teach the shared crate VS Code's **layer-2 event stream** — the tool reads it, the daemon does not, and it carries the tool outcomes and turn timing the journal lacks.

## Open questions

- Does the goal vocabulary need to be per-ACP? A Copilot CLI session and a Claude Code session may not categorise alike.
- `activity.sessions` still has no `languages` / `git_commits` / reply-time columns — column additions, not a new capture path.

## Known-broken / caveats

- Facet coverage is 131/149. Dropped sessions are **named** in the run output, not counted anonymously. Balaji is the weakest (17/22) — long sessions, quote falls in the omitted middle.
- Facet `outcome` is the model's read of what the transcript SAYS, not a verified result; it skews positive because transcripts end on the assistant's last word.
- Full `senseid` suite green: 2407 passed, 0 failed, twice consecutively. Tool suite 24 pass, shared crate 15 pass, clippy 0 across all three.
- The daemon ingests VS Code journals only; layer-2 event streams are still unread daemon-side.
