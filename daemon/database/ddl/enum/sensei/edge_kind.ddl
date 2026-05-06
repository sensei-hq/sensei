set search_path to sensei, extensions;

create type edge_kind
    as enum (
        'calls', 'implements', 'extends', 'imports', 'depends_on'
      , 'traces_to', 'references', 'covers'
      , 'rationale_for', 'duplicates', 'similar_to'
    );
