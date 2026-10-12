-- RLS for dojo.notification_prefs (#236 step 3b; access matrix "Relay").
--
-- A person's notification opt-ins are theirs alone: read and written as the
-- signed-in person, never another's. The push send (a gate raised by a daemon)
-- reads them as service_role, which bypasses RLS.
--
-- The UPDATE grant includes user_id because the route upserts, and PostgREST's
-- upsert sets every posted column on conflict; the with-check keeps it the
-- caller's own.
alter table dojo.notification_prefs enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists notification_prefs_service_only on dojo.notification_prefs;

grant select on dojo.notification_prefs to authenticated;
grant insert (user_id, events, updated_at) on dojo.notification_prefs to authenticated;
grant update (user_id, events, updated_at) on dojo.notification_prefs to authenticated;

drop policy if exists notification_prefs_own on dojo.notification_prefs;
create policy notification_prefs_own on dojo.notification_prefs
    for all to authenticated
    using (user_id = dojo.current_principal_id())
    with check (user_id = dojo.current_principal_id());
