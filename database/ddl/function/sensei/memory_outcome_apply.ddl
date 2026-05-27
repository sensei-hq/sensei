set search_path to sensei, extensions;

create or replace function sensei.memory_outcome_apply()
    returns trigger
    language plpgsql as
$$
declare
    last_violated_at  timestamptz;
    consec_applied    integer;
    cur_status        sensei.memory_status;
begin
    if NEW.outcome = 'applied' then
        select status into cur_status from sensei.memories where id = NEW.memory_id;

        update sensei.memories
           set reinforced_count = reinforced_count + 1
             , strength         = least(strength + 0.5, 5.0)
             , last_relevant_at = now()
             , modified_at      = now()
         where id = NEW.memory_id;

        if cur_status = 'challenged' then
            -- Recover to 'reinforced' only after 3 consecutive applied since last violation.
            select recorded_at
              into last_violated_at
              from sensei.memory_outcomes
             where memory_id = NEW.memory_id
               and outcome = 'violated'
             order by recorded_at desc
             limit 1;

            select count(*) into consec_applied
              from sensei.memory_outcomes
             where memory_id = NEW.memory_id
               and outcome = 'applied'
               and (last_violated_at is null or recorded_at > last_violated_at);

            if consec_applied >= 3 then
                update sensei.memories set status = 'reinforced' where id = NEW.memory_id;
            end if;
        else
            -- Promote to battle_tested when strength >= 4.0 and never violated.
            update sensei.memories
               set status = 'battle_tested'
             where id = NEW.memory_id
               and strength >= 4.0
               and violated_count = 0
               and status in ('active', 'reinforced');
        end if;

    elsif NEW.outcome = 'violated' then
        update sensei.memories
           set violated_count = violated_count + 1
             , strength       = greatest(strength - 0.7, 0.0)
             , status         = case when greatest(strength - 0.7, 0.0) < 1.0
                                     then 'archived'::memory_status
                                     else 'challenged'::memory_status end
             , last_relevant_at = now()
             , modified_at      = now()
         where id = NEW.memory_id;

    elsif NEW.outcome = 'consulted' then
        update sensei.memories
           set last_relevant_at = now()
             , modified_at      = now()
         where id = NEW.memory_id;

    -- ignored: no-op
    end if;

    return NEW;
end;
$$;

drop trigger if exists memory_outcome_apply_trg on sensei.memory_outcomes;
create trigger memory_outcome_apply_trg
    after insert on sensei.memory_outcomes
    for each row
    execute function sensei.memory_outcome_apply();
