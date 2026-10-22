set search_path to dojo, extensions;

-- Append one row to the dōjō's audit trail (dojo.audit_events) from inside a
-- SECURITY DEFINER function, in the same transaction as the change it records —
-- so the change and its record commit together or not at all (#313). Not
-- callable by a signed-in user: only the functions that make the change call it.
create or replace function dojo.append_audit_event(p_tenant uuid, p_actor uuid, p_action text, p_detail jsonb)
    returns void
    language sql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
    insert into dojo.audit_events (tenant_id, actor_id, action, detail)
    values (p_tenant, p_actor, p_action, p_detail);
$$;

revoke all on function dojo.append_audit_event(uuid, uuid, text, jsonb) from public;

comment on function dojo.append_audit_event(uuid, uuid, text, jsonb) is
'Append an audit row in the caller''s transaction (#313). Internal: called by the SECURITY DEFINER functions that change who may view.';
