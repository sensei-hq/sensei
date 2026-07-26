set search_path to sensei, extensions;

-- One recorded run of a rule's checker against a repo (D-CHECKER) — the artifact
-- that turns a `verification = 'checker'` rule from documentation into an
-- enforceable pass/fail signal. The daemon resolves a repo's adopted
-- checker-backed rules, maps each rule's `checker_ref` (a canonical command verb:
-- lint | test | build) to that repo's discovered command (sensei.project_commands),
-- runs it, and appends a row here.
--
-- Denormalised on purpose: `rule_statement` + `checker_ref` are copied in rather
-- than FK'd to rule_pack_rules(id), because a pack re-seed clears + re-inserts its
-- rules (new ids), which would orphan a verdict FK — the statement is the stable
-- human handle. `command` is the exact command line that ran, for reproducibility.
create table if not exists rule_check_runs (
  id             uuid          primary key default gen_random_uuid()
, folder_id      uuid          not null references folders(id) on delete cascade
, rule_statement text          not null
, checker_ref    text          not null      -- the canonical verb the rule names (lint|test|build)
, command        text          not null      -- the resolved command line that ran (or '' when skipped)
, verdict        check_verdict not null
, exit_code      integer                     -- process exit code; null when skipped or spawn failed
, output_tail    text          not null default ''  -- last slice of combined stdout+stderr, for triage
, ran_at         timestamptz   not null default now()
);

create index if not exists rule_check_runs_folder_idx on rule_check_runs(folder_id, ran_at desc);
