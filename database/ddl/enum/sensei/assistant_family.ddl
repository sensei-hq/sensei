set search_path to sensei, extensions;

create type assistant_family
    as enum (
        'claude'
      , 'cursor'
      , 'zed'
      , 'continue'
      , 'codex'
      , 'aider'
      , 'opencode'
      , 'kiro'
      -- APPENDED, not inserted. `copilot_cli` and `vscode` have had adapters
      -- since before this enum knew about them: `transcript_turns.family` is
      -- plain `text` and accepted either, but `assistant_events.family` casts to
      -- THIS type, so ingesting from either source failed outright. It stayed
      -- invisible because neither had ingested a row.
      --
      -- At the end rather than in alphabetical position because a value's
      -- ordinal is its sort order, and `alter type … add value` appends by
      -- default; placing them mid-list would renumber the rest for the sake of
      -- tidiness nobody reads.
      --
      -- `vscode` is the HOST EDITOR and `copilot` the harness, which is why they
      -- are two values and not one: mapping both onto a single name would make
      -- "which tool produced this turn" unanswerable for the two sources that
      -- most need it. See docs/spec/transcript-corpus.md.
      , 'copilot'
      , 'vscode'
    );
