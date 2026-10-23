set search_path to dojo, extensions;

-- Whether a repository key names this org — and, for Azure DevOps, this project —
-- AT ITS PLACE in the key (#308 R5b security review). Reading "anywhere in the
-- path" let a repository named like an org or a project file under it. The
-- places are the ones repo-mapping.ts reads (the daemon's normalised
-- `host/path` key, lowercase):
--   {host}/{org}/…                                      github.com, gitlab.com, bitbucket.org, dev.azure.com
--   ssh.dev.azure.com/v3/{org}/{project}/{repo}         and vs-ssh.visualstudio.com
--   {org}.visualstudio.com[/defaultcollection]/…        the org is in the host
-- An Azure project is `{project}/_git/{repo}`, or `_git/{repo}` for the
-- repository named after its project (https), or `{project}/{repo}` (SSH). The
-- project segment is compared percent-decoded. A project on a forge with none is
-- not named.
create or replace function dojo.repo_key_names(p_repo_key text, p_org text, p_project text)
    returns boolean
    language plpgsql
    immutable
    set search_path = dojo, pg_temp
as $$
declare
    v_p    text[] := string_to_array(lower(btrim(coalesce(p_repo_key, ''))), '/');
    v_org  text := lower(btrim(coalesce(p_org, '')));
    v_proj text := nullif(lower(btrim(coalesce(p_project, ''))), '');
    v_host text;
    v_b    integer;
    v_seg  text;
begin
    if v_org = '' or cardinality(v_p) < 3 then
        return false;
    end if;
    v_host := v_p[1];

    if v_host in ('ssh.dev.azure.com', 'vs-ssh.visualstudio.com') then
        if v_p[2] <> 'v3' or v_p[3] is distinct from v_org then
            return false;
        end if;
        if v_proj is null then
            return true;
        end if;
        return cardinality(v_p) >= 5 and dojo.percent_decode(v_p[4]) = v_proj;
    end if;

    if v_host like '%.visualstudio.com' then
        if v_host <> v_org || '.visualstudio.com' then
            return false;
        end if;
        v_b := case when v_p[2] = 'defaultcollection' then 3 else 2 end;
    else
        if v_p[2] is distinct from v_org then
            return false;
        end if;
        if v_proj is null then
            return true;
        end if;
        if v_host <> 'dev.azure.com' then
            return false;
        end if;
        v_b := 3;
    end if;

    if v_proj is null then
        return true;
    end if;
    v_seg := case
        when v_p[v_b] = '_git' then v_p[v_b + 1]
        when v_p[v_b + 1] = '_git' then v_p[v_b]
    end;
    return v_seg is not null and dojo.percent_decode(v_seg) = v_proj;
end;
$$;

revoke all on function dojo.repo_key_names(text, text, text) from public, anon, authenticated;

comment on function dojo.repo_key_names(text, text, text) is
'Whether a repository key names the org (and Azure project) at its place in the key (#308 R5b). Internal to dojo.register_repository.';
