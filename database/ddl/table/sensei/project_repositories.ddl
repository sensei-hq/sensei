set search_path to sensei, extensions;

-- The project ↔ repository junction. THE one place a project's membership lives.
--
-- DECISION CHANGED 2026-09-30, superseding the folder-grain junction of D1/D2/D10.
-- Membership used to be `folders.project_id`, settable independently on all 13,697
-- folder rows, so nothing stopped a repository's folders disagreeing about their
-- project — and one already did: the swarco `documentation` checkout's root sat in
-- project `swarco` while its 362 subfolders sat in `documentation`. A junction
-- keyed on the repository makes that unrepresentable rather than merely discouraged.
--
-- M:N IS REAL AND IS NOT A HYPOTHETICAL. A canonical repository is keyed on its
-- REMOTE, so two checkouts of one remote collapse to one row here. Measured
-- 2026-09-30, four repositories serve two projects each — `kavach` serves `kavach`
-- and `vite-multi-adapter`, `bridge` serves `bridge` and `sparsh`. A `project_id`
-- COLUMN on `repositories` would have forced each to pick one and silently dropped
-- the other, which is why this is a table and not a column.
--
-- A folder resolves its project through its repo ANCHOR
-- (`repo_anchor_for` → `folders.repository_id` → here), never by carrying one of
-- its own. For a repository in two projects that resolution is deliberately
-- MULTI-VALUED: its nodes really do belong to both, and a consumer that needs one
-- must say which rather than be handed an arbitrary pick.
create table if not exists project_repositories (
  project_id    uuid        not null references sensei.projects(id)     on delete cascade
, repository_id uuid        not null references sensei.repositories(id) on delete cascade
, created_at    timestamptz not null default now()
, primary key (project_id, repository_id)
);

create index if not exists project_repositories_repository_id_idx
    on project_repositories(repository_id);

comment on table project_repositories is
'Project ↔ repository membership — the ONE place it lives, replacing
`folders.project_id` (decision changed 2026-09-30, superseding D1/D2/D10).

WHY A TABLE AND NOT A COLUMN ON `repositories`: a repository is keyed on its
REMOTE, so two checkouts of one remote are one row there. Four repositories
measured 2026-09-30 serve two projects each — `kavach` in `kavach` and
`vite-multi-adapter`, `bridge` in `bridge` and `sparsh`. A column would force each
to pick one and drop the other silently.

WHY NOT ON FOLDERS, which is what this replaces: `project_id` was settable
independently on all 13,697 folders, so a repository''s folders could disagree —
and the swarco documentation repo already did, root in one project and its 362
subfolders in another. Keying on the repository makes the drift unrepresentable
instead of merely unlikely.

RESOLUTION IS MULTI-VALUED BY DESIGN. A folder reaches its project through its
repo anchor (`repo_anchor_for` → `folders.repository_id` → here). A repository in
two projects yields two rows, because its nodes genuinely belong to both; a
consumer needing exactly one must name which, rather than be handed an arbitrary
pick that looks authoritative.';

comment on column project_repositories.repository_id is
'The CANONICAL repository (keyed on remote), not a checkout. Two checkouts of one remote share this id, which is precisely why a project cannot be stored on the repository row itself.';
