set search_path to sensei, extensions;

create type node_kind
    as enum (
        'file'
      , 'module', 'package'
      , 'class', 'interface', 'function', 'method'
      , 'property', 'field', 'parameter'
      , 'type', 'const', 'enum', 'enum_variant'
      , 'section'
      , 'rationale'
    );
