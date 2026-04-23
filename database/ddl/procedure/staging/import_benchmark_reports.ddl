set search_path to staging, extensions;

create or replace procedure import_benchmark_reports()
language plpgsql
as $$
begin
  insert into sensei.benchmark_reports (
      id, folder_id, run_name, strategy, score
    , tokens, elapsed_ms, payload, promoted, created_at
    , modified_at
  )
  select
      coalesce(stg.id, gen_random_uuid())
    , stg.folder_id, stg.run_name, stg.strategy, stg.score
    , stg.tokens, stg.elapsed_ms, stg.payload
    , coalesce(stg.promoted, false)
    , coalesce(stg.created_at, now())
    , coalesce(stg.modified_at, stg.created_at, now())
  from staging.benchmark_reports stg
  join sensei.folders f on f.id = stg.folder_id
  where stg.run_name is not null
  on conflict (id) do nothing;
end;
$$;

comment on procedure import_benchmark_reports is
'Import staging.benchmark_reports into sensei.benchmark_reports.
Joins to sensei.folders to skip rows whose folder has not been imported yet.';
