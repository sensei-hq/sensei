set search_path to sensei, extensions;

-- Family a metric belongs to (the machine field the UI groups/colours from, never a
-- hardcoded name). See features/metrics/catalog.md for the per-metric assignment.
create type metric_family
    as enum (
        'outcome'
      , 'cost'
      , 'velocity'
      , 'quality'
      , 'autonomy'
      , 'knowledge'
      , 'tool'
      , 'composite'
    );
