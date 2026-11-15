set search_path to sensei, extensions;

-- What a build or test prints when it fails or passes (spec tracker-sdlc-metrics S3).
-- A marker is owned by the stages of its toolchain and category; it decides an
-- outcome only when exactly one stage owns it. Seeded from
-- database/import/staging/command_markers.jsonl.
create table if not exists command_markers (
  toolchain    text         not null
, category     text         not null check (category in ('build', 'typecheck', 'test', 'e2e'))
, kind         text         not null check (kind in ('fail', 'ok', 'fail_re', 'ok_re'))
, marker       text         not null check (marker <> '')
, modified_at  timestamptz  not null default now()
, primary key (toolchain, category, kind, marker)
);

comment on table command_markers is
'Output markers of builds and tests (spec tracker-sdlc-metrics S3). Seeded from
database/import/staging/command_markers.jsonl by staging.import_command_markers.';

comment on column command_markers.toolchain
     is 'The toolchain that prints it (matches sensei.command_patterns.toolchain).';
comment on column command_markers.category
     is 'What the marker reports on: build, typecheck, test or e2e.';
comment on column command_markers.kind
     is 'fail / ok: a literal substring. fail_re / ok_re: a regex, for markers that need a count (svelte-check found N errors).';
comment on column command_markers.marker
     is 'The substring or regex searched for in the command''s output.';
comment on column command_markers.modified_at
     is 'Import guard: a re-seed only overwrites a row when the datafile is at least as new.';
