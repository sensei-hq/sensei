set search_path to sensei, extensions;
create table if not exists workspaces (
  id                       uuid             primary key default gen_random_uuid()
, name                     text             not null
, root_abs_path            text                              -- the workspace's collection root on disk (spec 2026-08-18): the shallowest member repo's abs_path; a workspace knows where it lives
, description              text
, client                   text
, maturity                 workspace_maturity not null default 'discovery'
, goal                     text
, icon                     jsonb            not null default '{}'
, stack                    jsonb            not null default '{}'
, commands                 jsonb            not null default '{}'
, links                    jsonb            not null default '[]'
, guidelines               jsonb            not null default '[]'
, preferred_acp            text
, backlog                  jsonb            not null default '[]'
, privacy                  jsonb            not null default '{}'
, excluded_globs           jsonb            not null default '[]'
, tags                     text[]           not null default '{}'
, dojo_id                  uuid
, modified_at              timestamptz      not null default now()
);

create index if not exists workspaces_maturity_idx
    on workspaces(maturity);

create index if not exists workspaces_tags_idx
    on workspaces using gin(tags);

create index if not exists workspaces_dojo_id_idx
    on workspaces(dojo_id);

comment on table workspaces is
'Workspaces — independent grouping entity for 1+ folders.
One workspace per git/subtree folder by default, but users can merge or split.
- maturity: discovery → active → maintenance → archived
- stack: {languages, frameworks, runtimes, services} — derived from member folders
- icon: {kind, value, bg, fg} — e.g. {kind:"kanji", value:"工", bg:"var(--shu-soft)", fg:"var(--shu)"}
- links: external references [{id, kind, label, url}] — docs, dashboards, issue trackers
- guidelines: workspace rules [{id, rule, source}]
- preferred_acp: default AI coding tool for this workspace (claude-code, cursor, etc.)
- backlog: task items [{id, task, added}]
- privacy: per-workspace privacy settings {logPrompts, logFileContents, redactSecrets, shareWithCloud}
- excluded_globs: file/folder patterns to exclude from indexing ["dist/**", "*.generated.*"]';

comment on column workspaces.id
     is 'Surrogate primary key (UUID).';
comment on column workspaces.name
     is 'Workspace display name — initially derived from parent folder name or git remote.';
comment on column workspaces.root_abs_path
     is 'The workspace''s collection root on disk — the shallowest member repo''s abs_path (spec
2026-08-18). Lets a workspace own a repo-collection boundary (e.g. ~/Work/client-a spanning
many repos) and drive grouping. Single-repo workspaces: the repo''s abs_path.';
comment on column workspaces.description
     is 'Optional workspace description.';
comment on column workspaces.client
     is 'Optional client or owner name.';
comment on column workspaces.maturity
     is 'Lifecycle stage: discovery (just scanned), active (in development), maintenance, or archived.';
comment on column workspaces.goal
     is 'What this workspace is for — populated from README, sensei.json, or user input.';
comment on column workspaces.icon
     is 'Display icon: {kind:"kanji"|"emoji"|"custom", value:"工", bg:"color", fg:"color"}.';
comment on column workspaces.stack
     is 'Detected technology stack: {languages:[], frameworks:[], runtimes:[], services:[]}. Derived from member folders.';
comment on column workspaces.commands
     is 'JSON object of common commands: {dev, test, build, lint, ...}. Derived from config files.';
comment on column workspaces.links
     is 'External references: [{id, kind, label, url}]. Kinds: docs, dashboard, issues, runbook, design, wiki.';
comment on column workspaces.guidelines
     is 'Workspace rules: [{id, rule, source}]. Source can be "house-style", a session id, or "manual".';
comment on column workspaces.preferred_acp
     is 'Default AI coding tool for sessions in this workspace: claude-code, cursor, zed, etc.';
comment on column workspaces.backlog
     is 'Task backlog items: [{id, task, added}]. Surfaced in workspace settings.';
comment on column workspaces.privacy
     is 'Per-workspace privacy settings: {logPrompts:bool, logFileContents:bool, redactSecrets:bool, shareWithCloud:bool}. Defaults to empty = inherit global config.';
comment on column workspaces.excluded_globs
     is 'Glob patterns to exclude from indexing for this workspace: ["dist/**", "*.generated.*"]. Layered on top of folders_to_watch.excluded.';
comment on column workspaces.tags
     is 'Array of tag strings for quick filtering. Vocabulary controlled by sensei.tags table.';
comment on column workspaces.dojo_id
     is 'Optional binding to the Dōjō membership this workspace routes findings to (dojo.memberships.id in the separate Dōjō service DB). Nullable — most workspaces have no Dōjō. Not a DB-level FK: dojo.* lives in another database (Fork 1). Client-membership bindings take precedence over employer ones.';
comment on column workspaces.modified_at
     is 'Timestamp of the last modification to this row.';
