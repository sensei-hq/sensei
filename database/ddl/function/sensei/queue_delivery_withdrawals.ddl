set search_path to sensei, extensions;

-- A tracker leaving — deleted by itself, or with its workspace (pruned, or
-- removed) — queues a withdrawal for every board push it made, before the cascade
-- takes its marks (spec tracker-delivery-metrics B13; #355). A trigger, because
-- trackers leave on several paths and every one must withdraw what was shared.
create or replace function queue_delivery_withdrawals()
returns trigger
language plpgsql
as
$$
begin
  insert into sensei.delivery_withdrawals (board_key, tenant_id, persona_id)
  select distinct m.board_key, m.tenant_id, m.persona_id
    from sensei.tracker_push_marks m
   where m.tracker_id = old.id
  on conflict (board_key, tenant_id, persona_id)
  do update set queued_at = now(), refused_at = null, error = null;
  return old;
end;
$$;

drop trigger if exists trackers_queue_delivery_withdrawals on sensei.trackers;
create trigger trackers_queue_delivery_withdrawals
before delete
    on sensei.trackers
   for each row
       execute function queue_delivery_withdrawals();
