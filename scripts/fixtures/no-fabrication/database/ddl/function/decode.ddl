create or replace function decode(t text) returns text language plpgsql as $$
begin
  return convert_from(decode(t, 'escape'), 'utf8');
exception when others then -- expect: S2
  return t;
end;
$$;
