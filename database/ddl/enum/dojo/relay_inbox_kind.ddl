set search_path to dojo, extensions;

-- What a Relay inbox row is asking for. `approval` = a blocking OK/deny gate;
-- `decision` = a 3–4-option (+ free-text) choice; `chat` = a free-form message in
-- the live session; `nudge` = an unsolicited human→agent steer/unstick; `stall` =
-- the engine surfacing a detected stall for the human.
create type dojo.relay_inbox_kind
    as enum ('approval', 'decision', 'chat', 'nudge', 'stall');
