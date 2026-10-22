-- Retire the ladder's `team` and `client` rungs (#318 R6, D25/D32, user
-- 2026-10-21/22: the ladder is dōjō → workspace → repository; teams are
-- reporting-only groupings, a client engagement is a workspace classification).
--
-- The seed no longer has them, but the seed import only upserts — it never
-- removes a rung — so an upgraded database keeps both rows until this drops
-- them. Both held no namespace on the user's machine or in production
-- (measured 2026-10-22). A namespace on either is a rule, stance, seat or
-- binding nobody decided where to move: REFUSE, naming each, and let the user
-- decide. Never delete one.
--
-- Runs on any plane: `sensei.scopes` and `sensei.namespaces` are deployed to the
-- dōjō as well. A no-op once the rungs are gone, and on a fresh database.
-- The technology rung is converted, not refused: see
-- apply/after/sensei_stack_packs_converted.sql.
do $$
declare
    occupied text;
begin
    -- Nested, not `and`-ed: an IF condition is planned whole, so a read of
    -- sensei.namespaces in it fails on a fresh database before the guard is tested.
    if to_regclass('sensei.scopes') is null or to_regclass('sensei.namespaces') is null then
        return;
    end if;
    select string_agg(scope_key || ':' || slug, ', ' order by scope_key, slug) into occupied
      from sensei.namespaces
     where scope_key in ('team', 'client');
    if occupied is not null then
        raise exception 'the team and client rungs are retired (#318 R6, D25/D32), but namespaces still sit on them: %', occupied
            using hint = 'Move each one''s rules to the dōjō (organization) or a workspace, then delete the namespace and deploy again.';
    end if;
    delete from sensei.scopes where key in ('team', 'client');
end $$;
