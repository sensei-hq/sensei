set search_path to sensei, extensions;

-- Tombstone the OLD keyring entry whenever a tracker stops referencing it: on
-- delete (disconnect, or the workspace cascade) and on a secret swap (re-connect).
-- No code path writes these tombstones by hand, so a concurrent re-connect cannot
-- leak one. Spec: docs/spec/daemon/tracker-jira-linkage.md R3.
create or replace function tracker_secret_tombstone()
returns trigger
language plpgsql
as
$$
begin
  insert into sensei.tracker_secret_tombstones (secret_service, account, not_before)
  values (old.secret_service, old.account, now())
  on conflict (secret_service) do update
     set not_before = least(tracker_secret_tombstones.not_before, excluded.not_before);

  if (tg_op = 'DELETE') then
    return old;
  end if;
  return new;
end;
$$;

drop trigger if exists trackers_tombstone_on_delete on sensei.trackers;
create trigger trackers_tombstone_on_delete
after delete
    on sensei.trackers
   for each row execute function tracker_secret_tombstone();

drop trigger if exists trackers_tombstone_on_secret_swap on sensei.trackers;
create trigger trackers_tombstone_on_secret_swap
after update of secret_service
    on sensei.trackers
   for each row
  when (old.secret_service is distinct from new.secret_service)
       execute function tracker_secret_tombstone();
