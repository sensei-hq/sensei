# 庫 · Project window · Libraries

**Segment:** 04 · The project window
**Route:** `/project/[id]/libraries`
**Source mockup:** [`lib/observatory/libraries.jsx`](../../mockups/Sensei/lib/observatory/libraries.jsx) → project variant
**App file:** `app/src/routes/project/[id]/libraries/+page.svelte`

## Purpose

Project-scoped library list with **one-click wrap** as the primary
action. Same primitive as [[screen/observatory-libraries]] but
filtered to this project's used libraries. Wrap-me candidates
appear at the top with rich context (why sensei thinks this is
worth wrapping) and the wrap action generates a scaffolded module
in the project's own repo.

Kanji is 庫 — *repository*.

## Data invariants

- `GET /api/libraries?project=<id>` returns rows from
  `sensei.project_libraries` joined with `sensei.libraries`.
- `POST /api/projects/{id}/libraries/{library_id}/wrap` triggers
  the wrap scaffold; response contains the generated wrapper
  location and files.
- Wrap target = the primary folder or a user-picked folder within
  the project.

## Signals shown

Same as [[screen/observatory-libraries]] plus:

| Element | Value |
|---|---|
| Wrap-me hero card | top candidate with reason ("used 84× in 14d, no wrapper") |
| Version-conflict warning | when `sensei.project_dependencies` view shows conflicts (see (memory: project_p2_sweep_2026_07)) |
| Wrap-target picker | for multi-repo projects, choose which folder to wrap into |
| Row action | Wrap (primary for candidates) · Docs · Details |

## Done gate

- Wrap action generates a scaffold module in the chosen folder.
- Version conflicts render prominently when present.
- Docs button opens Playground with the query pre-filled to the
  library.
- Multi-repo projects show the wrap-target picker.

## Wrong gate

- **Wrap generates in the wrong folder for multi-repo project.**
  Picker not honored.
- **Version-conflict warning missing for a project with clear
  conflicts.** View query broken.
- **Wrap silently succeeds without generating files.** No-op
  regression.
- Every failure mode inherited from
  [[screen/observatory-libraries]] applies.

## Related

- [[pipeline/libraries]] — wrap detection + docs ingestion
- [[pipeline/traceability]] — drift against wrapper surface
- [[screen/observatory-libraries]] — multi-project peer
- [[screen/project-overview]] — libraries stat
