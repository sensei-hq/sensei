set search_path to sensei, extensions;
-- Who decided a turn's intent (#447): the measured model, or the phrase list when
-- the model was unavailable. Metrics tell them apart; a later model pass relabels
-- phrase-sourced turns.
create type turn_intent_source as enum ('intent', 'phrase');

comment on type turn_intent_source is
'Who labelled a turn''s intent (#447): intent = the measured classifier; phrase = the key-phrase fallback, relabelled when the model returns.';
