# GitLab CE 19.5.3, a group with no milestones

Recorded 2026-11-07 with `scripts/record-gitlab-fixtures.py` from a self-managed
GitLab Community Edition (a private group of 3 projects, 52 issues, 34 merge
requests, no milestones), with a group access token (`read_api`). Scrubbed:
titles, descriptions, names, emails, labels, the host (`gitlab.example.com`)
and the group path (`acme`).

What it settles (spec `docs/spec/daemon/tracker-gitlab.md`):

| Claim / gate | What the recording shows |
|---|---|
| D5: CE's iterations probe | `group_iterations_probe.json`: **404**, so the source is milestones |
| C26: merge requests are not in `/issues` | `group_issues_p1.json`: 52 items, every `issue_type` is `issue` |
| D9: no sprints | `group_milestones.json`, `project_milestones.json`: empty |
| §G1 board resolution | `project_by_group_path.json` 404, then `group.json` 200 |
| §G8 owner | `user.json`: `bot: true` (a group token) → `NotAPerson` |
| §G4 key | `references.full` = `acme/backend#44`; `web_url` is `…/-/work_items/44` on 19.x |

Not covered here: milestone rows (dated, undated, parent-group, project), a
`task`-type issue, and iterations (Premium). Those need another recording.
