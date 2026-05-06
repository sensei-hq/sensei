set search_path to sensei, history;

create or replace function historize_memories()
returns trigger
language plpgsql
as
$$
begin
  if (tg_op = 'DELETE') then
    update history.past_memories
       set effective_to = now()
     where memory_id = old.id
       and effective_to is null;
    return old;
  end if;

  -- Auto-set modified_at if caller did not advance it
  if (new.modified_at is null
      or (tg_op = 'UPDATE' and new.modified_at = old.modified_at)) then
    new.modified_at := now();
  end if;

  -- Close the previous open history record on UPDATE
  if (tg_op = 'UPDATE') then
    update history.past_memories
       set effective_to = new.modified_at
     where memory_id = old.id
       and effective_to is null;
  end if;

  -- Insert new history snapshot
  insert into history.past_memories (
    memory_id
  , project_id
  , scope
  , scope_filter
  , type
  , title
  , content
  , impact
  , strength
  , status
  , session_id
  , operation
  , effective_from
  , effective_to
  , changed_at
  )
  values (
    new.id
  , new.project_id
  , new.scope::text
  , new.scope_filter
  , new.type::text
  , new.title
  , new.content
  , new.impact
  , new.strength
  , new.status::text
  , new.session_id
  , tg_op::sensei.dml_operation
  , new.modified_at
  , null
  , new.modified_at
  );

  return new;
end;
$$;

drop trigger if exists memories_historize on sensei.memories;
create trigger memories_historize
before insert or update or delete
    on sensei.memories
   for each row execute function historize_memories();
