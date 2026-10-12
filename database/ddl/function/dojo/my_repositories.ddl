set search_path to dojo, extensions;

-- The caller's rows of dojo.all_my_repositories — the one way a SESSION reads it
-- (#236). The view computes each (repository, member) verdict from facts a plain
-- member may not read (the tenant's billing account, share policy and claim);
-- read as the invoker it would see fewer of them and answer wrong — "not
-- subscribed" to every non-admin. SECURITY DEFINER sees every fact; the
-- principal is pinned to the caller, so it returns their rows and nobody else's.
--
-- This is the structural fix the view's header asks for: the principal filter
-- is no longer a consumer's `.eq('principal_id', …)` that one caller can forget.
create or replace function dojo.my_repositories()
    returns setof dojo.all_my_repositories
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select *
      from dojo.all_my_repositories
     where principal_id = dojo.current_principal_id();
$$;

revoke all on function dojo.my_repositories() from public;
grant execute on function dojo.my_repositories() to authenticated;

comment on function dojo.my_repositories() is
'The signed-in principal''s rows of dojo.all_my_repositories, with verdicts
computed from every fact (billing, share policy, claim) — the session''s only
route to the view (#236).';
