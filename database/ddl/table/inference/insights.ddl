set search_path to inference, extensions;

create table if not exists insights (
  id                       uuid        primary key default gen_random_uuid()
, batch_id                 uuid        references inference.insight_batches(id) on delete set null
, source_table             text        not null
, source_id                uuid        not null
, category                 text        not null
, payload                  jsonb       not null default '{}'
, created_at               timestamptz not null default now()
);

create index if not exists insights_batch_id_idx
    on insights(batch_id)
 where batch_id is not null;

create index if not exists insights_category_idx
    on insights(category);

comment on table insights is
'Collective intelligence — references to local insights shared with the network.
Points to source records in recommendation_outcomes, detected_patterns, sessions, etc.
- source_table: which table the original insight lives in
- source_id: PK of the source record
- payload: anonymized snapshot of the insight at time of sharing
- batch_id: null until shared; links to the batch it was sent in';

comment on column insights.id
     is 'Surrogate primary key (UUID).';
comment on column insights.batch_id
     is 'Foreign key to insight_batches — which send batch included this insight. Null until shared.';
comment on column insights.source_table
     is 'Source table name: recommendation_outcomes, detected_patterns, sessions, etc.';
comment on column insights.source_id
     is 'Primary key of the source record in source_table.';
comment on column insights.category
     is 'Insight category: pattern, model, skill, correction, ftr, anti_pattern, tool, stack.';
comment on column insights.payload
     is 'Anonymized snapshot of the insight at share time. No code, paths, or identifiable info.';
comment on column insights.created_at
     is 'Timestamp when this insight was created.';
