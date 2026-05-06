CREATE OR REPLACE VIEW sensei.project_drift AS
SELECT di.*, f.project_id
  FROM inference.drift_items di
  JOIN sensei.folders f ON f.id = di.folder_id
 WHERE f.project_id IS NOT NULL;
