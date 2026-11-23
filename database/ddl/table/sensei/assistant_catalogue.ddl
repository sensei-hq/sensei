set search_path to sensei, extensions;

create table if not exists assistant_catalogue (
  id                       text              primary key
, name                     text              not null
, kind                     tool_kind         not null
, aliases                  text[]            not null default '{}'
, joins_crew               boolean           not null default true
, launch_template          text
, sequence                 integer
, modified_at              timestamptz       not null default now()
);

comment on table assistant_catalogue is
'The supported tools (#455): every assistant and editor sensei knows, seeded from
import/staging/assistant_catalogue.jsonl by staging.import_assistant_catalogue().
It owns their display names and kinds; supporting a new tool is a new seed row.
Read it through sensei.assistant_catalogue_words to resolve any spelling.';
comment on column assistant_catalogue.id
     is 'The tool''s id, the word the system writes most (claude, cursor, zed …).';
comment on column assistant_catalogue.name
     is 'The name people know it by (Claude Code, Cursor, VS Code).';
comment on column assistant_catalogue.kind
     is 'assistant (runs a model and does the work) or editor (hosts an assistant).';
comment on column assistant_catalogue.aliases
     is 'Other spellings the system writes for this tool (claude-code, copilot_cli, gemini-cli).';
comment on column assistant_catalogue.joins_crew
     is 'Whether a session of it can join a relay crew. False for Claude Desktop, which does not run in a repository.';
comment on column assistant_catalogue.launch_template
     is 'One command string that starts this tool, with a {repo} placeholder for the repository (#513). Null when the invocation is not known — a wrong command is worse than none.';
comment on column assistant_catalogue.sequence
     is 'Display order within its kind.';
comment on column assistant_catalogue.modified_at
     is 'When the seed row last changed; the import only overwrites with a row at least as new.';
