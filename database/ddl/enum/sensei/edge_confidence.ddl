set search_path to sensei, extensions;

create type edge_confidence
    as enum ('extracted', 'inferred', 'ambiguous');
