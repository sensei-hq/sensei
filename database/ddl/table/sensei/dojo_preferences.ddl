set search_path to sensei, extensions;

create table if not exists dojo_preferences (
  scope       text        not null default 'user'
, capability  text        not null
, preferred   text        not null
, note        text
, updated_at  timestamptz not null default now()
, primary key (scope, capability)
);

comment on table dojo_preferences is
'Capability → preferred-tool bias for the command surface (the get_commands MCP
tool + the project-window action buttons). When a project exposes several
commands of the same canonical category — e.g. two `test` commands, `cargo test`
and `cargo nextest` — the preference marks/ranks the one to prefer. `scope` is
`user` today (the local developer''s own choice); when a Dōjō exists it will also
carry org / team / dojo scopes that override the user default (the same
specificity ladder as governance). One row per (scope, capability).';

comment on column dojo_preferences.scope
     is 'Preference scope: ''user'' now; later a dojo / org / namespace key. PK part.';
comment on column dojo_preferences.capability
     is 'Canonical command verb the preference applies to (test / build / lint / run / format / typecheck / e2e / bench / docs / start / dev). PK part.';
comment on column dojo_preferences.preferred
     is 'Match token for the preferred command — matched against a command''s raw_name (exact) or command_line (substring), case-insensitively. E.g. ''nextest''.';
comment on column dojo_preferences.note
     is 'Optional human note on why this preference exists.';
comment on column dojo_preferences.updated_at
     is 'When the preference was last set.';
