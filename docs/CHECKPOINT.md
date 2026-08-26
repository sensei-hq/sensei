# Checkpoint

**Slice:** developer retrospective — session facets across ACPs (spec `docs/spec/2026-08-26-session-facets-and-retrospective-report.md`)

## Done

- **VS Code layer-2 parsing** — `GitHub.copilot-chat/transcripts/` is the Copilot CLI event format; `parse_events()` extracted and reused. Closed the `n/a` columns (rajkumar: 42h active, 0.6% of 10,930 calls failed).
- **Mechanical signals** — languages, commits/pushes, human reply time, derived from tool arguments in all three ACPs. Verified by independent recount (Balaji: TypeScript 12,358, C# 7,769, 580 commits — exact match).
- **Facet layer** — one local-ollama call per session → fixed-shape grounded record; report sections are group-bys, remedies a lookup table. **131 of 149 sessions** covered across the five users. Nothing left the machine.
- **Tables created and live** — `activity.session_facets`, `activity.session_facet_tags`, `sensei.goal_outcome`, `sensei.facet_tag_kind`. Applied via `dbd reconcile`; `dbd diff --exit-code` clean.
- **#123 A1 fixed** (both copies) — untrusted journal index no longer allocates; guard-removed mutation confirmed both tests fail.
- **Disk** — 17 G reclaimed (116 → 133 GiB free). Cleared the preflight NO-GO blocking #123 B2.

## Next

1. `#123` A2 (workspace.json parent depth), A3 (empty-path poison pill), A4 (fabricated 1970 timestamps) — all daemon-side, all "make the daemon match the tool".
2. `#124` — extract the shared journal-replay crate. Do it **with** A2–A4, not after: those fixes move into the same crate, so doing them first means writing them twice.
3. Wire the daemon's process analyzer to populate the new facet tables (spec D1: same gated pass, not a second one).

## Open questions

- Does the goal vocabulary need to be per-ACP? A Copilot CLI session and a Claude Code session may not categorise alike.
- `activity.sessions` still has no `languages` / `git_commits` / reply-time columns — column additions, not a new capture path.

## Known-broken / caveats

- Facet coverage is 131/149. Dropped sessions are **named** in the run output, not counted anonymously. Balaji is the weakest (17/22) — long sessions, quote falls in the omitted middle.
- Facet `outcome` is the model's read of what the transcript SAYS, not a verified result; it skews positive because transcripts end on the assistant's last word.
- Full `senseid` suite not run this session — only the new guard test. Tool suite: 27 pass, clippy 0.
