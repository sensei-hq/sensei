-- Add `classify_turn_intent` (#447) to an existing sensei.task_execution_kind: dbd
-- never adds a value to an enum that already exists, so an upgraded install would
-- refuse to log the intent pass's executions. Appended, as the type's DDL appends it.
-- A no-op on a fresh database (no type yet) and once the value exists.
do $$
begin
    if to_regtype('sensei.task_execution_kind') is not null then
        alter type sensei.task_execution_kind add value if not exists 'classify_turn_intent';
    end if;
end
$$;
