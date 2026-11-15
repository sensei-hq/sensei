# GitLab CE 19.5.3, one closed group milestone

The same group as `../ce-no-milestones`, recorded 2026-11-07 after one group
milestone was created for these fixtures: "Sprint 1" (scrubbed to "Milestone
1"), 2026-05-26 → 2026-06-14, **closed**, holding the 32 issues closed in that
window. The 5 issues closed in July and the 15 open ones are in no milestone.

What it adds (spec `docs/spec/daemon/tracker-gitlab.md`):

| Claim / gate | What the recording shows |
|---|---|
| §G4 milestone → sprint, D6 | a dated, `closed` group milestone; `due_date` `2026-06-14` → `end_at` `2026-06-15T00:00Z` |
| §G4 membership | `issue.milestone.id` = 214 on 32 issues; 20 issues with `milestone: null` sit in the backlog |
| §G4 `include_ancestors` on a project board | `project_milestones.json` (`acme/backend`) lists the **group** milestone |

Still not covered: an undated milestone, a project-level milestone, a
`task`-type issue, and iterations (Premium).
