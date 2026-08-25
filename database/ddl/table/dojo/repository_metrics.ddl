set search_path to dojo, extensions;

-- The shared metric values. Mirrors the daemon's sensei.repository_metrics.
--
-- Note what is NOT here: no sessions, no turns, no events. A `scope='user'` row
-- is an aggregate the daemon computed locally and pushed as a number, so dōjō can
-- show per-person metrics without ever holding a session, a prompt or a tool
-- call. That is the whole privacy shape of the sync.
create table if not exists repository_metrics (
  id            uuid        primary key default gen_random_uuid()
, tenant_id     uuid        not null references dojo.tenants(id)       on delete cascade
, repository_id uuid        not null references dojo.repositories(id)  on delete cascade
, metric_id     uuid        not null references dojo.metrics(id)       on delete cascade
  -- repo = the whole repository, all authors. user = one principal's own work.
, scope         text        not null check (scope in ('repo', 'user'))
  -- WHO a scope='user' row belongs to. A principal, never a git email: a commit
  -- trailer is an unverified assertion — anyone can set user.email to a
  -- colleague's address — so attributing shared numbers by it would be an
  -- attribution attack. The email may travel in props; it must not be the key.
, principal_id  uuid        references dojo.principals(id) on delete set null
, commit_sha    text
, computed_on   date        not null
, grain         text        not null default 'daily'
, value         numeric     not null
, props         jsonb       not null default '{}'
, source        text        not null default 'measured'
, pushed_at     timestamptz not null default now()
, unique (metric_id, repository_id, scope, principal_id, commit_sha, computed_on, grain)
);

create index if not exists repository_metrics_lookup
    on repository_metrics(repository_id, metric_id, computed_on);

comment on table repository_metrics is
'Shared metric values, pushed by the daemon. Aggregates only — no session, turn
or event ever crosses, so per-user metrics are visible here without the raw
activity behind them being shared at all.';

comment on column repository_metrics.principal_id
     is 'The person a scope=user row belongs to. A principal, NOT a git email: commit trailers are unverified and would let anyone attribute work to a colleague.';

alter table repository_metrics enable row level security;

-- The authorization path, as a policy rather than as endpoint code:
--   principal → team_members → team → team_projects → project
--             → repositories_in_projects → repository
--
-- Being a member of the tenant is deliberately NOT enough. Access is granted at
-- the TEAM, so an org member outside the relevant team sees nothing — which is
-- the thing teams exist to enforce.
drop policy if exists repository_metrics_team_read on repository_metrics;
create policy repository_metrics_team_read on repository_metrics
  for select to authenticated
  using (
    dojo.can_read_repository_metric(
      repository_id, scope, principal_id, tenant_id)
  );

-- Writes are service_role only: a push is governed (per-repo authorization,
-- attribution rules), and that governance cannot be expressed as a row policy.
drop policy if exists repository_metrics_no_client_write on repository_metrics;
create policy repository_metrics_no_client_write on repository_metrics
    for all to anon using (false) with check (false);

-- Table-level SELECT grant for the `authenticated` read path. RLS filters WHICH
-- rows; the grant is what lets the role touch the table at all — without it the
-- policy above is dead code and every read fails with "permission denied"
-- rather than returning an authorised subset. Same pairing as dojo.projects and
-- dojo.relay_inbox.
grant select on dojo.repository_metrics to authenticated;

