set search_path to sensei, extensions;

-- Value type of a metric — drives how a roll-up aggregates it (ratio/pct re-derive
-- from numerator/denominator; count/currency sum; value/score take the period end).
create type metric_type
    as enum (
        'ratio'
      , 'pct'
      , 'count'
      , 'duration'
      , 'currency'
      , 'value'
      , 'score'
    );
