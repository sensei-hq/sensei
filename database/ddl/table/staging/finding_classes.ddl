set search_path to staging, extensions;

-- Scratch buffer for the finding-classes seed: rebuilt by dbd import from
-- database/import/staging/finding_classes.jsonl, then merged by
-- staging.import_finding_classes(). No guard needed: the merge upserts on
-- code and never touches added_by, so re-imports are idempotent and the
-- learning loop's rows are safe.
drop table if exists staging.finding_classes cascade;

create table staging.finding_classes (
  code  text not null,
  label text not null
);

alter table staging.finding_classes enable row level security;
