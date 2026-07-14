set search_path to dojo, extensions;

-- A member's authority inside a Dōjō. `contributor` = pull + contribute;
-- `maintainer` = triage/approve/distribute on owned scopes; `lead` =
-- guards confidentiality on client engagements (audit, incidents); `admin` =
-- runs the server, identity, provisioning, policies. Roles are usually
-- derived from the git provider role and may be overridden by an admin.
create type dojo.member_role
    as enum ('contributor', 'maintainer', 'lead', 'admin');
