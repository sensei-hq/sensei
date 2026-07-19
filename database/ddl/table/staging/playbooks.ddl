set search_path to staging, extensions;
drop table if exists playbooks cascade;
create table playbooks (
  name text, title text, when_to_use text, opening_tone text,
  method_ref text, enabled boolean default true, source text default 'builtin',
  modified_at timestamptz not null default now()
);
