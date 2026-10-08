# Checkpoint

**Slice:** diagrams → data hygiene → consent → session integrity. One row per
issue; closed only with evidence.

| # | item | status | to close |
|---|---|---|---|
| 220 | Neighbourhood diagram | CLOSED | — |
| 219 | World diagram | CLOSED — 3 live defects fixed `3527399a` | — |
| 231 | fqn segment 3 | CLOSED — unknownUnit 13.7% → 1.0% | — |
| 233 | diagram cache | CLOSED — hits 21→2.6 ms, 109→3 ms | — |
| 218 | transcript consent | CLOSED `ea9dd96d` | user switches Claude Code on |
| 248 | e2e rewrote ~/.claude/CLAUDE.md | CLOSED `5e58b4e3` | — |
| 247 | pruner + root removal | gates 1–5 done | gate 6: user runs the SQL below |
| 238 | Claude Code mods | 1 of 4 defects fixed `542cd859` | nudge-guard order (hook-only), then mods steps 1–5 |
| 221 | Schema diagram | BLOCKED — decision A/B/C | user decision |
| 236 | dōjō → kavach data routes | BLOCKED — plan invalidated (3/167 fit) | user decision on the reframe |
| 249 | relay: members read/answer each other's gates | filed | user: intended or not? |
| 225 | dōjō team view | queued | sync-shape decision |
| 239 | EPIC productionise (#240 #241) | queued | — |
| 245 | app e2e red gate | open | 4 groups, separate commits |

**Next command:** confirm `/tmp/sensei-install6.log` ends `exit=0` (deploys
`542cd859`), then the #238 nudge-guard reorder in
`marketplace/plugins/sensei/hooks/nudge` + `test-hooks.sh`.

**Orphan cleanup the classifier blocked** (user to run; newest backup
`database/backup/essential/20261007-200259`):
`DELETE FROM sensei.repositories r WHERE NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.repository_id = r.id) AND r.synced_at IS NULL AND r.tenant_id IS NULL AND NOT EXISTS (SELECT 1 FROM sensei.repositories_in_projects rp WHERE rp.repository_id = r.id);`
→ 7,795 rows. The 5,704 in history-bearing projects stay.

**Known broken:** e2e #245 (`daemon-verification` hook-configure is one of its
21). Marketplace hook changes are unpublished until `make bump`. Commits
unpushed on `develop`.
