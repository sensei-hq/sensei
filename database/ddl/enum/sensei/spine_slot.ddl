set search_path to sensei, extensions;
-- The doc-slot names the scaffolder produces (project spine §3.2 + feature dossier).
-- A memory's `feature` column disambiguates scope: design/decisions exist at both
-- project and feature scope; feature=null → project, feature set → that feature.
create type spine_slot as enum (
  'vision', 'personas', 'journeys', 'roadmap', 'design', 'mockups',
  'decisions', 'brief', 'plan', 'tests'
);
