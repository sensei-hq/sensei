set search_path to inference, sensei, extensions;

create table if not exists insight_batches (
  id                       uuid        primary key default gen_random_uuid()
, insight_count            integer     not null default 0
, target                   text        not null
, reference                text
, sent_at                  timestamptz not null default now()
);

comment on table insight_batches is
'Collective intelligence send batches. Groups insights shared together.
- target: "git", "posthog" — where insights were sent
- reference: git commit SHA, PostHog batch ID, etc. — traceable external reference';

comment on column insight_batches.id
     is 'Surrogate primary key (UUID).';
comment on column insight_batches.insight_count
     is 'Number of insights in this batch.';
comment on column insight_batches.target
     is 'Where insights were sent: git, posthog.';
comment on column insight_batches.reference
     is 'External reference: git commit SHA, PostHog batch ID, etc.';
comment on column insight_batches.sent_at
     is 'Timestamp when this batch was sent.';
