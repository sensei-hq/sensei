-- A DDL file that persists nothing but announces itself if it runs. It follows
-- midline.ddl in the L-b pin: the per-file marker must stop the run before
-- this file executes (a transaction-ending file would otherwise let it
-- autocommit).
do $$ begin raise notice 'apply-after-midline-ran'; end $$;
