set search_path to dojo, extensions;

-- An organisation's DEFAULT for its own private repositories.
--
-- Needed alongside per-repository elections, not instead of them: a per-repo row
-- alone leaves a NEWLY CREATED repository un-elected until somebody touches it,
-- so an org's mandate would silently fail to cover new work — the exact thing a
-- mandate exists to prevent.
create table if not exists tenant_share_policy (
  tenant_id            uuid        primary key references dojo.tenants(id) on delete cascade
  -- FALSE deliberately: an organisation that has not decided has not mandated.
  -- The absence of a policy is not consent.
, private_repos_shared boolean     not null default false
, set_by               uuid        references dojo.principals(id) on delete set null
, set_at               timestamptz not null default now()
, created_at           timestamptz not null default now()
, modified_at          timestamptz not null default now()
);

comment on table tenant_share_policy is
'Whether an organisation mandates sharing for its own PRIVATE repositories.
Applies only where authority is `organization`; a public repo is the member''s
call whoever owns the tenant.

Default false — not deciding is not consenting. A per-repository election in
dojo.repository_elections overrides this either way, which is how "share all
private repos except this one" is expressed.';

comment on column tenant_share_policy.private_repos_shared
     is 'The org''s mandate for its private repos. Members cannot override it in either direction.';
comment on column tenant_share_policy.set_by
     is 'The admin who set it. Nullable so removing a member does not erase the policy they set.';
