set search_path to sensei, extensions;

create table if not exists index_errors (
  id                       serial      primary key
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, file_path                text        not null
, error                    text        not null
, adapter                  text
, phase                    text
, created_at               timestamptz not null default now()
);

create index if not exists idx_index_errors_folder
    on index_errors(folder_id);

comment on table index_errors is
'Files that failed during indexing. Persisted for debugging.';

comment on column index_errors.id
     is 'Surrogate primary key (serial).';
comment on column index_errors.folder_id
     is 'Foreign key to folders — which repo this error occurred in.';
comment on column index_errors.file_path
     is 'Path to the file that failed to index.';
comment on column index_errors.error
     is 'Error message or stack trace.';
comment on column index_errors.adapter
     is 'Language adapter that produced the error.';
comment on column index_errors.phase
     is 'Indexing phase when the error occurred.';
comment on column index_errors.created_at
     is 'Timestamp when this error was recorded.';
