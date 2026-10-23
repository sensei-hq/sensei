-- dojo.invite_accept_failures became dojo.request_attempts (#308 R5b): one
-- rate-limit table for every limited action, not one per action. Its rows were
-- failed invite accepts counted over 15 minutes — nothing worth carrying across
-- (pre-v1: dropping dōjō data is approved), so the old table is dropped and the
-- reconcile creates the new one.
--
-- WHY A HOOK. The release reconciles without --prune, so a table that leaves the
-- design would sit orphaned. No-op once it is gone, and where `dojo` is absent.
do $$
begin
    if to_regclass('dojo.invite_accept_failures') is not null then
        drop table dojo.invite_accept_failures;
    end if;
end $$;
