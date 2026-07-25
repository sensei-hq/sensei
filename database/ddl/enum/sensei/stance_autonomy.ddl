set search_path to sensei, extensions;

-- The autonomy axis of a stance: how far a session runs before it asks. Feeds the
-- relay drive / "progress-over-asking" gate — a higher setting lets a run proceed
-- through more without pausing for a human. Declared in ascending autonomy so
-- `ORDER BY` surfaces the freest last. `ask_on_guarded` (pause on a guarded step
-- — money/creds/destructive) is the sensible default.
create type stance_autonomy
    as enum ('ask_always', 'ask_on_guarded', 'ask_on_risky', 'run_freely');
