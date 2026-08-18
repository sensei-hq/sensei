set search_path to sensei, extensions;

-- A user's stance at a scope — the three behavioural dials (autonomy · sharing ·
-- review) that govern HOW a session runs and how its output flows, as opposed to
-- the rules that govern WHAT it may do. User-scoped (D-STANCE-SCOPE): a stance
-- follows the user with or without a dōjō, and is daemon-local (it drives the
-- local session's autonomy), NOT tenant-shared. Set per namespace (a different
-- stance for an employer vs a client vs personal work), with a `namespace_id`
-- NULL row as the user's default; resolution picks the most-specific rung on the
-- same ladder as rules (sensei.scopes.level).
create table if not exists sensei.stances (
  id            uuid                   primary key default gen_random_uuid()
, user_key      text                   not null   -- the git identity (get_user_for_project); user-scoped
, namespace_id  uuid                   references sensei.namespaces(id) on delete cascade
                                                   -- the scope this stance applies at; NULL = the user's default
, autonomy      stance_autonomy        not null default 'ask_on_guarded'
, sharing       stance_sharing         not null default 'patterns'
, review        stance_review          not null default 'one_maintainer'
, updated_at    timestamptz            not null default now()
, constraint stances_user_namespace unique (user_key, namespace_id)
);

-- One default stance per user (namespace_id NULL): the composite unique above is
-- non-restrictive on NULL (NULLs are distinct), so pin the default with a partial
-- index.
create unique index if not exists stances_user_default
    on sensei.stances(user_key) where namespace_id is null;

-- Covering index for the namespace_id FK (nullable) — a namespace delete cascades
-- here; the unique (user_key, namespace_id) leads with user_key, so namespace_id
-- needs its own index.
create index if not exists stances_namespace_idx
    on sensei.stances(namespace_id) where namespace_id is not null;

comment on table sensei.stances is
'A user''s behavioural stance at a scope — autonomy (how far a run goes before
asking), sharing (what surfaces to the dōjō), review (approvers before a rule
adopts). Governs HOW work runs, complementing the rules (WHAT). User-scoped +
daemon-local (D-STANCE-SCOPE); per-namespace with a NULL-namespace default;
most-specific rung wins on the sensei.scopes ladder.';
comment on column sensei.stances.user_key
     is 'The user this stance belongs to — the git identity (user.email, resolved like get_user_for_project). Stance follows the user, not a tenant.';
comment on column sensei.stances.namespace_id
     is 'The scope instance this stance applies at (organization/client/project/stack …); NULL = the user''s default across scopes. Most-specific rung overrides.';
