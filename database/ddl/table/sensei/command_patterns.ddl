set search_path to sensei, extensions;

-- Which shell commands are builds and tests (spec tracker-sdlc-metrics S3).
-- Seeded from database/import/staging/command_patterns.jsonl; adding a toolchain
-- is a datafile row, never a code change (D8). The classifier reads every row and
-- tries them by priority, then by pattern length, both descending.
create table if not exists command_patterns (
  toolchain    text         not null
, pattern      text         not null
, category     text         check (category in ('build', 'typecheck', 'test', 'e2e', 'none'))
, wrapper      boolean      not null default false
, compiles     boolean      not null default false
, owns         text[]
, priority     integer      not null default 0
, modified_at  timestamptz  not null default now()
, primary key (toolchain, pattern)
);

comment on table command_patterns is
'Build/test command catalogue (spec tracker-sdlc-metrics S3). Seeded from
database/import/staging/command_patterns.jsonl by staging.import_command_patterns.';

comment on column command_patterns.toolchain
     is 'What runs the command (cargo, vitest, make, ...); the key its output markers are owned by (sensei.command_markers).';
comment on column command_patterns.pattern
     is 'Regex anchored at the start of a stage, after prefix stripping. Its first capture group, when present, is the script or target name a NULL category is derived from.';
comment on column command_patterns.category
     is 'build, typecheck, test, e2e, or none (a known command that is not a run). NULL: categorise() of the captured name, whole (or, for a wrapper, the repository''s folder-command category).';
comment on column command_patterns.wrapper
     is 'The stage runs a named script or target whose toolchain is unknown (bun run <name>, make <target>), so it may claim the output markers of the toolchains in owns, in its category.';
comment on column command_patterns.compiles
     is 'The stage compiles before it runs (cargo test, cargo clippy), so it also owns its toolchain''s build-fail markers.';
comment on column command_patterns.owns
     is 'For a wrapper: the toolchains whose output markers it may claim (an npm script runs vitest, never cargo). NULL: any (make, just).';
comment on column command_patterns.priority
     is 'Higher is tried first, so a specific row (make test-template -> none) beats a general one (make <target>).';
comment on column command_patterns.modified_at
     is 'Import guard: a re-seed only overwrites a row when the datafile is at least as new.';
