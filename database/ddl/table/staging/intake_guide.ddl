set search_path to staging, extensions;
drop table if exists intake_guide cascade;
create table intake_guide (
  kind text, axis text, prompt text, help text,
  enabled boolean default true, source text default 'builtin',
  modified_at timestamptz not null default now()
);
