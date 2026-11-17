-- The `intent` chain (#447): the correction-intent classifier runs on one local
-- model, qwen3.8 through Ollama, and nothing else.
--
-- Run after `dbd apply` + `dbd import`.
--
-- WHY ONE LEG: the classifier was measured on qwen3.8 (openspec
-- update-correction-detection-intent). A different model is a different
-- classifier with unmeasured precision and recall, so falling back along a
-- chain would mix labels silently; when qwen3.8 is unavailable the turns are
-- labelled by the phrase list, marked `phrase`. And the turns are the user's own
-- prompts: no cloud leg.
--
-- WHAT BREAKS THIS TEST: losing the chain, adding a leg, or the model or its
-- Ollama route dropping out of the seed.

begin;

do $$
declare
    legs text;
begin
    if not exists (select 1 from gateway.fallback_chains
                    where name = 'intent' and capability = 'classify') then
        raise exception 'no intent chain with capability classify';
    end if;
    select string_agg(format('%s:%s@%s', fcm.sequence_order, m.full_name, r.name), ' ' order by fcm.sequence_order)
      into legs
      from gateway.fallback_chain_models fcm
      join gateway.fallback_chains fc on fc.id = fcm.chain_id
      join gateway.models m  on m.id = fcm.model_id
      join gateway.routers r on r.id = fcm.router_id
     where fc.name = 'intent';
    if legs is distinct from '1:qwen3.8:27b@ollama' then
        raise exception 'intent chain legs are %', coalesce(legs, '<empty>');
    end if;
end $$;

rollback;
