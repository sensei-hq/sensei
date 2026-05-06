set search_path to staging, extensions;

drop table if exists benchmark_reports cascade;
create table benchmark_reports (
  id             uuid
, folder_id      uuid
, run_name       text
, strategy       text
, score          numeric
, tokens         integer
, elapsed_ms     integer
, payload        jsonb
, promoted       boolean
, modified_at    timestamptz
);

create unique index if not exists benchmark_reports_id_ukey
    on benchmark_reports(id);

comment on table benchmark_reports is
'Staging buffer for sensei.benchmark_reports.';
