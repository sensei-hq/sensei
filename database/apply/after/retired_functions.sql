-- Functions the design no longer has. dbd's reconcile prunes orphaned TABLES but
-- not functions, so a function removed from ddl/ would linger in every database
-- that ever had it — still callable, answering a question nothing asks anymore.
-- Each is dropped here, idempotently; a no-op once gone.
--
-- 2026-10-21 (#307 D5): access lives on repositories, not team membership.
drop function if exists dojo.on_team_of(uuid);
drop function if exists dojo.leads_team_of(uuid);

-- 2026-10-21 (#307 D8): add_dojo_viewer also keeps the invited email.
drop function if exists dojo.add_dojo_viewer(uuid, uuid, uuid);

-- 2026-10-21 (#307 D11): a dōjō is an organization's or a person's; none is made by hand.
drop function if exists dojo.create_dojo(text, text, dojo.membership_kind);

-- 2026-10-21 (#307 D12): a project is a workspace. The renamed functions replace these.
drop function if exists dojo.administers_project(uuid);
drop function if exists dojo.attach_run_to_project(uuid, text, jsonb);
drop function if exists dojo.can_read_project(uuid);
drop function if exists dojo.file_repository_project(uuid, text, text);
drop function if exists dojo.grant_project_viewer_to_team(uuid, uuid);
drop function if exists dojo.grant_project_viewer(uuid, uuid);
drop function if exists dojo.is_project_viewer(uuid);
drop function if exists dojo.project_in_my_dojo_or_none(uuid);
drop function if exists dojo.revoke_project_viewer(uuid, uuid);

-- 2026-10-21 (#307 D30): sensei says workspace. The renamed function replaces this.
drop function if exists sensei.sole_project_of(uuid);

-- 2026-10-22 (#307 R7, D34): a dōjō has no Clients section. The confidentiality
-- ledger's action filter and the incidents policy's owner check go with it.
drop function if exists dojo.is_confidentiality_action(text);
drop function if exists dojo.principal_is_member(uuid, uuid);

-- 2026-10-23 (#308 R5c review): a complete member read also keeps each member's
-- Entra directory, and a sign-in is matched only from it — both functions gained
-- an argument. Left in place, the old reconcile is an ambiguous overload of the
-- new one (its sixth argument has a default).
drop function if exists dojo.reconcile_unit_members(uuid, dojo.forge_provider, text, dojo.auth_method, text[]);
drop function if exists dojo.units_listing_member(dojo.forge_provider, text);
