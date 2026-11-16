# 繋 · Workspace window · Integrations

**Segment:** 04 · The project window
**Route:** `/workspace/[id]/integrations`
**Source mockup:** none yet. The shape follows `ProvidersSection.svelte` (secret entry)
and the connected-state line of the Dōjō mockup's Integrations tab
(`mockups/Sensei/lib/dojo2/dojo2-project.jsx`).
**App files:**
- `app/src/routes/(workspace)/workspace/[id]/integrations/+page.svelte`
- the sidebar entry `{ id: 'integrations', kanji: '繋', label: 'Integrations' }` in
  `project-sidebar-view.ts` `SECTIONS`, after `sprints` and before `about`
  (`isSectionActive` matches it by the `/integrations` route prefix)
- `app/src/lib/api.ts` wrappers on `senseiApi`: `tryListTrackers`, `tryConnectTracker`,
  `trySyncTracker`, `tryDisconnectTracker`, and `trySetTrackerWriteback` (usage and SDLC write-back,
  [`tracker-metrics-writeback.md`](../daemon/tracker-metrics-writeback.md) W8)

**Daemon contract:** [`spec/daemon/tracker-jira-linkage.md`](../daemon/tracker-jira-linkage.md) R3, R4, R8, R9;
GitLab: [`spec/daemon/tracker-gitlab.md`](../daemon/tracker-gitlab.md) §G1, §G3, §G8, §G10.

**Naming:** "Integrations" means external tools this *project* connects to (issue
trackers today). The Observatory's "Dōjō connections" are a different thing: your
memberships in Dōjō organisations. The two names stay distinct on purpose.

## Several boards, by level (D40, 2026-10-22)

This section overrides "one project has one tracker" below (spec `docs/spec/tracker-levels.md`).

- **Header** "Issue trackers". The screen lists the workspace's boards under a heading per level, in the
  order **Epics**, **Sprints**, **Tasks**; a level without a board has no heading. Each board is a card
  (`data-tracker=<id>`) with its own Sync now, Replace token and Disconnect.
- **Add a board** (`data-action="add"`) opens the form beside the cards; Cancel closes it.
- The form's first field is **"What the board serves"**: three radios (Epics, Sprints, Tasks, each with a
  one-line help). **None is preselected**, and Connect stays disabled until one is chosen. Replace token
  pre-selects the board's own level.
- **Disconnect board #N?** names the board; the body says the workspace's other boards stay and the tracker
  itself is not changed.
- Polling runs while any board is `pending`; the status line follows the board connected last.
- New codes: `tracker_not_found` "This board is no longer connected to this workspace."; `level_required`
  "Choose what the board serves." and `invalid_level` "A board serves epics, sprints or tasks." (both
  describe the level field).



Where a user **connects this project to its issue tracker**: Jira, GitLab or GitHub
(amended 2026-11-07, `tracker-gitlab.md`; 2026-11-09, `tracker-github.md`). One project has one tracker (spec R1), so this is a
per-project screen, not a global setting.

**Eyebrow:** `繋 · INTEGRATIONS`. **Header:** "Issue trackers". **Sub-line** (superseded
2026-10-26 by tracker-metrics-writeback W-UI, 2026-11-07 by `tracker-gitlab.md`,
2026-11-09 by `tracker-github.md`, and 2026-11-14 by `tracker-forge-comment-writeback.md`):
"Connect this workspace's Jira, GitLab or GitHub boards, each at a level, so sensei can
link AI sessions to tickets and report time, tokens and cost per ticket. On tickets
assigned to you, sensei writes these numbers back to the tracker: as a comment, or on
Jira in the fields you map. It never comments on a public or internal GitLab project or
GitHub repository."

## Write-back on each board card (tracker-metrics-writeback W-UI)

