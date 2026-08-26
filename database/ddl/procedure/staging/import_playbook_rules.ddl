set search_path to staging, extensions;
create or replace procedure import_playbook_rules()
language plpgsql as $$
begin
  insert into sensei.playbook_rules
     (name, match_lifecycle, match_intent, match_risk, playbook, rationale, priority, base_priority, enabled, source, modified_at)
  select name,
         nullif(match_lifecycle,'')::sensei.chunk_lifecycle,
         nullif(match_intent,'')::sensei.chunk_intent,
         nullif(match_risk,'')::sensei.chunk_risk,
         playbook, rationale, priority, priority, coalesce(enabled,true), coalesce(source,'builtin')::sensei.source_kind,
         coalesce(modified_at, now())
    from staging.playbook_rules stg
  on conflict (name) do update
     set match_lifecycle = excluded.match_lifecycle
       , match_intent    = excluded.match_intent
       , match_risk      = excluded.match_risk
       , playbook        = excluded.playbook
       , rationale       = excluded.rationale
       , priority        = excluded.priority
       , base_priority   = excluded.base_priority
       , enabled         = excluded.enabled
       , modified_at     = excluded.modified_at
   -- Was insert-only, so a datafile edit never reached an existing rule.
   where sensei.playbook_rules.source = 'builtin'
     and excluded.modified_at >= sensei.playbook_rules.modified_at;
end;
$$;
