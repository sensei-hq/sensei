set search_path to relay, extensions;

-- The claimable work, read-only (F01: a read never starts work). One row per
-- task in the pool or in review, classed for the claim order: a floating
-- review first, then a ready task whose dependencies are all done, then a
-- ready task that may be claimed early (its unmet dependencies listed; the
-- report is gated until they are done). A claimed review reads as such.
-- Security-invoker: the caller's RLS applies.
create or replace view relay.task_queue with (security_invoker = true) as
select
  t.user_id,
  t.crew_id,
  t.run_id,
  t.id as task_id,
  t.title,
  t.phase,
  t.state,
  t.round,
  t.repo_key,
  t.owned_paths,
  t.acceptance,
  t.lane_id as author_lane_id,
  t.review_claimed_by,
  case
    when t.state = 'in_review' and t.review_claimed_by is null then 'review_floating'
    when t.state = 'in_review' then 'review_claimed'
    when not exists (
      select 1 from relay.tasks d
       where d.user_id = t.user_id and d.id = any (t.deps) and d.state <> 'done')
      then 'ready_unblocked'
    else 'ready_claim_early'
  end as claim_eligibility,
  array(
    select d.id from relay.tasks d
     where d.user_id = t.user_id and d.id = any (t.deps) and d.state <> 'done'
     order by d.id) as unmet_deps,
  t.state_changed_at
from relay.tasks t
where t.state in ('ready', 'in_review');

comment on view relay.task_queue is
'The claim queue: review_floating, review_claimed, ready_unblocked, ready_claim_early. Read-only; claims go through the daemon.';