Every card whose `writeback.supports.comment` is true has the checkbox and the state line;
only a card whose `writeback.supports.fields` is true (Jira) has **Fields** (amended 2026-11-14, tracker-forge-comment-writeback). A **GitLab**
or **GitHub** card has the checkbox and no **Fields**: the daemon writes the comment for both
and refuses `PUT …/fields` with 409 `fields_unsupported` (`tracker-gitlab.md` §G8,
`tracker-github.md` §H6). The block is gated on `supports`, never on the kind. A card
whose `supports.comment` is false shows an unsupported line (`data-writeback="unsupported"`)
in place of the checkbox; no registered tracker is in that state.

- **Write back to {tracker}** ("Write back to Jira", "Write back to GitLab", "Write back to
  GitHub"): a labelled checkbox (`data-action="writeback"`) that sends
  `PATCH …/trackers/{tracker}` `{writeback_enabled}` (`spec/daemon/tracker-metrics-writeback.md` W8). It is disabled while any request
  runs. The card shows the tracker the daemon returns, so a refused switch stays as it
  was and the alert says why.
- **A state line** (`data-writeback-state=<state>`), from `writeback.state`:
  - `active`: "Writing to N tickets assigned to you." (N = `writeback.assigned −
    writeback.not_private`, so skipped tickets are not counted (amended 2026-11-14, tracker-forge-comment-writeback); "1 ticket"), plus
    " K couldn't be written." when `failing > 0` (`failing` never counts `not_private`)
  - `disabled`: "Write-back is off."
  - `pass_failed`, by `error`, naming the card's tracker ({tracker} is Jira, GitLab or
    GitHub, (amended 2026-11-14, tracker-forge-comment-writeback)): token → "Couldn't read the {tracker} token to write back.
    Replace the token."; `db` → "Couldn't record what was written. Write-back retries
    on each sync."; `rate_limited` → "{tracker} is rate-limiting writes. Write-back resumes
    on the next sync."; `transport` → "Writing didn't finish in time. Write-back resumes
    on the next sync."
  - `owner_not_a_person`: "This token belongs to an app account, so sensei can't tell
    which tickets are yours. Nothing is written."
  - `owner_lookup_failed`: "Couldn't check which tickets are yours: <error text>.
    Retrying on each sync." (the error text in the card's tracker's words)
  - `owner_unknown`: "Checking which tickets are yours on the next sync."
  - `nothing_assigned`: "No tickets on this board are assigned to you."
- **Skipped tickets** (`data-writeback="not-private"`), only when `writeback.not_private > 0`
  (amended 2026-11-14, tracker-forge-comment-writeback): "N tickets are in public or internal repositories; sensei doesn't comment there."
  ("1 ticket is …" for one.)
- **Comment-only note** (`data-writeback="comment-only"`), on a card whose
  `supports.fields` is false (amended 2026-11-14, tracker-forge-comment-writeback):
  - GitLab: "Sensei posts one comment on each ticket assigned to you, in private projects
    only, and keeps it up to date. The token needs the api scope. The first comment
    notifies the ticket's participants; updates don't."
  - GitHub: "Sensei posts one comment on each ticket assigned to you, in private
    repositories only, and keeps it up to date. The token needs Issues: read and write on
    each repository (or a classic repo token). The first comment notifies the ticket's
    participants; updates don't."
- **Fields** (`data-action="fields"`, "Fields (N mapped)"), only when `supports.fields`
  (Jira), opens the board's mapping in a
  `data-section="fields"` editor:
  - one row per mapped field (`data-field-row=<i>`): a metric select and a "Jira field
    id" input. The select offers the 13 usage metrics, then the 21 SDLC
    metrics (`tracker-sdlc-metrics.md` S9), in the daemon's write order (`METRICS`, pinned
    to the `tracker_metric` enum by the state spec)
  - Remove per row, **Add a field**, and **Save fields**, which sends the whole set with
    `PUT …/fields`
  - under the rows: "Each ticket counts its own sessions in full; don't sum these fields
    across tickets."
  - a refused save names its row (`data-field-error`): the W7 code's text from
    `FIELD_MAPPING_ERRORS`; a duplicate marks the second row. A failure that names no
    row is the card alert.

## States

