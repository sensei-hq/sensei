set search_path to dojo, extensions;

-- The caller's PRINCIPAL id, resolved from their Supabase login.
--
-- `dojo.principals` is the stable identity every dōjō foreign key points at, and
-- `principals.auth_user_id` is a re-pointable POINTER at the login (see that
-- table for why the indirection exists). `auth.uid()` returns the LOGIN id, so
-- anything comparing it directly to a column holding a principal id —
-- `memberships.user_id`, `projects.user_id` — matches nothing.
--
-- That is a silent failure, not a loud one: the Worker connects as
-- `service_role` and bypasses RLS, so a policy can be entirely wrong while every
-- application path keeps working. Only a client-direct read notices, and it
-- notices as an empty list rather than an error. Hence one shared resolver
-- rather than the translation hand-rolled at each site — it was hand-rolled
-- three different ways before this existed (spec dojo-auth-provisioning §VIII.2).
--
-- STABLE            → folded to a single initplan evaluation per statement, not
--                     re-run per row.
-- SECURITY DEFINER  → reads dojo.principals, which `authenticated` has no grant
--                     on; a policy executes as the querying role, so without
--                     this the caller would need read access to the principal
--                     table just to identify themselves.
-- search_path pinned → a SECURITY DEFINER function resolving names through the
--                     caller's search_path can be hijacked by a same-named
--                     object in a schema they control. pg_temp is listed
--                     explicitly so a temp table cannot shadow `principals`.
create or replace function dojo.current_principal_id()
    returns uuid
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select p.id
      from dojo.principals p
     where p.auth_user_id = (select auth.uid());
$$;

-- RLS policies call this as the querying role, so `authenticated` MUST have
-- EXECUTE or those policies cannot evaluate. Revoke the public default first.
-- NOTE (advisor "SECURITY DEFINER RPC callable by authenticated"): INTENTIONAL
-- and safe — a direct RPC call returns only the CALLER's own principal id,
-- derived from their own JWT. It takes no argument, so it cannot be pointed at
-- anyone else. Keep.
revoke all on function dojo.current_principal_id() from public;
grant execute on function dojo.current_principal_id() to authenticated;

comment on function dojo.current_principal_id() is
'The caller''s dojo.principals id, resolved from auth.uid(). The single place the
login→principal translation lives: memberships.user_id and projects.user_id hold
principal ids, so comparing either to auth.uid() directly silently matches no
rows (and the service_role app path never notices).';
