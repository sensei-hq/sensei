# 識 · Project window · About

**Segment:** 04 · The project window
**Route:** `/project/[id]/about`
**Source mockup:** [`lib/project/project-lite-panes.jsx`](../../mockups/Sensei/lib/project/project-lite-panes.jsx) → `ProjAboutPane`
**App file:** `app/src/routes/project/[id]/about/+page.svelte`

## Purpose

About is the **editable project identity** — the place to review
and update everything sensei believes about this project:

- Name, client, kanji, icon
- **Vision** (one-line description) — daemon-owned but editable
  here
- Stack (languages, frameworks, runtimes, services)
- Maturity level
- **Multi-repo membership** — which folders belong, their roles
  (ui / backend / docs / …), which is primary
- **Dōjō binding** — which membership routes this project's
  findings (employer / client / community / personal)
- Preferred assistant family default

Kanji is 識 — *knowledge / awareness*.

## Data invariants

- `GET /api/projects/{id}/about` returns the editable document
  including folder membership, dojo binding, stack, vision.
- `PUT /api/projects/{id}/about/{key}` writes single-key
  updates.
- Editing icon triggers a re-run of [[pipeline/project-icon]]
  chain if the source changes; user can also set icon manually.
- Changing dojo binding affects future upstream routing
  ([[pipeline/dojo-lifecycle]]). Past routings are not
  retroactively re-routed.
- **User name** for attribution defaults comes from the profile
  auto-derive rule (see [[screen/preferences]]) — About shows
  the current effective value.

## Signals shown

| Element | Value |
|---|---|
| Header | project kanji + name |
| Editable fields | name, client, vision, kanji (icon picker with inference source noted), stack list |
| Multi-repo folder list | folder rows: kanji · path · role (`ui`/`backend`/`docs`/…) · primary chip · remove |
| Add folder | picks a scanned folder to add to this project (fires `POST /api/projects/{id}/repos`) |
| Split project | opens split flow (see [[pipeline/capture]] locked decision) |
| Dojo binding chip strip | list of memberships with the bound one highlighted |
| Preferred assistant | picker (Claude Code / Zed / …) |

## Done gate

- Editing vision writes immediately and appears in
  [[screen/project-overview]] hero on next load.
- Multi-repo folder add / remove works and triggers a re-scan of
  the added folder if needed.
- Split action separates a folder into its own project preserving
  historical sessions and memories with the correct
  attribution.
- Dojo binding change persists and affects the next upstream
  batch (existing queued items stay bound to the prior dojo).
- Client binding shows the client-precedence rule inline so the
  user understands why client's confidentiality wins.

## Wrong gate

- **Vision edit resets on next page load.** Write path broken.
- **Split loses sessions.** Attribution must be preserved.
- **Dojo binding change retroactively re-routes past items.**
  Only future items should be affected.
- **Client binding doesn't lock the credit to `anonymous`.**
  Credit regression (source-dereference stays always-on regardless).
- **Icon picker doesn't show the inferred source.** User can't
  tell why the current icon was chosen.
- **User name field is blank** despite the profile auto-derive
  running. See [[screen/preferences]] wrong-gate on the same
  bug.

## Related

- [[pipeline/capture]] — multi-repo membership + split endpoint
- [[pipeline/project-icon]] — icon inference source
- [[pipeline/narration-cache]] — proposed vision suggestions (future)
- [[pipeline/dojo-lifecycle]] — binding + attribution
- [[screen/project-overview]] — vision consumer
- [[screen/preferences]] — profile identity + name auto-derive
