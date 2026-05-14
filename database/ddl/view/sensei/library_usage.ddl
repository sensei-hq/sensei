set search_path to sensei, extensions;

CREATE OR REPLACE VIEW sensei.library_usage AS
SELECT rl.library_id
     , l.name                                           AS library_name
     , f.name                                           AS folder
     , f.project_id
     , rl.version_used
     , (SELECT COUNT(*)
          FROM sensei.edges e
         WHERE e.folder_id = rl.folder_id
           AND e.target_id IS NULL
           AND e.target_name LIKE l.name || '::%'
       )                                                AS unresolved_import_count
     , rl.props
     , rl.modified_at
  FROM sensei.referenced_libraries rl
  JOIN sensei.libraries l ON l.id = rl.library_id
  JOIN sensei.folders   f ON f.id = rl.folder_id;

comment on view library_usage is
'Per-folder library usage with import counts.
Powers the library detail panel: detected libraries, versions, and call site counts.
- library_id/library_name: which library
- folder/project_id: which repo uses it
- version_used: version from manifest (package.json, Cargo.toml)
- unresolved_import_count: edges targeting this library (approximation via name prefix)
- props: extensible metadata from referenced_libraries';
