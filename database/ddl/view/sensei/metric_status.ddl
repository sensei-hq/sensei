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
-- ## "Never computed" needs BOTH the cursor and the values, in that order
--
-- The first version keyed it on the watermark alone: no row ⇒ never computed.
-- MEASURED 2026-08-31 on a 67-repository install, that reported `never_computed`
-- for 201 pairs — and 12 of them had metric values computed THAT DAY.
--
-- The cause is cadence. `planner::snapshot_active` runs the SNAPSHOT groups
-- (today `cost`, `coverage`, `knowledge`) as a today-only rolling compute that
-- writes NO watermark by design, because there is no historical day to settle.
-- Their absence of a cursor is correct, and reading it as "never ran" described a
-- working group as a broken one.
--
-- The obvious repair — decide "never" from the VALUES instead — is worse, and was
-- also measured: it moved 1,126 pairs out of `sealed` into `never_computed`,
-- because a day group SEALS AN EMPTY DAY. A repository whose days are all settled
-- and which simply had nothing to measure has no values and is perfectly current.
-- That is this file's own principle two sections up, and keying on values alone
-- contradicts it.
--
-- So the cursor is authoritative WHERE IT EXISTS, and the values are consulted
-- only where it does not:
--
--   cursor present  →  coverage is whatever the cursor says (walked/sealed/behind)
--   cursor absent   →  values decide: some ⇒ `no_day_cursor`, none ⇒ `never_computed`
--
-- On the live install that splits the 201 into 12 and 189 and moves nothing else.
--
-- `no_day_cursor` names the OBSERVATION (values, no cursor) rather than the
-- classification (snapshot group). That matters because the same state is
-- reachable a second way — a day-keyed group that wrote values and then failed
-- before sealing, since `fill_and_seal` advances nothing unless every day
-- succeeded. A code named `snapshot` would assert a cause the view cannot see.
--
-- ## Status precedence
--
-- First match wins, and the order encodes what a reader most needs to know:
--   1. `not_yet_effective` / `retired` — the registry says not now. Nothing else
--      matters, and a deactivation on a retired metric is noise.
--   2. `deactivated`        — every consuming dōjō switched it off. A CHOICE, so
--      it outranks progress: "behind" would invite fixing something deliberate.
--   3. `never_computed`     — no VALUE has ever been written for this pair.
--   4. `no_day_cursor`      — it has run, but its group keeps no watermark.
--   5. `walked`             — commit cadence, cursor held.
--   6. `sealed` / `behind`  — day cadence, against yesterday.
--
-- `reason_code` joins `sensei.reason_codes` (domain `metric_computation`) for the
-- human summary and remedy, exactly as `repository_sharing` does. The code is the
-- status: one vocabulary, not a status enum plus a parallel reason string.
create or replace view metric_status
    as
