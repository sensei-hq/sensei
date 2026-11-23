set search_path to sensei, extensions;

create table if not exists assistant_installs (
  catalogue_id             text              primary key references assistant_catalogue(id)
, installed                boolean           not null
, version                  text
, path                     text
, config_path              text
, configured               boolean           not null default false
, configured_at            timestamptz
, configured_version       text
, detected_at              timestamptz       not null
, modified_at              timestamptz       not null default now()
);

comment on table assistant_installs is
'What this machine has installed (#455), one row per catalogue tool, written by
assistants::sync_installs on first run, after configure/remove/upgrade, and when
the person asks from Settings. A tool never detected has no row: readers say
"not detected yet", never "not installed". Replaces sensei.assistants.';
comment on column assistant_installs.catalogue_id
     is 'The catalogue tool this row is about.';
comment on column assistant_installs.installed
     is 'Whether the probe found the tool on this machine at detected_at.';
comment on column assistant_installs.version
     is 'The detected version, when the probe reports one.';
comment on column assistant_installs.path
     is 'Where the tool was found (binary or app bundle), when the probe reports it.';
comment on column assistant_installs.config_path
     is 'The tool''s config file sensei writes its MCP entry into.';
comment on column assistant_installs.configured
     is 'Whether sensei is wired into the tool (plugin installed or MCP entry present).';
comment on column assistant_installs.configured_at
     is 'When sensei last configured itself into it from this machine (configure, or an upgrade
that re-pushed it). Detection never sets it: null when sensei has not configured it here.';
comment on column assistant_installs.configured_version
     is 'The sensei version that made that configuration; null with configured_at.';
comment on column assistant_installs.detected_at
     is 'When the probe last ran for this tool.';
comment on column assistant_installs.modified_at
     is 'When this row last changed.';
