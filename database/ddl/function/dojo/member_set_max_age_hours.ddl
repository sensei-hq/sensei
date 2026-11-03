set search_path to dojo, extensions;

-- How old a connection's member set may be before a sign-in stops provisioning
-- from it (#308 R5c review): 48 hours, eight runs of the scheduled member read
-- (every 6 hours, dojo/wrangler.jsonc). A set older than this outlived a grant
-- that stopped working or a schedule that stopped running, and would keep joining
-- people the organization has since removed. The ONE place the age is set: the
-- sign-in lookup (dojo.units_listing_member) and the Integrations screen's
-- "stale" (azure-import.ts) both read it.
create or replace function dojo.member_set_max_age_hours()
    returns integer
    language sql
    set search_path = dojo, pg_temp
    immutable
as $$
    select 48;
$$;

revoke all on function dojo.member_set_max_age_hours() from public, anon, authenticated;
grant execute on function dojo.member_set_max_age_hours() to service_role;

comment on function dojo.member_set_max_age_hours() is
'Hours a connection''s last complete member set stays fresh enough to provision a sign-in from (48:
eight 6-hourly scheduled reads). #308 R5c. service_role only.';