with repo as (
    select r.id, r.repo_key, r.name
      from sensei.repositories r
), computed as (
    -- Has this (repository × metric) EVER produced a value? The question
    -- `never_computed` claims to answer, asked of the values rather than
    -- inferred from a cursor. `repository_id is not null` keeps this to
    -- repo-scoped rows: a shared registry metric writes a NULL-repository row
    -- that belongs to no repository and must not credit one.
    select rm.metric_id
         , rm.repository_id
         , max(rm.computed_on) as last_computed_on
      from sensei.repository_metrics rm
     where rm.repository_id is not null
     group by rm.metric_id, rm.repository_id
)
select repo.id                          as repository_id
     , repo.repo_key
     , repo.name                        as repository_name
     , m.key                            as metric
     , m.task_name                      as metric_group
     -- Read off the DATA, not a hardcoded group list. Three cadences, and each
     -- branch is an OBSERVATION rather than a classification:
     --
     --  * `commit` — a `last_sha` cursor exists. MEASURED: null in every live
     --    row, so the commit cadence the watermark table documents is
     --    unimplemented. Branching on the column means this reports `commit` by
     --    itself if the engine ever writes one.
     --  * `day` — a `sealed_through` cursor exists, so days are being settled.
     --  * `snapshot` — no cursor at all. The SNAPSHOT groups (`planner::
     --    snapshot_active` — today `cost`, `coverage`, `knowledge`) compute
     --    today-only and write no watermark BY DESIGN, so there is nothing to
     --    advance. Naming the observation and not the classification keeps this
     --    honest for the other way to reach the same state: a day-keyed group
     --    that wrote values and then failed before sealing.
     , case
         when w.last_sha is not null      then 'commit'
         when w.repository_id is not null then 'day'
         else                                  'snapshot'
       end                              as cadence
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
         -- The CURSOR is asked first, and values only decide the no-cursor case.
         --
         -- Both halves of that were measured. Keying "never" on the watermark
         -- alone called 201 pairs never_computed, 12 with values from that same
         -- day — their groups are snapshot cadence and keep no cursor. But
         -- keying it on values alone was worse: it moved 1,126 pairs from
         -- `sealed` to `never_computed`, because a day group SEALS AN EMPTY DAY.
         -- A repository with complete coverage and nothing to measure has no
         -- values and is not behind, which is the principle in this file's
         -- header — a gap in the values is not a gap in coverage.
         --
         -- So: with a cursor, coverage is whatever the cursor says. Without one,
         -- values separate "has run, keeps no cursor" from "never ran".
         when w.repository_id is null and c.metric_id is null then 'never_computed'
         when w.repository_id is null                    then 'no_day_cursor'
         when w.last_sha is not null                     then 'walked'
         when w.sealed_through >= current_date - 1        then 'sealed'
         else                                                 'behind'
       end                              as reason_code
     -- APPENDED, not placed beside the other timestamps, because `create or
     -- replace view` can only add columns at the END — inserting one mid-list
     -- fails with "cannot change name of view column". Position is not part of
     -- this view's contract: every reader selects by name.
     --
     -- The last day this pair actually produced a value. Independent of the
     -- cursor, and the evidence that separates "never ran" from "runs, but keeps
     -- no cursor".
     , c.last_computed_on
  from repo
 cross join sensei.metrics m
  left join sensei.metric_watermarks w
    on w.repository_id = repo.id
   and w.metric_group  = m.task_name
  left join sensei.metric_deactivations d
    on d.repository_id = repo.id
   and d.metric_key    = m.key
  left join computed c
    on c.metric_id     = m.id
   and c.repository_id = repo.id;

comment on view metric_status is
'One row per (repository × metric): the registry window, the coverage cursor, the
tenants'' ruling, and a `reason_code` from sensei.reason_codes (domain
metric_computation) saying why it is not current. Answers "why is there no row for
today?" in one read — the question that previously needed three tables and the
planner''s source.

"Never computed" is decided by VALUES (sensei.repository_metrics), not by the
absence of a cursor. Keying it on the watermark reported never_computed for 201
pairs on a 67-repository install, 12 of which had values computed that same day:
their groups are SNAPSHOT cadence (cost/coverage/knowledge) and write no watermark
by design. Those now read `no_day_cursor` — named for the observation, since a
day-keyed group that wrote values and then failed before sealing reaches the same
state. The other 189 have no values either, so never_computed is true of them.

Cadence is reported, and read off the DATA: DAY settles calendar days
(`sealed_through`, today never sealed, so current means >= yesterday) and an empty
day still seals — a gap in the values is not a gap in coverage. SNAPSHOT has no
cursor to advance. The COMMIT cadence the watermark table documents is
unimplemented: `last_sha` is null in every live row, so `cadence` branches on that
column rather than on a group name, and would report `commit` by itself if the
engine ever wrote one.

Precedence: registry lifecycle, then deactivation (a choice outranks progress),
then coverage. Cross-joined over the catalogue on purpose: a metric that has never
run for a repository still needs a row, or the absence is the thing you cannot
explain.';
