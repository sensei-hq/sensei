-- The seeded catalogue of supported tools and this machine's installs (#455).
--
-- `sensei.assistant_catalogue` is seeded data (import/staging/assistant_catalogue.jsonl
-- through staging.import_assistant_catalogue), and `sensei.assistant_catalogue_words`
-- resolves every spelling the system writes (the id and each alias) to one row.
-- `sensei.assistant_installs` keeps what this machine has installed, one row per
-- catalogue tool, and refuses a tool the catalogue does not know.
--
-- WHAT BREAKS THIS TEST: the seed drifting (a tool's kind, name or alias changed or
-- dropped); the view not unnesting aliases; Claude Desktop offered to a crew; an
-- install row accepted for a tool with no catalogue row; the import losing its
-- modified_at guard.
--
-- Run from database/tests/ (`run.sh`, or psql $SENSEI_DATABASE_URL -f sensei/assistant_catalogue.sql).
-- The seed is loaded here through the real path (JSONL → staging → proc) and the whole
-- file rolls back.

begin;
set local client_min_messages = warning;

create temp table catalogue_seed (line text);
\copy catalogue_seed from '../import/staging/assistant_catalogue.jsonl'
truncate staging.assistant_catalogue;
insert into staging.assistant_catalogue (id, name, kind, aliases, joins_crew, sequence, modified_at)
  select line::jsonb->>'id'
       , line::jsonb->>'name'
       , line::jsonb->>'kind'
       , (select coalesce(array_agg(a), '{}') from jsonb_array_elements_text(coalesce(line::jsonb->'aliases', '[]')) a)
       , (line::jsonb->>'joins_crew')::boolean
       , (line::jsonb->>'sequence')::integer
       , (line::jsonb->>'modified_at')::timestamptz
    from catalogue_seed;
call staging.import_assistant_catalogue();

do $$
declare
  r record;
  n int;
begin
  -- 13 supported tools: 8 assistants, 5 editors.
  select count(*) into n from sensei.assistant_catalogue;
  if n <> 13 then raise exception 'catalogue should hold 13 tools, saw %.', n; end if;
  select count(*) into n from sensei.assistant_catalogue where kind = 'editor';
  if n <> 5 then raise exception 'catalogue should hold 5 editors, saw %.', n; end if;

  -- An alias resolves to its tool.
  select * into r from sensei.assistant_catalogue_words where word = 'claude-code';
  if r.id is distinct from 'claude' or r.name is distinct from 'Claude Code' or r.kind::text is distinct from 'assistant' then
    raise exception 'claude-code should resolve to claude / Claude Code / assistant, saw % / % / %.', r.id, r.name, r.kind;
  end if;
  -- The id resolves to itself.
  select * into r from sensei.assistant_catalogue_words where word = 'cursor';
  if r.id is distinct from 'cursor' or r.name is distinct from 'Cursor' or r.kind::text is distinct from 'editor' then
    raise exception 'cursor should resolve to Cursor / editor, saw % / % / %.', r.id, r.name, r.kind;
  end if;
  -- Every spelling the adapters and lanes write resolves.
  select count(*) into n from sensei.assistant_catalogue_words
   where word in ('copilot', 'copilot-cli', 'copilot_cli', 'gemini', 'gemini-cli', 'zed', 'vscode');
  if n <> 7 then raise exception 'seven spellings should resolve, saw %.', n; end if;
  -- One row per word.
  select count(*) into n from (select word from sensei.assistant_catalogue_words group by word having count(*) > 1) d;
  if n <> 0 then raise exception 'a word resolves to more than one tool (% words).', n; end if;
  -- A word with no row is unknown: the view does not guess it.
  select count(*) into n from sensei.assistant_catalogue_words where word = 'muse';
  if n <> 0 then raise exception 'muse is not in the catalogue, yet it resolved.'; end if;

  -- Claude Desktop is in the catalogue but cannot join a crew.
  select * into r from sensei.assistant_catalogue where id = 'claude-desktop';
  if r.joins_crew is distinct from false then
    raise exception 'claude-desktop must have joins_crew = false, saw %.', r.joins_crew;
  end if;
  select count(*) into n from sensei.assistant_catalogue where joins_crew and id <> 'claude-desktop';
  if n <> 12 then raise exception 'twelve tools join a crew, saw %.', n; end if;
end $$;

-- The import is guarded by modified_at: an older staging row does not overwrite.
truncate staging.assistant_catalogue;
insert into staging.assistant_catalogue (id, name, kind, aliases, joins_crew, sequence, modified_at)
  values ('cursor', 'Old Cursor', 'editor', '{}', true, 1, '2000-01-01');
call staging.import_assistant_catalogue();
do $$
begin
  if (select name from sensei.assistant_catalogue where id = 'cursor') is distinct from 'Cursor' then
    raise exception 'an older staging row overwrote the catalogue.';
  end if;
end $$;

-- An install row references the catalogue: a known tool is stored, an unknown one refused.
insert into sensei.assistant_installs (catalogue_id, installed, config_path, configured, detected_at)
  values ('cursor', true, '/home/x/.cursor/mcp.json', true, now());
do $$
begin
  begin
    insert into sensei.assistant_installs (catalogue_id, installed, detected_at) values ('muse', true, now());
    raise exception 'an install row for muse (no catalogue row) was accepted.';
  exception when foreign_key_violation then
    null;
  end;
end $$;

-- A session carries its host (the editor it ran in), null by default.
do $$
declare
  n int;
begin
  select count(*) into n from information_schema.columns
   where table_schema = 'activity' and table_name = 'sessions' and column_name = 'host' and is_nullable = 'YES';
  if n <> 1 then raise exception 'activity.sessions.host (nullable text) is missing.'; end if;
  -- The retired table is gone.
  if to_regclass('sensei.assistants') is not null then
    raise exception 'sensei.assistants should be dropped (replaced by assistant_installs).';
  end if;
end $$;

rollback;
