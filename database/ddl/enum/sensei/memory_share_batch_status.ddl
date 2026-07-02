set search_path to sensei, extensions;

-- Lifecycle of a batch proposal to share memories from a project to the
-- federation (hive-mind). Batches start `proposed`, move to `approved` on
-- accept (memories then flow through the normal federation writer), or
-- `rejected` on decline. `withdrawn` covers user-initiated cancellation
-- before either verdict lands.
create type memory_share_batch_status
    as enum ('proposed', 'approved', 'rejected', 'withdrawn');
