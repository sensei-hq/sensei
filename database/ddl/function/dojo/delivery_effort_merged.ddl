set search_path to dojo, extensions;

-- One item's effort merged across its contributors (spec tracker-delivery-metrics
-- D1, A2, A3; #353): the figures one daemon holding everyone's sessions would
-- show, recomputed from summed numerators — never an average of ratios.
--
--   p_kind              project | sprint | ticket
--   p_rows              json array of the contributors' {values, reasons}
--   p_issue_created_at  the ticket's creation (a board fact), for lead_hours
--   p_tickets           the board's distinct tickets with a ticket row from anyone,
--                       the project's per-ticket denominator
--
-- Answers {values, reasons, partial}: every null in values has its reason; a
-- figure some contributor did not know has partial = how many. Summed minutes are
-- person-time. Pure.
create or replace function dojo.delivery_effort_merged(
    p_kind             text,
    p_rows             jsonb,
    p_issue_created_at timestamptz,
    p_tickets          integer
)
    returns jsonb
    language plpgsql
    immutable
    set search_path = dojo, pg_temp
as $$
declare
    acc       jsonb := '{"values": {}, "reasons": {}, "partial": {}}';
    f         text;
    m         record;
    pr        record;
    c         record;
    t         record;
    v_started record;
    v_cost    record;
    v_prompts record;
    v_sub     timestamptz;
    v_via     text;
    v_n       numeric;
    v_obj     jsonb;
