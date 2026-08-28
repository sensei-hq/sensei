set search_path to dojo, extensions;

-- Who may decide whether a repository's metrics are shared.
--
-- Derived from (tenant.origin, repository.visibility), never stored on the
-- repository: an org's PUBLIC repo is the user's call, its PRIVATE repo is the
-- organisation's. Stored on an ELECTION, which records who actually decided.
create type share_authority as enum ('user', 'organization');

comment on type share_authority is
'user = the electing member decides (personal repos, and any public repo).
organization = the organisation decides, and the member cannot override it in
either direction (its own private repos, on its own subscription).';