**1 · Not connected.** A tracker picker of three radios, **Jira** (selected), **GitLab**
(amended 2026-11-07: it replaced "GitLab, coming soon") and **GitHub** (amended 2026-11-09). The radios are
disabled while a token is being replaced, so the board's own tracker stays chosen. Below
it is the chosen tracker's form. The Jira form has its fields in this order everywhere
they appear:

| # | Label (visible `<label>`, not only a placeholder) | Input | Help text |
|---|---|---|---|
| 1 | Board URL | text | "Open your board in Jira and copy the address bar, e.g. `https://acme.atlassian.net/jira/software/projects/AUTH/boards/42`." |
| 2 | Atlassian account email | email | "The email you sign in to Jira with. Atlassian needs it together with the API token." |
| 3 | API token | `type="password"`, never echoed | "Sign in to Atlassian with the same account as the email above, create a token, and paste it here." Link: **Create an API token**, which opens `https://id.atlassian.com/manage-profile/security/api-tokens` in the system browser. |
| — | *Advanced ▸* Site URL + Board id | text + number | "Use instead of Board URL." The UI does **not** parse URLs itself; the daemon's Jira adapter owns that (spec R4), so there is one parser. |

Under the form: "Your token is stored in your system keychain, not in sensei's
database."

The **GitLab form** (`tracker-gitlab.md` §G1) has no email and no *Advanced* fields:

| # | Label | Input | Help text |
|---|---|---|---|
| 1 | Group or project URL | text | "Open the group or project in GitLab and copy the address bar, e.g. https://gitlab.example.com/acme/platform. https only, on the standard port." |
| 2 | Confirm the host | text; shown only while the URL's host is not `gitlab.com` | "Type the host from the URL again (e.g. gitlab.example.com). sensei sends your token only to this host." |
| 3 | Access token | `type="password"`, never echoed | "In GitLab, create a personal or group access token with the read_api scope, and paste it here. Write-back also needs the api scope." (amended 2026-11-14, tracker-forge-comment-writeback) No create-token link: the token page is on the user's own host. |

The GitLab form sends `{board_url, api_token}`, plus `confirm_host` when one was typed.
The daemon compares the host; the UI only reads it to decide whether to ask.

The **GitHub form** (`tracker-github.md` §H1, §H2) has no email, no host and no *Advanced*
fields:

| # | Label | Input | Help text |
|---|---|---|---|
| 1 | Project URL | text | "Open the project on GitHub and copy the address bar, e.g. https://github.com/orgs/acme/projects/3." |
| 2 | Access token | `type="password"`, never echoed | "Create a fine-grained, read-only token with Projects: read on the owner and Issues: read on the project's repositories. A classic token with repo and read:project also works, but can write to every repository it reaches. A ticket in a repository the token can't read shows as not found. Write-back also needs Issues: read and write." (amended 2026-11-14, tracker-forge-comment-writeback) Connect itself needs only read access. The `unauthorized` alert links to `https://github.com/settings/personal-access-tokens/new`. |
| 3 | Iteration field (optional) | text | "Only when the project has more than one Iteration field: its name, as the project shows it." |

The GitHub form sends `{board_url, api_token}`, plus `iteration_field` when one was typed.
On 422 `iteration_field_ambiguous` the alert reads "Choose the project's Iteration field:
<names>." and, under field 3, a row of buttons (`data-github-choices`), one per name the
daemon returned; a button fills field 3. **Connect** for GitHub needs the URL and the
token.

**Connect** stays disabled until Board URL (or Site + Board id), email and token are
all non-empty (Jira), or until the URL, the token and, for a host other than
`gitlab.com`, the confirmed host are (GitLab). Enter in any field submits. While the request runs, the button shows
"Checking credentials and board…" and the same text is announced in a
`role="status"` region.

**2 · First sync** (after the 201, in the same `role="status"` region). Connect
returns as soon as the connection is saved (`first_sync.pending`), and the first
sync runs in the daemon:
- While `status = pending`: "Connected · first sync running…". The screen polls
  `GET …/trackers` every 2 s while it is open, and stops polling when it closes.
