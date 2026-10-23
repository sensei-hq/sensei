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
- `app/src/lib/api.ts` wrappers: `listTrackers`, `connectTracker`, `syncTracker`,
  `disconnectTracker`

**Daemon contract:** [`spec/daemon/tracker-jira-linkage.md`](../daemon/tracker-jira-linkage.md) R3, R4, R8, R9.

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
- **Disconnect board #N?** names the board; the body says the workspace's other boards stay and Jira is not
  changed.
- Polling runs while any board is `pending`; the status line follows the board connected last.
- New codes: `tracker_not_found` "This board is no longer connected to this workspace."; `level_required`
  "Choose what the board serves." and `invalid_level` "A board serves epics, sprints or tasks." (both
  describe the level field).



Where a user **connects this project to its issue tracker**: Jira today, GitLab
later. One project has one tracker (spec R1), so this is a per-project screen, not
a global setting.

**Eyebrow:** `繋 · INTEGRATIONS`. **Header:** "Issue tracker". **Sub-line:** "Connect
this project's Jira board so sensei can link AI sessions to tickets and report time
and tokens per ticket. Read-only: sensei never changes anything in Jira."

## States

**1 · Not connected.** A tracker picker shows "Jira" selected, and "GitLab, coming
soon" disabled with `aria-disabled="true"`. Below it is the Jira form, with fields in
this order everywhere they appear:

| # | Label (visible `<label>`, not only a placeholder) | Input | Help text |
|---|---|---|---|
| 1 | Board URL | text | "Open your board in Jira and copy the address bar, e.g. `https://acme.atlassian.net/jira/software/projects/AUTH/boards/42`." |
| 2 | Atlassian account email | email | "The email you sign in to Jira with. Atlassian needs it together with the API token." |
| 3 | API token | `type="password"`, never echoed | "Sign in to Atlassian with the same account as the email above, create a token, and paste it here." Link: **Create an API token**, which opens `https://id.atlassian.com/manage-profile/security/api-tokens` in the system browser. |
| — | *Advanced ▸* Site URL + Board id | text + number | "Use instead of Board URL." The UI does **not** parse URLs itself; the daemon's Jira adapter owns that (spec R4), so there is one parser. |

Under the form: "Your token is stored in your system keychain, not in sensei's
database."

**Connect** stays disabled until Board URL (or Site + Board id), email and token are
all non-empty. Enter in any field submits. While the request runs, the button shows
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

- Fields come from `GET …/trackers`: `base_url`, `external_scope`, `board_name`,
  `board_ref`, `status`, `last_success_at`, `counts`.
- The status is always shown as **text**; the dot is decorative. The wording follows
  R9: `connected`; `stale` ("last synced 3 h ago"); `error` (mapped message below);
  `pending` ("first sync running…", also shown after **Replace token** until the new credential syncs).
- **Sync now** calls `POST …/trackers/sync`. A 409 shows "A sync is already running".
- **Replace token** opens the form with **Advanced** Site URL + Board id and email
  pre-filled from the connection (`base_url`, `board_ref`, `account`; the daemon keeps no
  board URL, and the UI never builds one) and the token field empty, then re-connects
  through R3.
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
| `busy` | "A sync is already running." | none |
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

## Wrong gate

- The token shown, logged, persisted or prefilled anywhere in the UI.
- "Not connected" displayed because a read failed; "daemon didn't respond" shown for a daemon error code.
- A connect button enabled without an email (it would always 401 on Jira Cloud).
- UI-side URL parsing (a second parser beside the daemon's).
- GitLab selectable before an adapter is registered.
- Status conveyed by colour alone; a destructive action without a confirming dialog.
- Per-person data (assignee, author) shown.

## Related

- Daemon: [`spec/daemon/tracker-jira-linkage.md`](../daemon/tracker-jira-linkage.md)
- Precedent: `app/src/lib/components/settings/ProvidersSection.svelte` (password input and save; it has no live regions or confirm dialog, so this screen adds both)
- Sidebar registry: `app/src/routes/(project)/project/[id]/project-sidebar-view.ts`
