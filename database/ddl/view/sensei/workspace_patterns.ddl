set search_path to sensei, extensions;

-- Patterns scoped to a workspace. `inference.detected_patterns` now carries
-- `workspace_id` (the authoritative scoping key) AND `folder_id` directly, so the
-- historical `SELECT dp.*, f.workspace_id` (which derived workspace_id from a folder
-- join back when the table lacked it) now DUPLICATES the `workspace_id` column —
-- and because CREATE OR REPLACE VIEW cannot add/reorder a column mid-list, that
-- definition fails to apply on any current schema (a fresh install can't create
-- it at all). Recreate the view straight over the table; the sole reader
-- (`PgStore::get_workspace_patterns`) filters `pp.workspace_id` and selects only
-- detected_patterns columns, so a bare passthrough is a superset of what it needs
-- and correctly includes folder-less patterns via the row's own workspace_id.
DROP VIEW IF EXISTS sensei.workspace_patterns;
CREATE VIEW sensei.workspace_patterns AS
SELECT * FROM inference.detected_patterns;