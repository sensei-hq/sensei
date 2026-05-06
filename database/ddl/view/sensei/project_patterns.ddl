CREATE OR REPLACE VIEW sensei.project_patterns AS
SELECT dp.*, f.project_id
  FROM inference.detected_patterns dp
  JOIN sensei.folders f ON f.id = dp.folder_id
 WHERE f.project_id IS NOT NULL;
