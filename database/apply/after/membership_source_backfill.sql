-- Mark the memberships org provisioning made before #309 recorded a source.
-- Invites sign in as `sso`; provisioning records the forge sign-in method — so
-- every non-sso membership is provisioned. Idempotent: rows already marked, and
-- every invited row, are untouched. Safe to run on every deploy; a no-op where
-- dojo.memberships is absent (the daemon's own database).
do $$
begin
    if to_regclass('dojo.memberships') is not null then
        update dojo.memberships
           set source = 'provisioned'
         where source = 'invited'
           and authenticated_via <> 'sso';
    end if;
end $$;
