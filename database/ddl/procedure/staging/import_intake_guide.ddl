set search_path to staging, extensions;
create or replace procedure import_intake_guide()
language plpgsql as $$
begin
  insert into sensei.intake_guide (kind, axis, prompt, help, enabled, source)
  select kind, nullif(axis,''), prompt, help, coalesce(enabled,true), coalesce(source,'builtin')
    from staging.intake_guide stg
  where not exists (
     select 1 from sensei.intake_guide g
      where g.kind = stg.kind and coalesce(g.axis,'') = coalesce(nullif(stg.axis,''),'')
        and g.source = 'builtin');
end;
$$;
