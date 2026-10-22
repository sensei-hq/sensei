set search_path to sensei, extensions;

create table if not exists repository_namespaces (
  repository_id            uuid        not null references sensei.repositories(id) on delete cascade
, namespace_id             uuid        not null references sensei.namespaces(id) on delete cascade
, modified_at              timestamptz not null default now()
, primary key (repository_id, namespace_id)
);

create index if not exists repository_namespaces_namespace_id_idx
    on repository_namespaces(namespace_id);

comment on table repository_namespaces is
'Which namespaces a REPOSITORY belongs to — the set membership governance
resolves rules through.

- **What it is.** One row per (repository, namespace). A repository joins one
  namespace per applicable scope: its organization, and its own namespace on the
  `repository` rung (sensei.repository_namespace) — where the packs of its
  detected stacks are adopted (#318 R6, D33).
- **How to use it.** Resolve a cwd to its folder, lift to
  `folders.repository_id`, then union the rules of every member namespace with
  the always-present `user` and `general` scopes, ordered by scope level. There
  is no tree to walk — multi-membership falls out for free, so a repository can
  belong to two organizations with no special case.
- **What it is NOT.** It is not workspace membership. That is
  `repositories_in_workspaces`, and it is the only place a workspace is recorded. A
  `workspace`-scope namespace must never be bound here: it would be a second
  writer for a fact `repositories_in_workspaces` already owns, which is exactly how
  the predecessor drifted.
- **Who writes it.** The scan, from README frontmatter (organization) and, through
  sensei.adopt_stack_pack, the detected stack (the repository''s own namespace).

Keyed on the REPOSITORY, not on a folder. The predecessor
(`folder_namespaces`) keyed the binding on a folder, and all 447 live rows sat
on repo-ROOT folders — none on a `folder` or `module` kind. The folder grain
was therefore never used as one; it only allowed two checkouts of the same
repository to disagree about which rules governed them, which four repositories
already did.';

comment on column repository_namespaces.repository_id
     is 'FK to sensei.repositories — the repo, by identity rather than by checkout, so every clone resolves the same rules.';
comment on column repository_namespaces.namespace_id
     is 'FK to sensei.namespaces — a scope instance the repository belongs to. Never a `workspace`-scope namespace; see the table comment.';
comment on column repository_namespaces.modified_at
     is 'Timestamp of the last modification to this row.';
