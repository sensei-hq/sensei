set search_path to inference, extensions;

create type edge_confidence
    as enum ('extracted', 'inferred', 'ambiguous');
