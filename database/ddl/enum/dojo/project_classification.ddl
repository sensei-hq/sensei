set search_path to dojo, extensions;

-- How a project relates to the user who works it — drives which governance ladder
-- applies (constitution-by-classification) and the phone/console badge. `personal`
-- = the user's own work (no dōjō, tenant_id null); `company` = the user's employer
-- dōjō; `client` = client engagement work (source-dereferenced on publish, the
-- always-on invariant); `community` = a community/collective dōjō.
-- NB: dbd deploys enum variants alphabetically — never rely on declared order.
create type dojo.project_classification
    as enum ('company', 'client', 'personal', 'community');
