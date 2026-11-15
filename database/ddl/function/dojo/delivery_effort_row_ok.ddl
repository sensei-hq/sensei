set search_path to dojo, extensions;

-- Whether one effort row's values and reasons are the D1 wire's, exactly (spec
-- tracker-delivery-metrics D1, A2; #353): only the kind's own keys (no rate, no
-- calibration, no linkage diagnostics — B10, B14), each of its type — a count or
-- amount a finite number, a time a timestamp, submitted_via pr or push, a map of
-- numbers where the wire has one — or null; and every reason a short code. Checked
-- in the database so a direct call cannot store a value every reader's merge would
-- then fail on. Pure.
create or replace function dojo.delivery_effort_row_ok(p_kind text, p_values jsonb, p_reasons jsonb)
    returns boolean
    language plpgsql
    immutable
    set search_path = dojo, pg_temp
as $$
declare
    v_times text[] := array['work_started_at', 'submitted_at', 'first_at', 'last_at'];
    v_maps  text[] := array['by_source', 'on_hold_by_reason'];
    v_keys  text[] := case p_kind
        when 'ticket' then array['sessions', 'api_cost_n', 'api_cost_partial', 'worked_ms', 'prompts',
            'ai_token_cost', 'compile_defects', 'test_defects', 'review_fixes', 'escaped_task_defects',
            'build_ok', 'build_fail', 'runs_unknown', 'unclassified_failures', 'build_unrecovered',
            'recovery_ms_sum', 'recovery_n', 'on_hold_ms', 'on_hold_count', 'on_hold_by_reason',
            'execution_ms', 'rework_ms', 'rework_cycles', 'effort_cost', 'rework_cost',
            'work_started_at', 'submitted_at', 'submitted_via']
        when 'sprint' then array['sessions', 'shared', 'by_source', 'active_secs', 'active_n',
            'tokens_fresh', 'tokens_fresh_n', 'cache_read', 'cache_read_n', 'cache_write',
            'cache_write_n', 'tokens_out', 'tokens_out_n', 'tokens_reasoning', 'tokens_reasoning_n',
            'metered_cost', 'metered_cost_n', 'claude_secs', 'claude_n', 'user_secs', 'user_n',
            'wall_secs', 'wall_n', 'first_at', 'last_at', 'api_cost', 'api_cost_n', 'api_cost_partial']
        when 'project' then array['sessions', 'tokens', 'sessions_without_tokens', 'prompts']
        end;
    e record;
    t timestamptz;
begin
    if v_keys is null or jsonb_typeof(p_values) is distinct from 'object'
       or jsonb_typeof(p_reasons) is distinct from 'object' then
        return false;
    end if;
    -- A contributor has at least one session on the item, or sends no row (D1).
    if jsonb_typeof(p_values -> 'sessions') is distinct from 'number' or (p_values ->> 'sessions')::numeric < 1 then
        return false;
    end if;
    for e in select * from jsonb_each(p_values) loop
        if not e.key = any(v_keys) then
            return false;
        end if;
        continue when jsonb_typeof(e.value) = 'null';
        if e.key = any(v_times) then
            if jsonb_typeof(e.value) <> 'string' then
                return false;
            end if;
            begin
                t := (e.value #>> '{}')::timestamptz;
            exception when invalid_datetime_format or datetime_field_overflow or invalid_parameter_value then
                return false;
            end;
        elsif e.key = 'submitted_via' then
            if (e.value #>> '{}') not in ('pr', 'push') then
                return false;
            end if;
        elsif e.key = any(v_maps) then
            if jsonb_typeof(e.value) <> 'object'
               or exists (select 1 from jsonb_each(e.value) m
                           where jsonb_typeof(m.value) <> 'number' or length(m.key) > 64) then
                return false;
            end if;
        elsif jsonb_typeof(e.value) <> 'number' or abs((e.value #>> '{}')::numeric) > 1e15 then
            return false;
        end if;
    end loop;
    return not exists (select 1 from jsonb_each(p_reasons) r
                        where not r.key = any(v_keys) or jsonb_typeof(r.value) <> 'string'
                           or length(r.value #>> '{}') > 64);
end
$$;

revoke all on function dojo.delivery_effort_row_ok(text, jsonb, jsonb) from public;

comment on function dojo.delivery_effort_row_ok(text, jsonb, jsonb) is
'Whether an effort row of this kind carries only the D1 wire''s keys, each of its type (number, timestamp, pr|push, map of numbers) or null, at least one session, and short reason codes for its own keys (#353). Pure.';
