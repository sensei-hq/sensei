-- The gateway seed: System One decision models (gateway SP-DEC-1, gh#72) and the
-- prominent models released Jun–Oct 2026, routed through the seeded routers.
--
-- Run after `dbd apply` + `dbd import` (the import procedures load
-- staging.* into gateway.*).
--
-- WHAT BREAKS THIS TEST: dropping `decision` from sensei.model_capability; a
-- typo in any staging row's provider/router/model name (the import procedures
-- INNER JOIN on names and silently drop the row — the last block catches that
-- for every row, not just the ones named here); losing the `decide` chain or
-- its order; a decision model also claiming `chat` (System One returns
-- probabilities, never text).

begin;

do $$
declare
    bad text;
begin
    -- ── the capability exists ───────────────────────────────────────────────
    if not ('decision' = any (enum_range(null::sensei.model_capability)::text[])) then
        raise exception 'sensei.model_capability has no ''decision'' value';
    end if;

    -- ── decision models: present, decision-only ─────────────────────────────
    select string_agg(n, ', ') into bad
      from unnest(array['nimble', 'tev1', 'clef', 'clef-flash', 'jev-1.13']) as n
     where not exists (
       select 1 from gateway.models m
        where m.full_name = n
          and m.capabilities = array['decision']::sensei.model_capability[]);
    if bad is not null then
        raise exception 'decision model(s) missing or not decision-only: %', bad;
    end if;

    -- ── each decision model reachable through the router that serves it ─────
    select string_agg(r || ' -> ' || f || ' as ' || id, '; ') into bad
      from (values
              ('ollama',     'nimble',     'nimble'),
              ('ollama',     'tev1',       'tev1'),
              ('ollama',     'clef',       'clef'),
              ('ollama',     'clef-flash', 'clef-flash'),
              ('openrouter', 'jev-1.13',   'typesafe/jev-1.13'),
              ('openrouter', 'tev1',       'togethercomputer/tev1-4b-experimental'),
              ('typesafe',   'jev-1.13',   'jev-1.13')
           ) as want(r, f, id)
     where not exists (
       select 1 from gateway.models_in_router mir
         join gateway.routers rt on rt.id = mir.router_id
         join gateway.models  m  on m.id  = mir.model_id
        where rt.name = want.r and m.full_name = want.f and mir.router_model_id = want.id);
    if bad is not null then
        raise exception 'decision routing missing: %', bad;
    end if;

    -- ── the hosted routers point where the gateway appends /v1/systemone ────
    select string_agg(name, ', ') into bad
      from (values ('openrouter', 'https://openrouter.ai/api'),
                   ('typesafe',   'https://api.typesafe.ai')) as want(name, url)
     where not exists (
       select 1 from gateway.routers r
        where r.name = want.name and r.api_base_url = want.url);
    if bad is not null then
        raise exception 'router(s) missing or with the wrong base url: %', bad;
    end if;

    -- ── the decide chain: local first, hosted last ──────────────────────────
    if not exists (select 1 from gateway.fallback_chains
                    where name = 'decide' and capability = 'decision') then
        raise exception 'no decide chain with capability decision';
    end if;
    select string_agg(format('%s:%s@%s', fcm.sequence_order, m.full_name, r.name), ' ' order by fcm.sequence_order)
      into bad
      from gateway.fallback_chain_models fcm
      join gateway.fallback_chains fc on fc.id = fcm.chain_id
      join gateway.models m  on m.id = fcm.model_id
      join gateway.routers r on r.id = fcm.router_id
     where fc.name = 'decide';
    if bad is distinct from '1:nimble@ollama 2:tev1@ollama 3:jev-1.13@openrouter' then
        raise exception 'decide chain order is %', coalesce(bad, '<empty>');
    end if;

    -- ── prominent recent models, each routable ──────────────────────────────
    select string_agg(n, ', ') into bad
      from unnest(array[
              'claude-fable-5-1', 'claude-opus-5-5', 'claude-sonnet-5-5',
              'gpt-6.1-sol', 'gpt-6-luna',
              'gemini-3.8-flash', 'grok-4.7', 'deepseek-v4.1-flash', 'kimi-k3', 'glm-5.3',
              'qwen3.8:27b', 'muse-glimmer:30b', 'nemotron-3.5-lightning:30b']) as n
     where not exists (
       select 1 from gateway.models m
         join gateway.models_in_router mir on mir.model_id = m.id
        where m.full_name = n);
    if bad is not null then
        raise exception 'recent model(s) missing or unroutable: %', bad;
    end if;

    -- ── nothing in the seed was silently dropped by an import join ──────────
    select string_agg(stg.router_name || '/' || stg.model_full_name, ', ') into bad
      from staging.models_in_router stg
     where not exists (
       select 1 from gateway.models_in_router mir
         join gateway.routers r on r.id = mir.router_id
         join gateway.models  m on m.id = mir.model_id
        where r.name = stg.router_name and m.full_name = stg.model_full_name);
    if bad is not null then
        raise exception 'staging.models_in_router rows that never landed: %', bad;
    end if;

    select string_agg(stg.full_name, ', ') into bad
      from staging.models stg
     where not exists (select 1 from gateway.models m where m.full_name = stg.full_name);
    if bad is not null then
        raise exception 'staging.models rows that never landed (unknown provider?): %', bad;
    end if;

    select string_agg(stg.chain_name || '/' || stg.model_full_name, ', ') into bad
      from staging.fallback_chain_models stg
     where not exists (
       select 1 from gateway.fallback_chain_models fcm
         join gateway.fallback_chains fc on fc.id = fcm.chain_id
         join gateway.models m on m.id = fcm.model_id
        where fc.name = stg.chain_name and m.full_name = stg.model_full_name);
    if bad is not null then
        raise exception 'staging.fallback_chain_models rows that never landed: %', bad;
    end if;
end $$;

rollback;
