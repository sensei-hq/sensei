set search_path to sensei, extensions;

-- Batched share proposal — one row per group of memories a user reviews and
-- approves (or rejects) together for federation. Persists so a batch's
-- verdict is auditable after the fact and the Memories screen can render
-- an at-a-glance "N pending, M approved" summary without recomputing from
-- individual memory rows every time.
create table if not exists memory_share_batches (
  id             uuid                       primary key default gen_random_uuid()
, project_id     uuid                       not null references sensei.projects(id) on delete cascade
, status         memory_share_batch_status  not null default 'proposed'
, note           text
, created_at     timestamptz                not null default now()
, decided_at     timestamptz
, check (
    (status in ('proposed', 'withdrawn') and decided_at is null)
    or
    (status in ('approved', 'rejected')  and decided_at is not null)
  )
);

create index if not exists memory_share_batches_project_idx
    on memory_share_batches(project_id, status, created_at desc);

create index if not exists memory_share_batches_proposed_idx
    on memory_share_batches(project_id, created_at desc)
 where status = 'proposed';

comment on table memory_share_batches is
'Batched proposal to share memories from a project to the federation.
Users review a group of learned memories together and approve or reject
the batch as a unit. On approve, the daemon iterates the linked members
and pushes each to the configured hive-mind. Kept as a first-class
entity (not a memory-status flag) so verdicts stay auditable and the
UI can surface a per-project "pending review" queue.
- proposed: created, awaiting user decision
- approved: user accepted — the federation writer picks it up
- rejected: user declined
- withdrawn: user cancelled before deciding';

comment on column memory_share_batches.id
     is 'Surrogate primary key (UUID).';
comment on column memory_share_batches.project_id
     is 'FK to projects — batches are always scoped to a single project.';
comment on column memory_share_batches.status
     is 'Lifecycle stage. `proposed` and `withdrawn` have decided_at NULL; `approved` and `rejected` require decided_at.';
comment on column memory_share_batches.note
     is 'Optional free-text rationale attached at approve or reject time.';
comment on column memory_share_batches.created_at
     is 'When the batch was proposed.';
comment on column memory_share_batches.decided_at
     is 'When the batch was approved or rejected. NULL while proposed or if withdrawn.';
