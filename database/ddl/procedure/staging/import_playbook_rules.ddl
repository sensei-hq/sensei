set search_path to staging, extensions;
create or replace procedure import_playbook_rules()
language plpgsql as $$
begin
  insert into sensei.playbook_rules
     (name, match_lifecycle, match_intent, match_risk, playbook, rationale, priority, base_priority, enabled, source)
  select name,
         nullif(match_lifecycle,'')::sensei.chunk_lifecycle,
         nullif(match_intent,'')::sensei.chunk_intent,
         nullif(match_risk,'')::sensei.chunk_risk,
         playbook, rationale, priority, priority, coalesce(enabled,true), coalesce(source,'builtin')
    from staging.playbook_rules stg
  where not exists (select 1 from sensei.playbook_rules r where r.name = stg.name and r.source='builtin');
end;
$$;
