set search_path to dojo, extensions;

-- Lifecycle of a Relay inbox row. `pending` = awaiting the human; `answered` = a
-- reply was recorded (the daemon consumes it to continue); `expired` = timed out
-- unanswered (advisory items may lapse); `superseded` = replaced by a newer row for
-- the same point (e.g. the run advanced past it).
create type dojo.relay_inbox_status
    as enum ('pending', 'answered', 'expired', 'superseded');
