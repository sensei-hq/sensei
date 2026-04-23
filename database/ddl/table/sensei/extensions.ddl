set search_path to sensei, extensions;

create type if not exists extension_kind
    as enum ('plugin', 'skill', 'command', 'agent', 'hook');

create table if not exists extensions (
  id                       uuid           primary key default gen_random_uuid()
, plugin_id                uuid           references sensei.extensions(id) on delete cascade
, kind                     extension_kind not null
, name                     text           not null unique
, version                  text
, description              text
, content                  text
, props                    jsonb          not null default '{}'
, scope                    text           not null default 'global'
                                          check (scope in ('global', 'project', 'folder'))
, enabled                  boolean        not null default true
, source                   text           not null default 'local'
                                          check (source in ('builtin', 'marketplace', 'local'))
, icons                    jsonb          not null default '{}'
, tags                     text[]         not null default '{}'
, modified_at              timestamptz    not null default now()
, created_at               timestamptz    not null default now()
);

create index if not exists extensions_plugin_id_idx
    on extensions(plugin_id);

create index if not exists extensions_kind_idx
    on extensions(kind);

create index if not exists extensions_tags_idx
    on extensions using gin(tags);

comment on table extensions is
'Skills, commands, agents, hooks, and plugins.
Plugins are containers — other extensions reference them via plugin_id (self-ref).
- content: markdown body — the actual skill/command text, editable and round-trippable to .md files
- props: parsed frontmatter + additional config (triggers, inputs, tool_access, etc.)
- scope: global (all projects), project (specific project), folder (specific repo)
- source: builtin (ships with sensei), marketplace (installed), local (user-created)
- plugin_id: null = standalone extension; set = belongs to a plugin';

comment on column extensions.id
     is 'Surrogate primary key (UUID).';
comment on column extensions.plugin_id
     is 'Self-referencing FK: null = standalone, set = belongs to a plugin extension.';
comment on column extensions.kind
     is 'Extension type: plugin (container), skill, command, agent, or hook.';
comment on column extensions.name
     is 'Unique identifier (e.g. "zero-errors-policy", "sensei:build").';
comment on column extensions.version
     is 'Semantic version string.';
comment on column extensions.description
     is 'Triggering conditions or purpose description (starts with "Use when..." for skills).';
comment on column extensions.content
     is 'Markdown body — the actual skill protocol, command instructions, or agent definition.';
comment on column extensions.props
     is 'Parsed frontmatter + config: {triggers, inputs, tool_access, hook_event, ...}.';
comment on column extensions.scope
     is 'Visibility: global (all projects), project (one project), folder (one repo).';
comment on column extensions.enabled
     is 'Whether this extension is currently active.';
comment on column extensions.source
     is 'Origin: builtin (ships with sensei), marketplace (installed from registry), local (user-created).';
comment on column extensions.icons
     is 'Display icons: {emoji, devicon, kanji, custom}.';
comment on column extensions.tags
     is 'Array of tag strings for filtering. Vocabulary controlled by sensei.tags table.';
comment on column extensions.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column extensions.created_at
     is 'Timestamp when this extension was first registered.';
