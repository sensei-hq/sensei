set search_path to sensei, extensions;

-- The workspace ↔ repository junction. THE one place a workspace's membership lives.
--
-- DECISION CHANGED 2026-09-30, superseding the folder-grain junction of D1/D2/D10.
-- Membership used to be `folders.workspace_id`, settable independently on all 13,697
-- folder rows, so nothing stopped a repository's folders disagreeing about their
-- workspace — and one already did: the client-q `documentation` checkout's root sat in
-- workspace `client-q` while its 362 subfolders sat in `documentation`. A junction
-- keyed on the repository makes that unrepresentable rather than merely discouraged.
--
-- M:N IS REAL AND IS NOT A HYPOTHETICAL. A canonical repository is keyed on its
-- REMOTE, so two checkouts of one remote collapse to one row here. Measured
-- 2026-09-30, four repositories serve two workspaces each — `kavach` serves `kavach`
-- and `vite-multi-adapter`, `bridge` serves `bridge` and `sparsh`. A `workspace_id`
-- COLUMN on `repositories` would have forced each to pick one and silently dropped
-- the other, which is why this is a table and not a column.
--
-- A folder resolves its workspace through its repo ANCHOR
-- (`repo_anchor_for` → `folders.repository_id` → here), never by carrying one of
-- its own. For a repository in two workspaces that resolution is deliberately
-- MULTI-VALUED: its nodes really do belong to both, and a consumer that needs one
-- must say which rather than be handed an arbitrary pick.
create table if not exists repositories_in_workspaces (
  workspace_id  uuid        not null references sensei.workspaces(id)     on delete cascade
, repository_id uuid        not null references sensei.repositories(id) on delete cascade
, created_at    timestamptz not null default now()
, primary key (workspace_id, repository_id)
);

create index if not exists repositories_in_workspaces_repository_id_idx
    on repositories_in_workspaces(repository_id);

comment on table repositories_in_workspaces is
'Workspace ↔ repository membership — the ONE place it lives, replacing
`folders.workspace_id` (decision changed 2026-09-30, superseding D1/D2/D10).

WHY A TABLE AND NOT A COLUMN ON `repositories`: a repository is keyed on its
REMOTE, so two checkouts of one remote are one row there. Four repositories
measured 2026-09-30 serve two workspaces each — `kavach` in `kavach` and
`vite-multi-adapter`, `bridge` in `bridge` and `sparsh`. A column would force each
to pick one and drop the other silently.

WHY NOT ON FOLDERS, which is what this replaces: `workspace_id` was settable
independently on all 13,697 folders, so a repository''s folders could disagree —
and the client-q documentation repo already did, root in one workspace and its 362
subfolders in another. Keying on the repository makes the drift unrepresentable
instead of merely unlikely.

RESOLUTION IS MULTI-VALUED BY DESIGN. A folder reaches its workspace through its
repo anchor (`repo_anchor_for` → `folders.repository_id` → here). A repository in
two workspaces yields two rows, because its nodes genuinely belong to both; a
consumer needing exactly one must name which, rather than be handed an arbitrary
pick that looks authoritative.';

comment on column repositories_in_workspaces.workspace_id is
'The workspace this repository belongs to. CASCADE on delete: membership is a
statement about a workspace that exists, so deleting the workspace removes the
claim rather than leaving a row pointing at nothing.

A repository may appear here more than once, under different workspaces — that is
the point of the table. See the table comment for why a column on `repositories`
could not express it.';

comment on column repositories_in_workspaces.created_at is
'When the membership was recorded. NOT when the repository was scanned and not
when the workspace was created — a repository can be scanned long before anyone
assigns it, and a workspace can exist with no repositories at all.

Carried so a membership that appeared without anyone intending it can be told
apart from one that was always there. The 2026-09-30 backfill and a user edit
are otherwise indistinguishable.';

comment on column repositories_in_workspaces.repository_id is
'The CANONICAL repository (keyed on remote), not a checkout. Two checkouts of one remote share this id, which is precisely why a workspace cannot be stored on the repository row itself.';
