set search_path to dojo, extensions;

-- Which organization dōjōs' last complete member set lists a person (#308 R5c,
-- D45): each (dōjō, unit) whose connection to p_provider holds p_subject in its
-- dojo.connection_members. A Microsoft sign-in cannot read Azure DevOps, so it is
-- provisioned into exactly these (provisioning.ts, from = p_provider and the unit).
-- p_directory is the sign-in's tenant (the Entra `tid`): only a set that recorded
-- the person in THAT directory names its dōjō (#308 R5c review — defence in depth
-- on top of the object id); a member recorded with no directory is never named.
-- Personal dōjōs and unproven connections are never named, nor a set older than
-- dojo.member_set_max_age_hours() (#308 R5c review: a dead grant or a stopped
-- schedule must not keep joining people removed since). service_role only.
create or replace function dojo.units_listing_member(
    p_provider  dojo.forge_provider,
    p_subject   text,
    p_directory text
)
    returns table (tenant_id uuid, unit text)
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select c.tenant_id, c.external_id
      from dojo.connection_members cm
      join dojo.tenant_connections c on c.id = cm.connection_id
      join dojo.tenants t on t.id = c.tenant_id
     where cm.subject = p_subject
       and cm.directory = p_directory
       and c.provider = p_provider
       and c.external_id is not null
       and t.origin = 'organization'
       and c.member_set_read_at > now() - make_interval(hours => dojo.member_set_max_age_hours())
     order by c.tenant_id, c.external_id;
$$;

revoke all on function dojo.units_listing_member(dojo.forge_provider, text, text) from public, anon, authenticated;
grant execute on function dojo.units_listing_member(dojo.forge_provider, text, text) to service_role;

comment on function dojo.units_listing_member(dojo.forge_provider, text, text) is
'The organization dōjōs (and their forge units) whose connection''s last complete member set lists
p_subject — where a sign-in that cannot read the forge provisions the person (#308 R5c, D45).
service_role only.';
