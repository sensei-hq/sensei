set search_path to dojo, extensions;

-- Which way a Relay inbox row flows. `agent_to_human` = the daemon asking (gate,
-- decision, stall); `human_to_agent` = the person answering or steering (reply,
-- nudge, chat).
create type dojo.relay_message_direction
    as enum ('agent_to_human', 'human_to_agent');
