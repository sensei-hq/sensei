set search_path to dojo, extensions;

-- A URL path segment, percent-decoded as UTF-8 (`mobile%20apps` → `mobile apps`,
-- `r%26d` → `r&d`), or NULL when it is not valid percent-encoding or not valid
-- UTF-8 — never a guessed reading. Used to compare a repository key's segment
-- with a forge name (#308 R5b).
create or replace function dojo.percent_decode(p_segment text)
    returns text
    language plpgsql
    immutable
    strict
    set search_path = dojo, pg_temp
as $$
declare
    v_bytes bytea := ''::bytea;
    v_i     integer := 1;
    v_ch    text;
begin
    while v_i <= length(p_segment) loop
        v_ch := substr(p_segment, v_i, 1);
        if v_ch = '%' then
            if substr(p_segment, v_i + 1, 2) !~ '^[0-9a-fA-F]{2}$' then
                return null;
            end if;
            v_bytes := v_bytes || decode(substr(p_segment, v_i + 1, 2), 'hex');
            v_i := v_i + 3;
        else
            v_bytes := v_bytes || convert_to(v_ch, 'UTF8');
            v_i := v_i + 1;
        end if;
    end loop;
    return convert_from(v_bytes, 'UTF8');
exception when others then
    return null;
end;
$$;

revoke all on function dojo.percent_decode(text) from public, anon, authenticated;

comment on function dojo.percent_decode(text) is
'A URL path segment percent-decoded as UTF-8, or NULL when it is not valid (#308 R5b). Internal.';
