-- Patterns scoped to a project. `inference.detected_patterns` now carries
-- `project_id` (the authoritative scoping key) AND `folder_id` directly, so the
-- historical `SELECT dp.*, f.project_id` (which derived project_id from a folder
-- join back when the table lacked it) now DUPLICATES the `project_id` column —
-- and because CREATE OR REPLACE VIEW cannot add/reorder a column mid-list, that
-- definition fails to apply on any current schema (a fresh install can't create
-- it at all). Recreate the view straight over the table; the sole reader
-- (`PgStore::get_project_patterns`) filters `pp.project_id` and selects only
-- detected_patterns columns, so a bare passthrough is a superset of what it needs
-- and correctly includes folder-less patterns via the row's own project_id.
DROP VIEW IF EXISTS sensei.project_patterns;
CREATE VIEW sensei.project_patterns AS
SELECT * FROM inference.detected_patterns;
