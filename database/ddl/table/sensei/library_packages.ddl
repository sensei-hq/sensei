set search_path to sensei, extensions;

-- The packages a library publishes, when it publishes under names other than its
-- own: `rokkit` → `@rokkit/ui`, `@rokkit/actions`, `@rokkit/states`, …
--
-- ## Why this exists
--
-- Two identities for the same library never met. Dependency detection reads
-- package.json and produces one `libraries` row per PACKAGE — measured
-- 2026-09-01: 11 `@rokkit/*` rows and 7 `@jerrythomas/dbd-*` rows. The library's
-- own `sensei.library.json` calls itself `rokkit`, and its skills and agents hang
-- off THAT row. So a project that depends on `@rokkit/ui` could not reach the four
-- curated rokkit skills: nothing connected the name it knows to the name the
-- capabilities live under.
--
-- ## Declared, never inferred
--
-- The rows come from the manifest's `packages` array. A name-prefix guess
-- (`@rokkit/*` → rokkit) would be wrong for any library whose packages are not
-- named after it, and would silently claim packages belonging to someone else who
-- happens to share a scope. The library is the only party that knows its own
-- membership, so it declares it.
--
-- ## Shape
--
-- `package_name` is the ecosystem-qualified name as a dependency file spells it
-- (`@rokkit/ui`), which is exactly what detection stored, so resolution is an
-- equality join and not a pattern match.
--
-- Primary-keyed on `package_name` alone, not on (library_id, package_name): a
-- package belongs to exactly ONE library, and two libraries claiming the same
-- package is a conflict to reject at write time rather than a row to store twice.
create table if not exists library_packages (
  package_name  text          primary key
, library_id    uuid          not null references sensei.libraries(id) on delete cascade
, source        text          not null default 'manifest'
, modified_at   timestamptz   not null default now()
);

create index if not exists library_packages_library_id_idx
    on library_packages(library_id);

comment on table library_packages is
'Packages a library publishes under other names (rokkit → @rokkit/ui, …), declared
in its sensei.library.json `packages` array. Lets a project that depends on
@rokkit/ui resolve to the library whose skills/agents hang off the `rokkit` row —
the two identities dependency detection and the manifest produce. Declared, never
prefix-inferred: a guess would claim packages in a shared scope that belong to
someone else. Keyed on package_name because a package belongs to one library.';
comment on column library_packages.package_name is 'Ecosystem-qualified name as a dependency file spells it (@rokkit/ui) — matches what detection stored, so resolution is an equality join.';
comment on column library_packages.library_id   is 'FK to libraries — the parent whose skills/agents this package should resolve to.';
comment on column library_packages.source       is 'Provenance: manifest (declared) — reserved for a future curated/registry writer.';
