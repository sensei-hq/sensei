set search_path to sensei, extensions;

-- Metrics NO consuming dōjō wants, per repository — the daemon's local mirror of
-- the tenants' rulings.
--
-- ## Why the daemon keeps a copy at all
--
-- `dojo_sync` is emphatic that it never remembers an ENTITLEMENT ruling: a cached
-- `may_share` would be a second source of truth for consent, and a revoked seat
-- has to bite on the next cycle. Activation is a different kind of fact, and the
-- asymmetry is the reason a copy is right here: staleness costs one cycle of
-- wasted computation, or one cycle of delay before a re-enabled metric returns.
-- Neither ships data without consent. Meanwhile the metric tasks run on their own
-- schedule and must not each open a dōjō round trip to ask — that would make the
-- cost lever cost something.
--
-- ## Already a UNION
--
-- One row means EVERY tenant that consumes this repository has switched the
-- metric off. The per-tenant detail stays in `dojo.metric_activations`, where the
-- choice is made; the daemon only needs the conjunction, because a repository
-- shared with two dōjōs must keep computing while either still wants the value.
-- Reducing on arrival rather than storing per-tenant rows also means the daemon
-- never holds a list of which tenant wanted what — it does not need to know.
--
-- ## Replaced a JSON blob in sensei.config
--
-- The first version stored `{repo_key: [metric_key]}` under one config key. That
-- worked for the in-process gate and nothing else: a view cannot join it usefully
-- and an enable/disable screen has nothing to read. Per-repository rows are what
-- `sensei.metric_status` reports a reason from.
create table if not exists metric_deactivations (
  -- The repository, by its stable id. `repo_key` is what the dōjō speaks (it is
  -- the identity both planes share), resolved to an id on write so this table
  -- joins like every other repo-scoped table and cascades when a repository goes.
  repository_id uuid        not null references sensei.repositories(id) on delete cascade
  -- The catalogue KEY, not a metric id. `sensei.metrics.id` differs between the
  -- two planes — separate databases loaded from the same staging file — so the
  -- key is the only value that survives the trip. Deliberately NOT a foreign key
  -- for the same reason a dōjō may name a metric this install has not seeded yet;
  -- an unknown key simply matches nothing.
, metric_key    text        not null
  -- When the daemon last learned this. Distinguishes a ruling confirmed this
  -- cycle from one left behind by a sync that has since stopped running.
, observed_at   timestamptz not null default now()
, primary key (repository_id, metric_key)
);

comment on table metric_deactivations is
'Metrics NO consuming dōjō wants, per repository — the daemon-side UNION of
dojo.metric_activations. One row = every tenant sharing this repository switched
this metric off, so the compute is skippable. Absence = wanted, which is the
default and needs no seeding. Rewritten whole on each successful plan pull;
`observed_at` says when. Caching is correct here and wrong for the entitlement
ruling: staleness costs a cycle of compute, never consent.';

comment on column metric_deactivations.metric_key
     is 'Catalogue key, not an id — sensei.metrics.id differs between the daemon and dōjō planes. Intentionally not an FK: a dōjō may name a metric this install has not seeded, and an unknown key should match nothing rather than fail the write.';
