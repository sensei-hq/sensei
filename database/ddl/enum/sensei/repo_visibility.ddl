set search_path to sensei, extensions;

-- Whether a repository participates in sync at all.
--
-- Private is the default: sync is authentication-gated, but a repo the user never
-- wanted shared should not start syncing merely because they signed in.
create type repo_visibility as enum ('private', 'shared');
