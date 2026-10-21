-- Functions the design no longer has. dbd's reconcile prunes orphaned TABLES but
-- not functions, so a function removed from ddl/ would linger in every database
-- that ever had it — still callable, answering a question nothing asks anymore.
-- Each is dropped here, idempotently; a no-op once gone.
--
-- 2026-10-21 (#307 D5): access lives on repositories, not team membership.
drop function if exists dojo.on_team_of(uuid);
drop function if exists dojo.leads_team_of(uuid);
