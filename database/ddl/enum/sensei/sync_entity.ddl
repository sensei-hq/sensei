set search_path to sensei, extensions;

-- What kind of thing a sync_state row tracks.
--
-- Enumerated rather than free text for the same reason task_kind was: a renamed
-- or retired entity would otherwise leave rows nothing ever reads again, and
-- nothing would notice.
create type sync_entity as enum (
    'repository'
  , 'project'
  , 'project_repository'
  , 'repository_metric'
  , 'metric_catalogue'
    -- One whole dōjō sync cycle for one persona, keyed on the persona's KEYCHAIN SLOT
    -- (sensei.personas.session_slot) — never the label, which a sign-in rewrites to
    -- the verified GitHub login.
    -- The other five name a THING that syncs; this names the FETCH that decides
    -- what may sync at all. Without it a failed plan fetch has no schema-legal
    -- (entity, key) to record against and the failure is invisible — the daemon
    -- would simply push nothing, indistinguishably from having nothing to push.
    -- It doubles as the per-persona last-sync watermark (synced_at), which is
    -- why the withdrawn sensei.dojo_personas table was not needed.
  , 'dojo_sync_plan'
    -- One tracked branch of a repository's shared code graph (#225), keyed
    -- `<repo_key>@<branch>`: one checkout's push, or why it was not pushed.
  , 'repository_graph'
    -- The per-person forge mirror (#307 A2d), keyed on the persona's keychain slot:
    -- when the daemon last asked the dōjō to mirror that member's repository roles
    -- from their forge token, and whether it could.
  , 'forge_mirror'
);
