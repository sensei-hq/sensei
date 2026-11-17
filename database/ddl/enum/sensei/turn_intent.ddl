set search_path to sensei, extensions;
-- What a user turn means for the reply it answers (#447): it corrects that reply,
-- states a durable rule, or neither.
create type turn_intent as enum ('correction', 'neither', 'principle');

comment on type turn_intent is
'A user turn''s intent toward the agent reply it answers (#447): correction, principle (a durable rule), or neither.';
