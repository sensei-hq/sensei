set search_path to staging, sensei, extensions;

create or replace procedure import_assistant_catalogue()
language plpgsql
as $$
begin
  insert into sensei.assistant_catalogue (id, name, kind, aliases, joins_crew, launch_template, sequence, modified_at)
  select
      stg.id
    , stg.name
    , stg.kind::sensei.tool_kind
    , stg.aliases
    , coalesce(stg.joins_crew, true)
    , stg.launch_template
    , stg.sequence
    , stg.modified_at
  from staging.assistant_catalogue stg
  where stg.id is not null
  on conflict (id)
  do update set
      name            = excluded.name
    , kind            = excluded.kind
    , aliases         = excluded.aliases
    , joins_crew      = excluded.joins_crew
    , launch_template = excluded.launch_template
    , sequence        = excluded.sequence
    , modified_at     = excluded.modified_at
  where excluded.modified_at >= sensei.assistant_catalogue.modified_at;
end;
$$;
