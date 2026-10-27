set search_path to sensei, extensions;

-- Why a tracker has no owner on record (spec tracker-metrics-writeback W1).
-- Order and labels are pinned to `crate::tracker::TrackerOwnerState::ALL` by a test.
create type tracker_owner_state as enum (
  'unknown'        -- the provider did not say who the token belongs to; asked again each sync
, 'not_a_person'   -- the token belongs to an app account; never an assignee, never asked again
);

comment on type tracker_owner_state is
'Why sensei.trackers.owner_ref is NULL (W1). Mirrors crate::tracker::TrackerOwnerState.';
