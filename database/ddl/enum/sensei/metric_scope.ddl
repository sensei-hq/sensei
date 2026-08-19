set search_path to sensei, extensions;

-- Grain a project_metrics row is attributed to. repo = the whole repository (all
-- authors, whole tree); user = the local user's own contribution (their commits ∩
-- the files they touched). Replaces the folder_id-IS-NULL scoping seam — scope is
-- now explicit, so folder_id no longer overloads "project vs module".
create type metric_scope
    as enum ('repo', 'user');
