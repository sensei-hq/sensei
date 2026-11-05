set search_path to dojo, extensions;

-- Withdraw what the signed-in person's install reported for a board in a tenant
-- (spec docs/spec/daemon/tracker-delivery-metrics.md D3 "Withdrawal", B13;
-- #355): every effort row of theirs from that install (p_source), and the board's
-- kept copy when they reported it — its sprints, tickets and every contributor's
-- effort with it; the next contributor whose mark expires (6 h) restores it.
-- Needs no consent: anyone may remove what is their own. Answers whether
-- anything was removed (the route's 204 or 404).
create or replace function dojo.withdraw_delivery(
    p_tenant    uuid,
    p_board_key text,
    p_source    uuid
)
    returns boolean
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_principal uuid := dojo.current_principal_id();
    v_rows      bigint;
    v_boards    bigint;
begin
    if v_principal is null then
        raise exception 'no signed-in principal' using errcode = 'insufficient_privilege';
    end if;
    if p_board_key is null or p_board_key !~ '^[0-9a-f]{64}$' or p_source is null then
        raise exception 'a withdrawal names a board key and its source install' using errcode = 'invalid_parameter_value';
    end if;

    delete from dojo.delivery_effort e
     using dojo.delivery_boards b
     where e.board_id = b.id
       and b.tenant_id = p_tenant
       and b.board_key = p_board_key
       and e.principal_id = v_principal
       and e.source = p_source;
    get diagnostics v_rows = row_count;

    delete from dojo.delivery_boards b
     where b.tenant_id = p_tenant
       and b.board_key = p_board_key
       and b.reported_by = v_principal;
    get diagnostics v_boards = row_count;

    return v_rows + v_boards > 0;
end
$$;

revoke all on function dojo.withdraw_delivery(uuid, text, uuid) from public;
grant execute on function dojo.withdraw_delivery(uuid, text, uuid) to authenticated;

comment on function dojo.withdraw_delivery(uuid, text, uuid) is
'Withdraw the caller''s install''s effort on a board in a tenant, and the board''s kept copy when the caller reported it (#355, B13). No consent needed: the rows are the caller''s own. Answers whether anything was removed.';
