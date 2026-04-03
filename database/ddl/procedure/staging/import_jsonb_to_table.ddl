set search_path to staging;

drop procedure if exists import_jsonb_to_table;
create or replace procedure import_jsonb_to_table(source varchar, target varchar)
language plpgsql as $$
declare
    type_definition text;
    dyn_sql         text;
begin

    -- construct type definition dynamically from target table columns
    -- ARRAY columns: udt_name is '_text', '_int4', etc. — strip leading _ and append []
    select string_agg(
        column_name || ' ' ||
        case
            when data_type = 'ARRAY' then regexp_replace(udt_name, '^_', '') || '[]'
            else data_type
        end,
        ', '
    )
      into type_definition
      from information_schema.columns
     where table_schema = split_part(target, '.', 1)
       and table_name   = split_part(target, '.', 2);

    -- Construct dynamic SQL for the INSERT operation
    -- ON CONFLICT DO NOTHING makes re-runs idempotent when staging tables have unique constraints
    dyn_sql := format(
        'insert into %s select rec.* from %s, lateral jsonb_to_record(data::jsonb) as rec(%s) on conflict do nothing',
        target, source, type_definition
    );

    -- Execute dynamic SQL
    execute dyn_sql;
end;
$$;
