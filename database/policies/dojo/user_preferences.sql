-- RLS for dojo.user_preferences (#133): a person's display preferences are
-- theirs alone — read and written as the signed-in person, never another's.
--
-- The UPDATE grant includes user_id because the route upserts, and PostgREST's
-- upsert sets every posted column on conflict; the with-check keeps it the
-- caller's own.
alter table dojo.user_preferences enable row level security;

grant select on dojo.user_preferences to authenticated;
grant insert (user_id, glyphs, glyphs_set_at, updated_at) on dojo.user_preferences to authenticated;
grant update (user_id, glyphs, glyphs_set_at, updated_at) on dojo.user_preferences to authenticated;

drop policy if exists user_preferences_own on dojo.user_preferences;
create policy user_preferences_own on dojo.user_preferences
    for all to authenticated
    using (user_id = dojo.current_principal_id())
    with check (user_id = dojo.current_principal_id());
