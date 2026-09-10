set search_path to sensei, extensions;

-- D11. A manifest-declared dependency between two FOLDERS.
--
-- REGRAINED from `project_dependencies`, which keyed on
-- (from_project_id, to_project_id, from_folder_id, source_manifest). A manifest
-- sits AT a folder, so the observed fact is folder->folder; the project ids were
-- pre-aggregation baked into the primary key and are derivable via
-- folders.project_id. Project- and repository-level answers are VIEWS over this,
-- never stored aggregates (S10) — a stored aggregate can disagree with the
-- manifest it came from.
--
-- The regrain is a BUG FIX, not just a rename. The old writer skipped an edge
-- whenever both folders belonged to one project (`to_pid != pid`), which
-- discards exactly the common case: a monorepo's crates depending on each other.
-- Measured on this repo — all 384 folders share ONE project, so every `path=`
-- dependency between the 8 workspace members was dropped, and the table held 0
-- rows despite the feature being live and tested. Folder-graining records them.
create table if not exists folder_dependencies (
  from_folder_id  uuid        not null references sensei.folders(id) on delete cascade
, to_folder_id    uuid        not null references sensei.folders(id) on delete cascade
, source_protocol text        not null
, source_manifest text        not null
, resolved_target text
, modified_at     timestamptz not null default now()
, primary key (from_folder_id, to_folder_id, source_manifest)
, check (from_folder_id <> to_folder_id)
);

create index if not exists folder_dependencies_from_idx on folder_dependencies(from_folder_id);
create index if not exists folder_dependencies_to_idx   on folder_dependencies(to_folder_id);

comment on table folder_dependencies is
'Manifest-declared dependency between two FOLDERS (D11). Written when a local
protocol dep (npm link:/workspace:/file:, Cargo path=) resolves to a sibling
folder. Regrained from project_dependencies: a manifest sits at a folder, so
that is the grain of the fact, and project/repository answers are views over
this. The composite PK makes it idempotent per declaring manifest; the CHECK
forbids a self-edge.';

comment on column folder_dependencies.from_folder_id is 'The folder whose manifest declares the dependency.';
comment on column folder_dependencies.to_folder_id   is 'The folder the local-protocol path resolves to. Derivable to a project via folders.project_id — which is why the project ids are no longer stored.';
comment on column folder_dependencies.source_protocol is 'link | workspace | file | path — the local protocol the manifest used.';
comment on column folder_dependencies.source_manifest is 'The manifest filename that declared it (package.json, Cargo.toml, ...). Part of the PK so two manifests in one folder are distinct edges.';
comment on column folder_dependencies.resolved_target is 'The raw target string after the protocol prefix, kept for display and re-derivation.';
