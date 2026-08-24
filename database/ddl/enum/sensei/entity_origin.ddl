set search_path to sensei, extensions;

-- Where a shareable entity came from.
--
-- Separate from [`entity_scope`] on purpose: provenance and visibility are
-- independent. A `builtin` rule pack can be `public`; a `learned` memory is
-- usually `local`. Collapsing them — which is effectively what a single `source`
-- column did — makes it impossible to express either cleanly.
--
-- The distinction is load-bearing at re-import: a re-run of the marketplace
-- import may replace `imported` rows, but must never clobber an `authored` one,
-- because that is the user's own writing and nothing can recreate it. Without a
-- provenance column, "safe to overwrite" has no answer.
create type entity_origin as enum (
  -- Written by a human here. Unreproducible — never overwritten by an import.
    'authored'
  -- Derived by the analyzer from observed work. Reproducible in principle, but
  -- re-running costs inference spend and returns different text.
  , 'learned'
  -- Pulled in from a marketplace manifest or another install. Replaceable.
  , 'imported'
  -- Ships with the product. Replaced wholesale on upgrade.
  , 'builtin'
);
