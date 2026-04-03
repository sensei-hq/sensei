set search_path to staging;

create or replace procedure import_benchmark_reports()
language plpgsql
as $$
begin
  insert into sensei.benchmark_reports (
      id, repo_id, run_name, strategy, score
    , tokens, elapsed_ms, payload, promoted, created_at
  )
  select
      coalesce(stg.id, gen_random_uuid())
    , stg.repo_id, stg.run_name, stg.strategy, stg.score
    , stg.tokens, stg.elapsed_ms, stg.payload
    , coalesce(stg.promoted, false)
    , coalesce(stg.created_at, now())
  from staging.benchmark_reports stg
  join sensei.repos r on r.id = stg.repo_id   -- skip rows whose repo hasn't been imported yet
  where stg.run_name is not null
  on conflict (id) do nothing;
end;
$$;
