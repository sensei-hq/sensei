set search_path to dojo, extensions;

-- A workspace's tracker boards and their kept copy's board facts, for a reader
-- of the workspace (spec tracker-delivery-metrics D4; #352). Nothing for anyone
-- else: dojo.can_read_workspace gates every row.
create or replace function dojo.workspace_delivery_boards(p_workspace uuid)
returns table (
    board_id              uuid,
    board_key             text,
    kind                  text,
    name                  text,
    predictability        numeric,
    predictability_reason text,
    history_at            timestamptz,
    pending_tickets       integer,
    reported_at           timestamptz
)
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
    select b.id, b.board_key, b.kind, b.name, b.predictability, b.predictability_reason,
           b.history_at, b.pending_tickets, b.reported_at
      from dojo.delivery_boards b
     where b.workspace_id = p_workspace
       and dojo.can_read_workspace(p_workspace)
     order by b.name, b.board_key;
$$;

revoke all on function dojo.workspace_delivery_boards(uuid) from public;
grant execute on function dojo.workspace_delivery_boards(uuid) to authenticated;

comment on function dojo.workspace_delivery_boards(uuid) is
'A workspace''s tracker boards with their kept copy''s facts (name, predictability, history, completeness), for a reader of the workspace (dojo.can_read_workspace); nothing for anyone else (#352).';
