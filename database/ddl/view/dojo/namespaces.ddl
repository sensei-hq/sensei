set search_path to dojo, sensei, extensions;

-- The dōjō's read-only projection of sensei.namespaces — the fields the billing / seat path
-- needs (project identity + visibility + scope). A dojo-schema view so the dōjō API never
-- queries the `sensei` schema directly over PostgREST — letting `sensei` stay OFF the exposed
-- API schema list (smaller attack surface; no rls_disabled advisor noise on the daemon-only
-- sensei.* tables). `security_invoker = on` — read only server-side as service_role (granted
-- below; bypasses RLS); NOT granted to any client role.
create or replace view dojo.namespaces
with (security_invoker = on)
as
select id, name, slug, visibility, scope_key
from sensei.namespaces;

comment on view dojo.namespaces is
'Read-only dojo projection of sensei.namespaces (id, name, slug, visibility, scope_key) for the
billing/seat path, so the dōjō API reaches namespaces without querying the sensei schema over
PostgREST (keeping sensei off the exposed API schemas). security_invoker = on; service_role-only.';

grant select on dojo.namespaces to service_role;