begin
    if p_kind = 'sprint' then
        -- Usage sums; each carries its coverage in its _n, so nothing is partial.
        foreach f in array array['sessions', 'shared', 'active_secs', 'active_n', 'tokens_fresh',
                                 'tokens_fresh_n', 'cache_read', 'cache_read_n', 'cache_write',
                                 'cache_write_n', 'tokens_out', 'tokens_out_n', 'tokens_reasoning',
                                 'tokens_reasoning_n', 'metered_cost', 'metered_cost_n', 'claude_secs',
                                 'claude_n', 'user_secs', 'user_n', 'wall_secs', 'wall_n', 'api_cost',
                                 'api_cost_n', 'api_cost_partial'] loop
            select * into m from dojo.delivery_merge_sum(p_rows, f, 'sum');
            acc := jsonb_set(acc, '{values}', (acc -> 'values') || jsonb_build_object(f, m.merged::numeric));
        end loop;
        select * into m from dojo.delivery_merge_sum(p_rows, 'first_at', 'min');
        acc := jsonb_set(acc, '{values}', (acc -> 'values') || jsonb_build_object('first_at', m.merged::timestamptz));
        select * into m from dojo.delivery_merge_sum(p_rows, 'last_at', 'max');
        acc := jsonb_set(acc, '{values}', (acc -> 'values') || jsonb_build_object('last_at', m.merged::timestamptz));
        select jsonb_build_object(
                   'branch', sum((r -> 'values' -> 'by_source' ->> 'branch')::numeric),
                   'prompt', sum((r -> 'values' -> 'by_source' ->> 'prompt')::numeric))
          into v_obj
          from jsonb_array_elements(p_rows) r;
        acc := jsonb_set(acc, '{values}', (acc -> 'values') || jsonb_build_object('by_source', v_obj));
        return acc;
    end if;

    if p_kind = 'project' then
        foreach f in array array['sessions', 'tokens', 'sessions_without_tokens', 'prompts'] loop
            select * into m from dojo.delivery_merge_sum(p_rows, f, 'sum');
            acc := dojo.delivery_merge_put(acc, f, to_jsonb(m.merged::numeric), m.reason, m.partial);
        end loop;
        select * into m from dojo.delivery_merge_sum(p_rows, 'tokens', 'sum');
        acc := dojo.delivery_merge_put(acc, 'tokens_per_ticket',
            case when p_tickets > 0 and (acc #>> '{values,sessions_without_tokens}')::numeric = 0
                 then to_jsonb(m.merged::numeric / p_tickets) end,
            case when p_tickets is null or p_tickets = 0 then 'no_usage'
                 when (acc #>> '{values,sessions_without_tokens}')::numeric > 0 then 'partial_tokens'
                 else m.reason end,
            0);
        select * into m from dojo.delivery_merge_sum(p_rows, 'prompts', 'sum');
        acc := dojo.delivery_merge_put(acc, 'prompts_per_ticket',
            case when p_tickets > 0 and m.merged is not null and m.partial = 0
                 then to_jsonb(m.merged::numeric / p_tickets) end,
            case when p_tickets is null or p_tickets = 0 then 'no_usage'
                 when m.merged is null then m.reason
                 else (select r -> 'reasons' ->> 'prompts' from jsonb_array_elements(p_rows) r
                        where r -> 'reasons' ->> 'prompts' is not null
                        order by 1 limit 1) end,
            0);
        return acc;
    end if;

    if p_kind is distinct from 'ticket' then
        raise exception 'unknown effort item kind %', p_kind using errcode = 'invalid_parameter_value';
    end if;

    -- Straight sums.
    foreach f in array array['sessions', 'api_cost_n', 'api_cost_partial', 'prompts', 'ai_token_cost',
                             'compile_defects', 'test_defects', 'review_fixes', 'escaped_task_defects',
                             'runs_unknown', 'unclassified_failures', 'build_unrecovered', 'on_hold_count',
                             'rework_cycles', 'effort_cost', 'rework_cost'] loop
        select * into m from dojo.delivery_merge_sum(p_rows, f, 'sum');
        acc := dojo.delivery_merge_put(acc, f, to_jsonb(m.merged::numeric), m.reason, m.partial);
    end loop;
    -- Person-time, from milliseconds.
    foreach f in array array['worked', 'on_hold', 'rework'] loop
        select * into m from dojo.delivery_merge_sum(p_rows, f || '_ms', 'sum');
        acc := dojo.delivery_merge_put(acc, f || '_minutes', to_jsonb(m.merged::numeric / 60000), m.reason, m.partial);
    end loop;
    -- Holds per reason, in minutes, summed per reason.
    select * into m from dojo.delivery_merge_sum(p_rows, 'on_hold_by_reason', 'sum');
    select jsonb_object_agg(q.k, q.s) into v_obj
      from (select e.key as k, sum(e.value::numeric) as s
              from jsonb_array_elements(p_rows) r,
                   jsonb_each_text(case when jsonb_typeof(r -> 'values' -> 'on_hold_by_reason') = 'object'
                                        then r -> 'values' -> 'on_hold_by_reason' end) e
             group by e.key) q;
    acc := dojo.delivery_merge_put(acc, 'on_hold_by_reason',
        case when m.reason is null then case when v_obj is null then '{}'::jsonb else v_obj end end,
        m.reason, m.partial);

    -- Defects: the counts are known together (one derivation per session).
    select * into c from dojo.delivery_merge_sum(p_rows, 'compile_defects', 'sum');
    select * into t from dojo.delivery_merge_sum(p_rows, 'test_defects', 'sum');
    select * into pr from dojo.delivery_merge_pair(p_rows, 'review_fixes', 'escaped_task_defects');
    acc := dojo.delivery_merge_put(acc, 'review_defects',
        case when pr.known > 0 then to_jsonb(pr.num + pr.den) end, c.reason, pr.partial);
    v_n := c.merged::numeric + t.merged::numeric + pr.num + pr.den;
    acc := dojo.delivery_merge_put(acc, 'total_defects', to_jsonb(v_n), c.reason, c.partial);
    acc := dojo.delivery_merge_put(acc, 'testing_efficiency',
        case when v_n > 0 then to_jsonb((c.merged::numeric + t.merged::numeric) / v_n * 100) end,
        case when v_n is null then c.reason else 'no_defects' end, c.partial);
    acc := dojo.delivery_merge_put(acc, 'review_efficiency',
        case when v_n > 0 then to_jsonb((pr.num + pr.den) / v_n * 100) end,
        case when v_n is null then c.reason else 'no_defects' end, c.partial);

    -- Builds.
    select * into pr from dojo.delivery_merge_pair(p_rows, 'build_ok', 'build_fail');
    select * into m from dojo.delivery_merge_sum(p_rows, 'build_ok', 'sum');
    acc := dojo.delivery_merge_put(acc, 'build_runs',
        case when pr.known > 0 then to_jsonb(pr.num + pr.den) end, m.reason, pr.partial);
    acc := dojo.delivery_merge_put(acc, 'build_failure_rate',
        case when pr.num + pr.den > 0 then to_jsonb(pr.den / (pr.num + pr.den) * 100) end,
        case when pr.known = 0 then m.reason else 'no_runs' end, pr.partial);
    select * into pr from dojo.delivery_merge_pair(p_rows, 'recovery_ms_sum', 'recovery_n');
    select * into m from dojo.delivery_merge_sum(p_rows, 'recovery_n', 'sum');
    acc := dojo.delivery_merge_put(acc, 'mean_build_recovery_minutes',
        case when pr.den > 0 then to_jsonb(pr.num / pr.den / 60000) end,
        case when pr.known = 0 then m.reason else 'no_recovery' end, pr.partial);

    -- Ratios of person-time over the contributors who know both sides (A3).
    select * into pr from dojo.delivery_merge_pair(p_rows, 'on_hold_ms', 'execution_ms');
    select * into m from dojo.delivery_merge_sum(p_rows, 'execution_ms', 'sum');
    acc := dojo.delivery_merge_put(acc, 'on_hold_ratio',
        case when pr.den > 0 then to_jsonb(pr.num / pr.den * 100) end,
        case when pr.known = 0 then m.reason else 'no_window' end, pr.partial);
    select * into pr from dojo.delivery_merge_pair(p_rows, 'rework_ms', 'worked_ms');
    select * into m from dojo.delivery_merge_sum(p_rows, 'rework_ms', 'sum');
    acc := dojo.delivery_merge_put(acc, 'post_review_rework_ratio',
        case when pr.den > 0 then to_jsonb(pr.num / pr.den * 100) end,
        case when pr.known = 0 then m.reason else 'no_activity' end, pr.partial);

    -- Cost per prompt: only when every contributor's cost and prompts are whole.
    select * into v_cost from dojo.delivery_merge_sum(p_rows, 'ai_token_cost', 'sum');
    select * into v_prompts from dojo.delivery_merge_sum(p_rows, 'prompts', 'sum');
    acc := dojo.delivery_merge_put(acc, 'cost_per_prompt_cycle',
        case when v_cost.merged is not null and v_prompts.merged is not null
                  and v_cost.partial = 0 and v_prompts.partial = 0
                  and (acc #>> '{values,api_cost_n}')::numeric = (acc #>> '{values,sessions}')::numeric
                  and (acc #>> '{values,api_cost_partial}')::numeric = 0
                  and v_prompts.merged::numeric > 0
             then to_jsonb(v_cost.merged::numeric / v_prompts.merged::numeric) end,
        case when v_cost.merged is null then v_cost.reason
             when v_prompts.merged is null then v_prompts.reason
             when v_cost.partial > 0 or v_prompts.partial > 0
                  or (acc #>> '{values,api_cost_n}')::numeric < (acc #>> '{values,sessions}')::numeric
                  or (acc #>> '{values,api_cost_partial}')::numeric > 0 then 'partial_cost'
             else 'no_prompts' end,
        0);

    -- Timing: work starts at the earliest start; submission is the earliest PR,
    -- else the earliest push (A3).
    select * into v_started from dojo.delivery_merge_sum(p_rows, 'work_started_at', 'min');
    acc := dojo.delivery_merge_put(acc, 'work_started_at', to_jsonb(v_started.merged::timestamptz),
        v_started.reason, v_started.partial);
    select min((r -> 'values' ->> 'submitted_at')::timestamptz) into v_sub
      from jsonb_array_elements(p_rows) r where r -> 'values' ->> 'submitted_via' = 'pr';
    v_via := case when v_sub is not null then 'pr' end;
    if v_sub is null then
        select min((r -> 'values' ->> 'submitted_at')::timestamptz) into v_sub
          from jsonb_array_elements(p_rows) r where r -> 'values' ->> 'submitted_via' = 'push';
        v_via := case when v_sub is not null then 'push' end;
    end if;
    select * into m from dojo.delivery_merge_sum(p_rows, 'submitted_at', 'min');
    acc := dojo.delivery_merge_put(acc, 'submitted_at', to_jsonb(v_sub), m.reason, m.partial);
    acc := dojo.delivery_merge_put(acc, 'submitted_via', to_jsonb(v_via), m.reason, m.partial);
    acc := dojo.delivery_merge_put(acc, 'execution_hours',
        case when v_sub is not null and v_started.merged is not null
             then to_jsonb(extract(epoch from v_sub - v_started.merged::timestamptz) / 3600) end,
        case when v_sub is null then m.reason else v_started.reason end,
        greatest(v_started.partial, m.partial));
    acc := dojo.delivery_merge_put(acc, 'lead_hours',
        case when v_sub is not null and p_issue_created_at is not null and v_sub >= p_issue_created_at
             then to_jsonb(extract(epoch from v_sub - p_issue_created_at) / 3600) end,
        case when p_issue_created_at is null then 'not_synced'
             when v_sub is null then m.reason
             else 'created_after_submit' end,
        m.partial);
    return acc;
end
$$;

revoke all on function dojo.delivery_effort_merged(text, jsonb, timestamptz, integer) from public;

comment on function dojo.delivery_effort_merged(text, jsonb, timestamptz, integer) is
'One board item''s effort merged across contributors (tracker-delivery-metrics D1, A3): sums of known values with partial counts, min/max, and ratios recomputed from numerators and denominators summed over the same contributors; submission is the earliest PR, else push; summed minutes are person-time. Answers {values, reasons, partial}. Pure (#353).';
