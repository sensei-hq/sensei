set search_path to sensei, extensions;
create table if not exists projects (
  id                       uuid             primary key default gen_random_uuid()
, name                     text             not null
, root_abs_path            text                              -- the project's collection root on disk (spec 2026-08-18): the shallowest member repo's abs_path; a project knows where it lives
, description              text
, client                   text
, maturity                 project_maturity not null default 'discovery'
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

create index if not exists projects_maturity_idx
    on projects(maturity);

create index if not exists projects_tags_idx
    on projects using gin(tags);

create index if not exists projects_dojo_id_idx
    on projects(dojo_id);

comment on table projects is
'Projects — independent grouping entity for 1+ folders.
One project per git/subtree folder by default, but users can merge or split.
- maturity: discovery → active → maintenance → archived
- stack: {languages, frameworks, runtimes, services} — derived from member folders
- icon: {kind, value, bg, fg} — e.g. {kind:"kanji", value:"工", bg:"var(--shu-soft)", fg:"var(--shu)"}
- links: external references [{id, kind, label, url}] — docs, dashboards, issue trackers
- guidelines: project rules [{id, rule, source}]
- preferred_acp: default AI coding tool for this project (claude-code, cursor, etc.)
- backlog: task items [{id, task, added}]
- privacy: per-project privacy settings {logPrompts, logFileContents, redactSecrets, shareWithCloud}
- excluded_globs: file/folder patterns to exclude from indexing ["dist/**", "*.generated.*"]';

comment on column projects.id
     is 'Surrogate primary key (UUID).';
comment on column projects.name
     is 'Project display name — initially derived from parent folder name or git remote.';
comment on column projects.root_abs_path
     is 'The project''s collection root on disk — the shallowest member repo''s abs_path (spec
2026-08-18). Lets a project own a repo-collection boundary (e.g. ~/Work/Alert spanning
many repos) and drive grouping. Single-repo projects: the repo''s abs_path.';
comment on column projects.description
     is 'Optional project description.';
comment on column projects.client
     is 'Optional client or owner name.';
comment on column projects.maturity
     is 'Lifecycle stage: discovery (just scanned), active (in development), maintenance, or archived.';
comment on column projects.goal
     is 'What this project is for — populated from README, sensei.json, or user input.';
comment on column projects.icon
     is 'Display icon: {kind:"kanji"|"emoji"|"custom", value:"工", bg:"color", fg:"color"}.';
comment on column projects.stack
     is 'Detected technology stack: {languages:[], frameworks:[], runtimes:[], services:[]}. Derived from member folders.';
comment on column projects.commands
     is 'JSON object of common commands: {dev, test, build, lint, ...}. Derived from config files.';
comment on column projects.links
     is 'External references: [{id, kind, label, url}]. Kinds: docs, dashboard, issues, runbook, design, wiki.';
comment on column projects.guidelines
     is 'Project rules: [{id, rule, source}]. Source can be "house-style", a session id, or "manual".';
comment on column projects.preferred_acp
     is 'Default AI coding tool for sessions in this project: claude-code, cursor, zed, etc.';
comment on column projects.backlog
     is 'Task backlog items: [{id, task, added}]. Surfaced in project settings.';
comment on column projects.privacy
     is 'Per-project privacy settings: {logPrompts:bool, logFileContents:bool, redactSecrets:bool, shareWithCloud:bool}. Defaults to empty = inherit global config.';
comment on column projects.excluded_globs
     is 'Glob patterns to exclude from indexing for this project: ["dist/**", "*.generated.*"]. Layered on top of folders_to_watch.excluded.';
comment on column projects.tags
     is 'Array of tag strings for quick filtering. Vocabulary controlled by sensei.tags table.';
comment on column projects.dojo_id
     is 'Optional binding to the Dōjō membership this project routes findings to (dojo.memberships.id in the separate Dōjō service DB). Nullable — most projects have no Dōjō. Not a DB-level FK: dojo.* lives in another database (Fork 1). Client-membership bindings take precedence over employer ones.';
comment on column projects.modified_at
     is 'Timestamp of the last modification to this row.';
