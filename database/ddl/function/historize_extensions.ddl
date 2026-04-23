set search_path to sensei, history;

create or replace function historize_extensions()
returns trigger
language plpgsql
as
$$
begin
  if (tg_op = 'DELETE') then
    update history.past_extensions
       set effective_to = now()
     where extension_id = old.id
       and effective_to  is null;
    return old;
  end if;

  -- Auto-set modified_at if caller did not advance it
  if (new.modified_at is null
      or (tg_op = 'UPDATE' and new.modified_at = old.modified_at)) then
    new.modified_at := now();
  end if;

  -- Close the previous open history record on UPDATE
  if (tg_op = 'UPDATE') then
    update history.past_extensions
       set effective_to = new.modified_at
     where extension_id = old.id
       and effective_to  is null;
  end if;

  -- Bump revision
  new.revision := coalesce(old.revision, 0) + 1;

  -- Insert new history snapshot
  insert into history.past_extensions (
    extension_id
  , plugin_id
  , kind
  , name
  , version
  , description
  , content
  , props
  , scope
  , enabled
  , source
  , icons
  , tags
  , operation
  , revision
  , effective_from
  , effective_to
  , changed_at
  , changed_by
  )
  values (
    new.id
  , new.plugin_id
  , new.kind::text
  , new.name
  , new.version
  , new.description
  , new.content
  , new.props
  , new.scope
  , new.enabled
  , new.source
  , new.icons
  , new.tags
  , tg_op::history.dml_operation
  , new.revision
  , new.modified_at
  , null
  , new.modified_at
  , new.modified_by
  );

  return new;
end;
$$;

drop trigger if exists extensions_historize on sensei.extensions;
create trigger extensions_historize
before insert or update or delete
    on sensei.extensions
   for each row execute function historize_extensions();