- When the status reaches `connected`: "Connected · 6 sprints · 142 tickets".
- When it reaches `error` (the connection is kept): "Connected, but the first sync
  failed: <mapped message>", with a **Retry sync** button.

**3 · Connected:** a status card:

```
Jira · acme.atlassian.net · project AUTH · board "AUTH Scrum" (#42)
● connected · synced 2 min ago · 6 sprints · 142 tickets
[Sync now]  [Replace token]  [Disconnect]
```

A GitLab card (amended 2026-11-07; write-back block (amended 2026-11-14, tracker-forge-comment-writeback)):

```
GitLab · gitlab.example.com · acme/platform
Sprints: milestones
● connected · synced 2 min ago · 4 sprints · 87 tickets
☐ Write back to GitLab
Write-back is off.
Sensei posts one comment on each ticket assigned to you, in private projects only, …
[Sync now]  [Replace token]  [Disconnect]
```

- Fields come from `GET …/trackers`: `base_url`, `external_scope`, `board_name`,
  `board_ref`, `status`, `last_success_at`, `counts`; for GitLab also `full_path`,
  `sprint_source` and `sprint_source_changed_at`; for GitHub also `url` and
  `iteration_field` (all from the row's `params`, null for the other kinds).
- A GitHub card (amended 2026-11-09) is titled `GitHub · <owner>/<number> · <board_name>`,
  its sprints line reads "Sprints: the <iteration_field> iteration field" (none when the
  project has no Iteration field), and **Replace token** re-sends `board_url = url` and
  the stored `iteration_field`, so only the token is new.
- GitLab's title is `GitLab · <host> · <full_path>` (`external_scope` when `full_path`
  is absent), followed by ` (<board_name>)` when the name differs from the path.
- **Sprints line** (GitLab, `data-sprint-source`): "Sprints: iterations" or "Sprints:
  milestones", from `sprint_source`.
- **Source-change notice** (GitLab, `role="note"`, `data-notice="sprint-source"`), for 14
  days after `sprint_source_changed_at`: "Sprints now come from iterations. Earlier
  milestone sprints are marked removed." (or the reverse, "…from milestones. Earlier
  iteration sprints…"). See `tracker-gitlab.md` §G3.
- The status is always shown as **text**; the dot is decorative. The wording follows
  R9: `connected`; `stale` ("last synced 3 h ago"); `error` (mapped message below);
  `pending` ("first sync running…", also shown after **Replace token** until the new credential syncs).
