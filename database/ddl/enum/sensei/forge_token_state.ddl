set search_path to sensei, extensions;

-- What we currently believe about a persona's forge (GitHub) access token.
--
-- Recorded rather than inferred, because the failure this exists for is
-- INVISIBLE: a token minted at sign-in was `401 Bad credentials` the next
-- morning, every forge-dependent operation degraded, and `/api/auth/status`
-- went on reporting `signedIn: true` throughout — that flag reflects the
-- SUPABASE session, which refreshes on every use and is a different credential
-- with a different lifetime.
--
-- `unknown` is a first-class state, not a placeholder. GoTrue's exchange does
-- not report the provider's expiry, so a token captured before this column
-- existed genuinely has no known standing, and saying so is more useful than
-- guessing: assuming `active` spends a doomed refresh, assuming `dead` tells the
-- user to sign in again for nothing.
create type forge_token_state as enum (
  'unknown'   -- stored, but never verified and no expiry recorded
, 'active'    -- verified against the forge, or within a known expiry
, 'dead'      -- expired or revoked; only a fresh sign-in clears this
, 'absent'    -- no token stored for this persona at all
);

comment on type forge_token_state is
'A persona forge token''s standing. `unknown` is REAL, not a default: GoTrue does not report the provider expiry, so a token captured before this existed has no known standing — and guessing either way costs something (a doomed refresh, or a needless sign-in prompt). Only a fresh sign-in clears `dead`.';
