# Sign in — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/signin` — `dojo/src/routes/signin/+page.svelte` → renders `DojoSignIn.svelte`. Unauthenticated requests to guarded routes (`/console/*`, `/orgs`, `/org/*`, `/you/*`) are redirected here by `hooks.server.ts` (kavach `.handle`).
- Mockup: dojo2-app.jsx `ScrSignIn` (L1360)
- Access axis: auth (pre-tenant). This screen establishes the **User** — the Supabase auth subject, which entity-access-model §1 defines as the same identity as the git commit author ("Sensei's subject"). No tenant/membership axis applies until after auth; org selection happens next at `/orgs`.
- Status: PARTIAL — GitHub OAuth + email magic link are wired live through kavach → Supabase; but the self-host "Connect" button is a dead stub, and the left panel diverges from the mockup (a "Welcome back / Acme Corp" fixture-metrics panel, not the mockup's generic local-first marketing).

## ⚠ Fabricated-data debt — MUST fix on build (2026-07-29 fallback audit)
`DojoSignIn.svelte:3`/`:10` renders the `metrics` fixture from `dojo/src/lib/dojo-data.ts:17` as a live "shared mind since your last visit" snapshot ("34 lessons shared this week", "Acme Corp"…). **Impact:** the pre-auth splash presents fabricated numbers as real activity to every visitor. **Fix on build:** source real metrics from a real read, or make the panel obviously-static marketing copy — NEVER fabricated live figures presented as real; on a fetch error render an explicit error/empty state, and drop the hardcoded "Acme Corp". (Ties the daemon-side fabrication fixes + the "no fabricated fallbacks" rule.)

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
| --- | --- | --- | --- | --- |
| "Continue with GitHub" | primary button | `kavach.signIn({provider:'github', redirectTo: origin})` → Supabase `signInWithOAuth` | have (needs GitHub provider enabled on cloud Supabase; else surfaces error) | no |
| "Work email" input | `<input you@company.com>` | local `$state email` | have | no |
| "Email me a magic link" | button | `kavach.signIn({provider:'magic', email})` → Supabase OTP magic link | have | no |
| Send status/message | — (mockup has none) | local `$state status/message` (`sending`/`sent`/`error`) | have | no |
| Self-host toggle | `setSelfHost` link | local `$state selfHost` | have | no |
| Self-host URL input | `<input dojo.acme.internal>` | local `$state selfHostUrl` (default `dojo.acme.internal`) | have | no |
| Self-host "Connect" | button | **no handler** | **plumb** — inert; multi-domain / self-hosted auth not implemented | no |
| Left: brand (結 · Dōjō · dojo.sensei-hq.com) | static | literal | have (static) | no |
| Left: headline + copy | "A quiet companion…" | **diverged** — shipped shows "Your team kept learning while you were away" / "Acme Corp's shared mind" | **bind** — hardcoded "Acme Corp"; mockup is org-agnostic local-first copy | no |
| Left: metric cards (共/決/盾) | mockup has 観/己/盾 feature cards | `metrics` from `dojo-data.ts` (`contribWeek`, `approvedWeek`, `dereferenced`, `adoptionLift`, `contribSpark`) — **fixture** | **bind/plumb** — static fixture; mockup shows non-metric feature cards instead | no |
| Left: "Just published · Never log refresh tokens" | not in mockup | hardcoded literal | have (static fixture) | no |
| Footer copy | "One sign-in for hosted + self-hosted" | literal | have (static) | no |

## APIs / loaders
- No page loader (`signin/+page.svelte` renders `DojoSignIn` directly; `metrics` is a module import).
- Auth via kavach browser instance from `getContext('kavach')` (hydrated in `dojo/src/routes/+layout.svelte`; `undefined` under SSR/prerender → every use guarded, form renders regardless). kavach maps `provider:'github'` → `signInWithOAuth`, `provider:'magic'` → email OTP; both against the cloud Supabase project.
- Guard: `hooks.server.ts` → `kavach.handle` reads the session cookie into `event.locals.session` and enforces `kavach.config.js` rules — unauth → `/signin`.
- Post-auth flow (next screens, not this one): OAuth/magic redirect returns to origin → session established → user lands on `/orgs` (`orgs/+page.server.ts`: `sessionUser(locals)` + `listUserOrgs(user.id)` over `dojo.memberships → tenants`) → `enterOrg` sets the `dojo_tenant` cookie → `/org/{slug}`.

## Interactions & states
- GitHub → `status='sending'`; on `result.error` → `status='error'` + message; success → provider redirect.
- Magic link → `preventDefault`; requires `email`; on success `status='sent'` ("Check your email…"); on error surfaces the message. No-op if kavach absent (SSR) — never throws.
- Self-host → toggle reveals URL field; Connect does nothing.
- Message region is `role="status"`; errors in `text-danger`, success in `text-success` (no silent failure).
- Responsive: left panel hidden `<md`, 57% split `md:+` (form is primary on phones).

## Gap / to-do (vs mockup)
- Self-host "Connect" is inert — implement (or hide) self-hosted-dōjō auth against a custom domain.
- Left panel diverges: mockup is generic local-first marketing (feature cards 観/己/盾, "yours by default"); shipped is a "Welcome back / Acme Corp" fixture-metrics panel. Decide which is canonical; if metrics stay, source them per-user/per-org instead of the `dojo-data` fixture, and drop the hardcoded "Acme Corp".
- **Dereference framing (canon §5/Rule B):** the metric label "anonymized from client work · 0 incidents" implies client-only stripping — universal dereference is always-on for ALL work, not client-only. Reword to credit-neutral/universal.
- Mockup shows an inline `GhMark` SVG; shipped uses `i-simple-icons:github` — cosmetic, fine.

## Open questions (for Jerry)
- Which left-panel design is canonical — the mockup's generic local-first marketing, or the shipped "welcome-back metrics" panel? If metrics: what's the real per-user/per-org source (and drop the "Acme Corp" literal)?
- Is self-hosted-dōjō sign-in in scope now (wire Connect to a custom-domain auth handshake), or defer and hide the affordance?
- Confirm the GitHub OAuth provider is enabled/configured on the cloud Supabase project (client id/secret + callback) — without it the button errors at runtime.

### Resolved design (2026-07-30)
- **Q1 self-host → DEFER + HIDE the Connect affordance** pre-release (hosted GitHub OAuth + email magic-link suffice; no dead stub).
- **New-Q adapter → kavach-in-component** (auth-only, no data load; the three-layer Load seam adds nothing — no `signin.ts`).
- **Left panel → drop the `dojo-data.ts` "Acme Corp" fixture-metrics panel; render the mockup's generic local-first feature-card panel** (`SignInPanel`). (fabricated-data debt.)
- **Config-verify task (Q2, ops):** confirm the GitHub OAuth provider is enabled/configured on the cloud Supabase (client id/secret + callback) — else the button errors at runtime.
- **Depends on:** kavach/Supabase (live) + hiding self-host + the generic-panel copy + the OAuth provider config verify.
- Should the sign-in metric copy be reworded to reflect universal (not client-only) dereference per the data-model fix register (Rule B)?

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** — no dōjō table read; **auth-only**. kavach browser instance → Supabase `auth.users` (`signInWithOAuth` github / magic OTP). This screen establishes the **User** (canon §1: the same identity as the git commit author) — no tenant/membership axis until post-auth `/orgs`. The left-panel metrics are currently a `dojo-data.ts` fixture with a hardcoded **"Acme Corp"** (drop it; see UI). Reference the documented kavach + Supabase flow + `hooks.server.ts` guard above — not restated.

**API** — reuse the documented kavach `signIn({ provider })` → Supabase and the post-auth `/orgs` flow; no `/v1` call on this screen.

**UI** (components / state / types):

**Domain types**:
```ts
type AuthForm = { email: string; selfHost: boolean; selfHostUrl: string;
  status: 'idle'|'sending'|'sent'|'error'; message: string|null }
type LeftPanel = { headline: string; copy: string; features: { glyph; title; body }[] }
```
`LeftPanel` is **universal local-first** marketing (mockup feature cards 観/己/盾) — NO "Acme Corp", NO client-only dereference copy (Rule B: universal, always-on).

**State** — `signin-state.svelte.ts` → `signinState`
- data: `form: AuthForm`
- `$derived`: `canMagicLink` (email non-empty), `busy` (status==='sending')
- methods: `setEmail`, `toggleSelfHost`, `setSelfHostUrl`, `continueWithGithub()`, `sendMagicLink()`, `connectSelfHost()` (currently inert → plumb) — each wraps the auth adapter and sets `status`/`message`; SSR-safe (kavach absent → no-op, never throws), no silent failure

**Load** — `signin.ts` → the **auth seam** (mock/real), signin having no data loader:
- a thin auth adapter `{ oauth(provider), magic(email) }` + a left-panel provider `leftPanel(): LeftPanel`
- mock-first: fake adapter (resolves sent/error) drives the form states in tests; `leftPanel()` returns the universal local-first copy + feature cards — **NO Acme, NO client-only metric copy**
- real (body-swap only): kavach `getContext('kavach')` OAuth/magic; if metrics stay, source them per-user/org (drop the `dojo-data` fixture)

**Components** (pure, semantic, own styles + `md:`) — replace inline `DojoSignIn`/`ScrSignIn`:
- `SignInForm` — GitHub button + email/magic-link via `@rokkit/forms` (email schema) + self-host toggle/URL/Connect + `role="status"` message region (danger/success). Reads `signinState`. `md:` 57% split, form primary on phones.
- `SignInPanel` — left brand (`結` `KanjiToken`) + `LeftPanel` (universal copy + feature cards); hidden `<md`. **The copy fix lands here: universal-dereference framing, drop "Acme Corp".**

**Copy** — paraglide `m.<key>()`; brand/feature glyphs `KanjiToken`/Solar; the reworded universal-dereference marketing copy (Rule B: "anonymized from client work" → credit-neutral / universal) in the `messages/en.json` catalog.

**Realtime = State**: n/a. **Test seams:** state methods against the mock auth adapter (sending/sent/error, `canMagicLink`); `SignInPanel` with a mock `LeftPanel`; SSR-safe (no throw when kavach absent).

**New open question:** signin has no data load — is extracting the auth adapter into `signin.ts` worth it, or is kavach-in-component fine? (Domain type + `SignInPanel` already assume the mockup feature-card panel with no Acme.)
