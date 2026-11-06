set search_path to staging, relay, extensions;

-- Merge the finding-classes seed into relay.finding_classes. Upserts on code:
-- labels follow the seed (it is the curated source), added_by is never
-- touched (the learning loop's signature survives re-imports). The source is
-- deduped on code: staging may hold the seed twice (a re-run without a
-- truncate, the test's own load on top of a seeded table), and one ON CONFLICT
-- row must not hit the same target twice. No freshness gate: ten curated rows
-- need no modified_at plumbing, and added_by must not be overwritten by seed
-- re-runs.
create or replace procedure staging.import_finding_classes()
language plpgsql
as $$
begin
  insert into relay.finding_classes (code, label)
  select distinct on (code) code, label from staging.finding_classes order by code, label
  on conflict (code) do update set label = excluded.label;
end;
$$;

comment on procedure staging.import_finding_classes() is
'Seed merge for relay.finding_classes. Idempotent; never touches added_by.';
