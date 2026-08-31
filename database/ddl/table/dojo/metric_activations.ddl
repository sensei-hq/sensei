set search_path to dojo, extensions;

-- Which catalogue metrics a tenant has switched off, PER REPOSITORY.
--
-- The one thing a tenant controls about the catalogue. Disabling a metric both
-- hides it and STOPS ITS COMPUTATION — the daemon skips the work — so this is a
-- cost lever, not a display preference. It is worth real money: the `scope=repo`
-- metrics are produced by a whole-tree `git log` over ALL authors, a separate
-- subprocess from the per-identity `git log --author=` that feeds the local
-- view, and nothing outside the dōjō reads them.
--
-- Absence means enabled. A tenant that has never touched this table gets the
-- whole catalogue, which is the sane default and avoids seeding a row per
-- (tenant × repository × metric) on tenant creation. It also means a metric
-- ADDED to the catalogue later is automatically on everywhere, which seeding
-- would have silently prevented.
--
-- ## Per repository, and per tenant, and therefore a UNION
--
-- The grain is (tenant, repository, metric): a tenant may want churn on one
-- repository and not another. And because a repository can be shared with more
-- than one dōjō, one tenant disabling a metric must NOT stop the others from
-- getting it. So the daemon computes a metric for a repository when ANY
-- consuming tenant still wants it, and skips only when every one of them has
-- switched it off.
create table if not exists metric_activations (
  tenant_id   uuid        not null references dojo.tenants(id) on delete cascade
-- The repository this ruling applies to. A tenant's answer for one repository
-- says nothing about another: churn may be worth paying for on the service you
-- operate and noise on a vendored mirror.
, repository_id uuid      not null references dojo.repositories(id) on delete cascade
-- References sensei.metrics — the ONE metric catalogue, not a dōjō-side copy.
--
-- Product-owned reference data follows the sensei.rule_packs pattern: a single
-- table in the `sensei` schema, deployed to BOTH planes (the daemon through the
-- default scope, Supabase through the `dojo` scope's includes). These are
-- separate databases, so each gets its own rows from the same staging file.
--
-- A dojo.metrics mirror existed briefly. Two tables for one thing diverge, and
-- these two already had: different columns, and text where the catalogue uses
-- enums. Deleted rather than kept in sync by hand.
, metric_id   uuid        not null references sensei.metrics(id) on delete cascade
, enabled     boolean     not null default true
, updated_at  timestamptz not null default now()
, primary key (tenant_id, repository_id, metric_id)
);

comment on table metric_activations is
'Per-(tenant, repository) on/off for catalogue metrics. Absence = enabled, so a
new tenant needs no seeding and a newly catalogued metric is on everywhere.
Disabling skips the computation, not just the display. A repository shared with
several dōjōs is computed while ANY of them still wants the metric — one tenant
opting out never degrades another.';

alter table metric_activations enable row level security;
drop policy if exists metric_activations_service_only on metric_activations;
create policy metric_activations_service_only on metric_activations
    for all to authenticated, anon using (false) with check (false);
