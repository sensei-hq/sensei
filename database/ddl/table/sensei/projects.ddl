set search_path to sensei, extensions;

create type if not exists project_maturity
    as enum ('discovery', 'active', 'maintenance', 'archived');

create table if not exists projects (
  id                       uuid             primary key default gen_random_uuid()
, name                     text             not null
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
, tags                     text[]           not null default '{}'
, modified_at              timestamptz      not null default now()
);

create index if not exists projects_maturity_idx
    on projects(maturity);

create index if not exists projects_tags_idx
    on projects using gin(tags);

comment on table projects is
'Projects — independent grouping entity for 1+ folders.
One project per git/subtree folder by default, but users can merge or split.
- maturity: discovery → active → maintenance → archived
- stack: {languages, frameworks, runtimes, services} — derived from member folders
- icon: {kind, value, bg, fg} — e.g. {kind:"kanji", value:"工", bg:"var(--shu-soft)", fg:"var(--shu)"}
- links: external references [{id, kind, label, url}] — docs, dashboards, issue trackers
- guidelines: project rules [{id, rule, source}]
- preferred_acp: default AI coding tool for this project (claude-code, cursor, etc.)';

comment on column projects.id
     is 'Surrogate primary key (UUID).';
comment on column projects.name
     is 'Project display name — initially derived from parent folder name or git remote.';
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
comment on column projects.tags
     is 'Array of tag strings for quick filtering. Vocabulary controlled by sensei.tags table.';
comment on column projects.modified_at
     is 'Timestamp of the last modification to this row.';