- **Sync now** calls `POST …/trackers/sync`. A 409 shows "A sync is already running".
- **Replace token** opens the form with **Advanced** Site URL + Board id and email
  pre-filled from the connection (`base_url`, `board_ref`, `account`; the daemon keeps no
  board URL, and the UI never builds one) and the token field empty, then re-connects
  through R3. For **GitLab** it opens the GitLab form with the URL built from `base_url`
  + `/` + `full_path` (the board's current path), **Confirm the host** = the stored host,
  and the token field empty, so only the token is new.
- **Disconnect** opens a modal dialog:
  - Title: "Disconnect Jira?"
  - Body: "This removes this project's synced sprints, tickets and session links from
    sensei. Jira itself is not changed."
  - Buttons: **Cancel** (focused by default) and **Disconnect** (danger style). Esc
    cancels, focus is trapped while the dialog is open, and on close focus returns to
    the Disconnect button.
  - Confirming calls `DELETE …/trackers`.

## Error messages (daemon code → UI text)

Errors render in a `role="alert"` region next to the form or the card, with
`aria-describedby` on the field they concern (Board URL, email or token). After a
failure, focus moves to the alert.

The words are **per tracker** (amended 2026-11-07): `errorFor` takes the kind of the
tracker the request was about, so a GitLab failure never names Jira. The table gives the
Jira words; GitLab's differ for `invalid_base_url` ("That isn't a GitLab board address.
Use https://<host>/<group> or https://<host>/<group>/<project>: https on the standard
port, no subpath, and not a local name such as .local or .internal."), `unauthorized`
("GitLab rejected the token. Check it has the read_api scope and hasn't expired.", with
no create-token link), `token_required` ("Enter a GitLab access token with the read_api
scope. Write-back also needs the api scope.", (amended 2026-11-14, tracker-forge-comment-writeback)), `board_not_found` ("No group or project <board_ref> on <base_url>."),
`not_found` ("GitLab no longer has this group or project (it may have been deleted or
moved)."), `rate_limited`, `bad_request` and the reach codes (`transport`, `http_error`,
`redirect`, `decode`), which name GitLab.

| Code | Shown as | Recovery offered |
|---|---|---|
| `invalid_base_url` | "That isn't a Jira Cloud board address. It should look like https://<your-site>.atlassian.net/…/boards/<number> (older sites: <your-site>.jira.com)." | focus Board URL |
| `unauthorized` | "Jira rejected the email and token." (concerns the token field) | "Check both, or **create a new token**" (link) |
| `forbidden` | "This account can't read that board." | "Ask the board admin for access, or use another account." |
| `board_not_found` | "No board #<board_id> on <base_url>." (values from the 404 body) | focus Board URL |
| `email_required` / `token_required` | "Enter your Atlassian account email and an API token." (normally unreachable: Connect is disabled until both are filled) | focus the empty field |
| `board_project_mismatch` | "That board belongs to a different Jira project than the one given." | edit Advanced fields |
| `board_already_connected` | "This board is already connected to project <name>." | link to that project's Integrations |
| `not_found` | "Jira no longer has this board (it may have been deleted)." | **Disconnect** or **Replace token** |
| `rate_limited` | "Jira is rate-limiting requests. Try again in a minute." | **Retry sync** |
| `transport` / `http_error` / `redirect` / `decode` | "Couldn't reach Jira (<code>). Check the site address and your connection." | **Retry sync** |
| `token_missing` / `credential_store` | "The saved token can't be read from the system keychain." | the **Replace token** button, named in the message |
| `forbidden_caller` | "The sensei daemon refused this window's request." (the app's origin is allowed, so this signals a misconfigured dev origin) | none |
| `not_connected` | "This project has no tracker connected." Reload to the not-connected form. | none |
| `project_not_found` | "This project no longer exists in sensei." | none |
| `internal` | "The sensei daemon hit a database error." | **Retry** |
| `db` (a sync's `last_error`) | "sensei hit a database error while syncing." | **Retry sync** |
| `unsupported_tracker` | "This tracker type isn't available in this version of sensei." | none |
| `host_not_confirmed` (GitLab) | "Type the GitLab host again, exactly as in the board URL." | focus Board URL |
| `writeback_unsupported` | "Write-back isn't available for this tracker yet." (PATCH on a tracker that cannot write the comment; none today) | none |
| `fields_unsupported` | "Field mapping isn't available for this tracker; sensei writes a comment instead." (PUT `/fields` on GitLab or GitHub, (amended 2026-11-14, tracker-forge-comment-writeback)) | none |
| `busy` | "A sync is already running." | none |
| `invalid_body` (a refused write-back PATCH) | "sensei refused that change." | none |
| Fields editor (`FIELD_MAPPING_ERRORS`, on the row it names; a duplicate names its second row): `unknown_metric` | "Unknown metric." | none |
| `duplicate_mapping` | "Already mapped." | none |
| `invalid_field_ref` | "Use a custom field id like customfield_10050." | none |
| `field_not_found` | "No such field on this Jira site." | none |
| `field_not_number` | "That field isn't a number field." | none |
| `field_in_use` | "That field already has values on this board. Pick an empty field." | none |
| `invalid_body` (a refused Fields save) | "Couldn't save the mapping." | none |
| `busy` (a Fields save while a sync or write-back pass holds the board; on the card) | "The board is syncing or writing back. Save again in a moment." | none; the editor stays open |
| any other code | "Unexpected error from sensei (<code>)." | **Retry** |
| daemon unreachable (network error to `127.0.0.1`) | "The sensei daemon didn't respond." | **Retry**. Never shown as "not connected". |

## Data invariants

- The token is sent **once**, in the connect request body, to the local daemon.
  The UI never stores it (no persisted state, no localStorage) and clears the field
  after the request finishes, whether it succeeded or failed. Nothing reads it back:
  `GET …/trackers` has no token field.
- The screen reads only `GET /api/projects/{id}/trackers` for state. It never treats
  a failed read as "not connected"; a failed read shows the daemon-error state
  (CLAUDE.md "never fabricate data on a failure path").
- Requests carry the app's own Origin, which the daemon's tracker guard allows
  through the reused `ALLOWED_ORIGINS` (`tauri://localhost`, dev
  `http://localhost:5173` and `http://127.0.0.1:5173`).
- The screen shows no ticket/sprint usage; that is the Tickets screen's
  ([`workspace-tickets.md`](workspace-tickets.md)).

## Done gate

1. The sidebar shows **Integrations** (繋) between Impact and About; the route renders, and `isSectionActive` highlights it.
2. With the daemon (fake adapter, or a real tenant):
   - Connecting with only a pasted Board URL (Advanced empty) succeeds; the card shows the daemon-derived site, board name and id.
   - A bad token shows the `unauthorized` text plus the create-token link, and focus lands on the alert.
   - A second project connecting the same board shows `board_already_connected` with that project's name and a link.
   - `board_not_found` shows the id and site from the 404 body.
3. After connect, whether it succeeded or failed, the token input is empty, and the page source, Svelte state snapshot and `localStorage` don't contain the token (Playwright, sentinel `SENTINEL-TOKEN-XYZ`).
4. Disconnect: the dialog opens with Cancel focused and Esc cancels. Confirming returns to the not-connected state, and `GET …/trackers` returns `[]`.
5. Daemon stopped → the daemon-error state with Retry, never the not-connected form.
6. a11y: every input has a programmatic label; the status and alert regions are announced (axe check passes on all three states).
7. Write-back: each card's checkbox reflects `writeback.enabled` and sends one PATCH; the state line has the text above for its state; a refused PATCH leaves the checkbox as it was and shows the alert (component tests).
8. GitLab (component tests, `tracker-gitlab.md` D16): the GitLab radio is enabled; a GitLab tracker with `unauthorized` and its card render no "Jira"; the card shows the "Write back to GitLab" checkbox, the comment-only note and no **Fields** (amended 2026-11-14, tracker-forge-comment-writeback); the source-change notice shows within 14 days of `sprint_source_changed_at`.

## Wrong gate

- The token shown, logged, persisted or prefilled anywhere in the UI.
- "Not connected" displayed because a read failed; "daemon didn't respond" shown for a daemon error code.
- A connect button enabled on the Jira form without an email (it would always 401 on Jira Cloud).
- UI-side URL parsing (a second parser beside the daemon's).
- A GitLab failure, card or form that names Jira; a Fields section on a GitLab or GitHub card (amended 2026-11-14, tracker-forge-comment-writeback).
- The write-back block or Fields gated on the kind's name rather than `writeback.supports`.
- The `active` line counting `not_private` tickets as written, or `not_private` shown as a failure.
- Status conveyed by colour alone; a destructive action without a confirming dialog.
- Per-person data (assignee, author) shown. (`assigned_to_me` and the write-back state are not person data; showing them is allowed.)
- The write-back checkbox flipping on a click the daemon refused.

## Related

- Daemon: [`spec/daemon/tracker-jira-linkage.md`](../daemon/tracker-jira-linkage.md)
- Precedent: `app/src/lib/components/settings/ProvidersSection.svelte` (password input and save; it has no live regions or confirm dialog, so this screen adds both)
- Sidebar registry: `app/src/routes/(project)/project/[id]/project-sidebar-view.ts`
