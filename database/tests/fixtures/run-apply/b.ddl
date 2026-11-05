create or replace function dojo.__apply_b() returns int language sql as $$ select dojo.__apply_a() + 1 $$;
