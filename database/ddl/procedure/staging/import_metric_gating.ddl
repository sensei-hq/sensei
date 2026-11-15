set search_path to staging, sensei, extensions;

-- Seed sensei.metric_gating from staging.metric_gating (dbd calls this on import).
-- Fails loudly rather than dropping a row: a row with no key (a misspelled field
-- loads as null) or no edit time, a key no catalogue row names, or a neutral
-- metric marked eligible, raises naming the key, so a target row never
-- goes missing in silence (a gating row for a metric L2 has not landed yet stays
-- out of the file until it lands). Incremental, as import_metrics: an existing row
-- is overwritten only by a staging row at least as new.
create or replace procedure import_metric_gating()
language plpgsql
set search_path = staging, sensei, extensions
as $$
declare
  keyless int;
  undated text;
  unknown text;
  neutral text;
begin
  select count(*) into keyless from staging.metric_gating where metric_key is null or btrim(metric_key) = '';
  if keyless > 0 then
    raise exception 'metric_gating has % row(s) with no metric_key (is the field misspelled?)', keyless;
  end if;

  -- The guard below orders edits by modified_at; a row without one is refused,
  -- never stamped with this import's clock.
  select string_agg(stg.metric_key, ', ' order by stg.metric_key) into undated
    from staging.metric_gating stg where stg.modified_at is null;
  if undated is not null then
    raise exception 'metric_gating rows with no modified_at: %', undated;
  end if;

  select string_agg(stg.metric_key, ', ' order by stg.metric_key) into unknown
    from staging.metric_gating stg
   where stg.metric_key is not null
     and not exists (select 1 from sensei.metrics m where m.key = stg.metric_key);
  if unknown is not null then
    raise exception 'metric_gating names metrics the catalogue does not have: %', unknown;
  end if;

  select string_agg(stg.metric_key, ', ' order by stg.metric_key) into neutral
    from staging.metric_gating stg
    join sensei.metrics m on m.key = stg.metric_key
   where stg.eligible and m.direction = 'neutral';
  if neutral is not null then
    raise exception 'metric_gating marks neutral metrics eligible (no direction to compare against): %', neutral;
  end if;

  insert into sensei.metric_gating (metric_id, eligible, delta_scoped, min_coverage, note, modified_at)
  select m.id, stg.eligible, stg.delta_scoped, stg.min_coverage, stg.note, stg.modified_at
    from staging.metric_gating stg
    join sensei.metrics m on m.key = stg.metric_key
  on conflict (metric_id)
  do update set
      eligible     = excluded.eligible
    , delta_scoped = excluded.delta_scoped
    , min_coverage = excluded.min_coverage
    , note         = excluded.note
    , modified_at  = excluded.modified_at
  where excluded.modified_at >= sensei.metric_gating.modified_at;
end;
$$;
