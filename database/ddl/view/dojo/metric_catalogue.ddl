set search_path to dojo, sensei, extensions;

-- The metric catalogue, readable from the dōjō's API.
--
-- WHY A VIEW RATHER THAN READING sensei.metrics DIRECTLY. The `sensei` schema is
-- deliberately NOT exposed to PostgREST (`supabase/config.toml`): its daemon
-- tables have RLS disabled, so exposing the schema would put them on the API and
-- the attack surface. The sanctioned route is a `dojo` view that qualifies
-- `sensei.*` internally — the same pattern as `dojo.rule_pack_library` and
-- `dojo.namespaces`.
--
-- This exists because the metric push needed it. `metrics-ingest.ts` resolves a
-- metric KEY to its id, and both wrong answers were tried live first: a bare
-- `.from('metrics')` gave `Could not find the table 'dojo.metrics' in the schema
-- cache`, and `.schema('sensei').from('metrics')` gave `Invalid schema: sensei` —
-- the config refusing exactly what it says it refuses.
--
-- Only the two columns a lookup needs. The catalogue carries definitions,
-- families, thresholds and units that the ingest has no business reading, and a
-- view that selects * would put all of it on the API the moment a column is
-- added.
create or replace view dojo.metric_catalogue
with (security_invoker = on)
as
select m.id
     , m.key
  from sensei.metrics m;

comment on view dojo.metric_catalogue is
'Metric key → id, for resolving a pushed metric to its catalogue row.

The sanctioned cross-schema read: sensei is not exposed to PostgREST (its daemon
tables have RLS disabled), so dojo views qualify sensei.* internally. Two columns
only — the ingest resolves a key and has no business reading definitions or
thresholds.';

-- Readable by any signed-in caller: the catalogue is public reference data — the
-- names of the metrics sensei computes — and a push cannot be validated without
-- it. `security_invoker = on` keeps the reader''s own privileges in force rather
-- than the view owner''s.
grant select on dojo.metric_catalogue to authenticated;
