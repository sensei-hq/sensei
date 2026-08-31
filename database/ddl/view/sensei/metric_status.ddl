set search_path to sensei, extensions;

-- One row per (repository × metric): should this compute, how far has it got, and
-- if it is not current, WHY.
--
-- ## Why this exists
--
-- The three facts needed to answer that question lived in three places and were
-- never joined: the registry's active window (`sensei.metrics`), the coverage
-- cursor (`sensei.metric_watermarks`, per repository × GROUP), and the tenants'
-- rulings (`sensei.metric_deactivations`). So "why is there no row for today?"
-- had no answer short of reading three tables and the planner's source.
--
-- That cost real time, and produced two wrong answers before the right one. A
-- deactivation was switched on for `dbd/churn`; no new row appeared. The control
-- — switching it back off — produced no row either, so both observations were
-- consistent with the gate working AND with it doing nothing. The first
-- explanation reached for was "churn is commit-cadence and dbd has no new
-- commits", read off the watermark table's comment. Also wrong. This view's own
-- answer is `sealed` through yesterday: churn is fully current, and the missing
-- days simply had no commits — sealing an empty day is correct. One read.
--
-- ## Cadence is read off the data, because the documented one is not implemented
--
-- A day-cadence group settles CALENDAR DAYS: `sealed_through` is the last settled
-- day, and today is never sealed (it reopens as late sessions land). "Current"
-- therefore means `sealed_through >= yesterday`. An empty day still seals, which
-- is why a gap in the VALUES is not a gap in coverage.
--
-- `sensei.metric_watermarks` also documents a COMMIT cadence: churn and quality
-- were to record the last commit walked in `last_sha` and process only newer
-- ones. MEASURED: `last_sha` is null in all 402 live rows, for every group. The
-- commit cadence is documented and unimplemented, and `last_sha` is a dead
-- column.
--
-- So cadence branches on the COLUMN, never on a group name. The first version of
-- this view trusted the doc and reported `walked` for groups that were in fact
-- sealed through yesterday — a worse error than the one it exists to prevent,
-- because it invented a reason instead of stating a state. If the engine ever
-- writes a `last_sha`, this begins reporting `commit` on its own.
--
-- ## Status precedence
--
-- First match wins, and the order encodes what a reader most needs to know:
--   1. `not_yet_effective` / `retired` — the registry says not now. Nothing else
--      matters, and a deactivation on a retired metric is noise.
--   2. `deactivated`        — every consuming dōjō switched it off. A CHOICE, so
--      it outranks progress: "behind" would invite fixing something deliberate.
--   3. `never_computed`     — no watermark row at all.
--   4. `walked`             — commit cadence, cursor held.
--   5. `sealed` / `behind`  — day cadence, against yesterday.
--
-- `reason_code` joins `sensei.reason_codes` (domain `metric_computation`) for the
-- human summary and remedy, exactly as `repository_sharing` does. The code is the
-- status: one vocabulary, not a status enum plus a parallel reason string.
create or replace view metric_status
    as
with repo as (
    select r.id, r.repo_key, r.name
      from sensei.repositories r
)
select repo.id                          as repository_id
     , repo.repo_key
     , repo.name                        as repository_name
     , m.key                            as metric
     , m.task_name                      as metric_group
     -- Read off the DATA, not a hardcoded group list. The watermark table's
     -- comment describes churn/quality as commit-cadence groups recording
     -- `last_sha`, but MEASURED: `last_sha` is null in all 402 live rows —
     -- every group, churn and quality included, advances `sealed_through`
     -- only. Branching on the column means this reports `commit` by itself
     -- if the engine ever writes one, and never contradicts the data.
     , case when w.last_sha is null then 'day' else 'commit' end as cadence
     -- The coverage cursor, per repository × GROUP: every metric in a group
     -- shares it, because the group is what computes as a unit.
     , w.sealed_through
     , w.last_sha
     , w.updated_at                     as watermark_updated_at
     -- Registry window, surfaced so a caller can see WHY without re-deriving it.
     , m.effective_from
     , m.effective_until
     , (d.metric_key is not null)       as deactivated
     , d.observed_at                    as deactivated_observed_at
     , case
         when m.effective_from > current_date            then 'not_yet_effective'
         when m.effective_until is not null
              and m.effective_until <= current_date      then 'retired'
         when d.metric_key is not null                   then 'deactivated'
         when w.repository_id is null                    then 'never_computed'
         when w.last_sha is not null                     then 'walked'
         when w.sealed_through >= current_date - 1        then 'sealed'
         else                                                 'behind'
       end                              as reason_code
  from repo
 cross join sensei.metrics m
  left join sensei.metric_watermarks w
    on w.repository_id = repo.id
   and w.metric_group  = m.task_name
  left join sensei.metric_deactivations d
    on d.repository_id = repo.id
   and d.metric_key    = m.key;

comment on view metric_status is
'One row per (repository × metric): the registry window, the coverage cursor, the
tenants'' ruling, and a `reason_code` from sensei.reason_codes (domain
metric_computation) saying why it is not current. Answers "why is there no row for
today?" in one read — the question that previously needed three tables and the
planner''s source.

Cadence is reported, and read off the DATA: a DAY group settles calendar days
(`sealed_through`, today never sealed, so current means >= yesterday) and an empty
day still seals — a gap in the values is not a gap in coverage. The COMMIT cadence
the watermark table documents is unimplemented: `last_sha` is null in all 402 live
rows, so `cadence` branches on that column rather than on a group name, and would
report `commit` by itself if the engine ever wrote one.

Precedence: registry lifecycle, then deactivation (a choice outranks progress),
then coverage. Cross-joined over the catalogue on purpose: a metric that has never
run for a repository still needs a row, or the absence is the thing you cannot
explain.';
