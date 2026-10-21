set search_path to dojo, extensions;

-- Why a membership was disabled (#309). `left_org`: a complete forge read no
-- longer lists the person in the org — rejoining restores it. Any other way a
-- membership ends gets its own value, so a rejoin never undoes it.
create type dojo.membership_disabled_reason
    as enum ('left_org');
