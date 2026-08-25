---
name: kavach-providers
description: Use when adding or configuring auth providers in Kavach — the providers[] array in kavach.config.js, provider modes (oauth / otp / password), the built-in provider names (google, github, azure, apple, linkedin, microsoft, magic, email, phone, …), per-provider OAuth scopes, custom/backend adapters, rendering the sign-in UI with @kavach/ui (AuthProvider) + the generated $kavach/providers list and kavach.signIn (instead of hand-rolled buttons calling the backend SDK directly), and theming/restyling those UI components via their data-* attributes (data-auth-provider, data-auth-mode, data-login-card, …) plus rokkit's data-skin / data-mode.
---

# Kavach Providers — Adding & Rendering Sign-In Methods

A provider is one row in the **`providers[]`** array of `kavach.config.js`. Kavach turns
that declaration into a normalized list on `$kavach/providers`, which your sign-in page
iterates to render `@kavach/ui` buttons. Each button calls `kavach.signIn(...)` on the shared
instance — you do **not** call the adapter's SDK (`supabase.auth.signInWithOAuth`, etc.) yourself.

```
kavach.config.js providers[]  ──▶  $kavach/providers  ──▶  {#each providers}<AuthProvider/>
   { name, mode, label,                (normalized)          │  getContext('kavach')
     scopes, params }                                        ▼  kavach.signIn({ provider, ... })
```

---

## 1. Declaring a provider

```js
// kavach.config.js
providers: [
  { name: 'google', label: 'Continue with Google' }, // oauth (inferred)
  { name: 'github', label: 'Continue with GitHub' }, // oauth (inferred)
  { name: 'magic', mode: 'otp', label: 'Email Magic Link' },
  { name: 'email', mode: 'password', label: 'Email' }
]
```

A provider entry (`ProviderConfig`):

| Field    | Required | Purpose                                                                          |
| -------- | -------- | -------------------------------------------------------------------------------- |
| `name`   | **yes**  | Provider id — drives the OAuth strategy and the `i-auth-<name>` icon             |
| `mode`   | no       | `oauth` \| `otp` \| `password` — inferred from `name` when omitted (table below) |
| `label`  | no       | Button text shown to the user                                                    |
| `scopes` | no       | `string[]` of OAuth scopes (see §3)                                              |
| `params` | no       | Extra provider params passed through to the adapter                              |

### Mode inference

When `mode` is omitted, Kavach picks it from the name:

| Name                                             | Inferred mode            |
| ------------------------------------------------ | ------------------------ |
| `magic`                                          | `otp` (email magic link) |
| `email`, `phone`                                 | `password`               |
| everything else (`google`, `github`, `azure`, …) | `oauth`                  |

Set `mode` explicitly whenever the default is wrong (e.g. `{ name: 'email', mode: 'password' }`).

### Built-in provider names

`google`, `azure`, `email`, `phone`, `apple`, `linkedin`, `microsoft`, `yahoo`, `github`,
`magic`, `twitter`, `facebook`. Each renders with the `i-auth-<name>` icon class by default.
An unlisted name still works as an `oauth` provider if your adapter/backend supports it.

---

## 2. Rendering the sign-in UI (the toolkit way)

Import the shared instance context once in the root layout (see **kavach-setup** §7), then
render one `AuthProvider` per generated provider. `AuthProvider` pulls `getContext('kavach')`
and calls `kavach.signIn(...)` internally — you never touch the backend SDK.

```svelte
<!-- src/routes/auth/+page.svelte -->
<script>
  import { AuthProvider } from '@kavach/ui'
  import { providers } from '$kavach/providers'
  import { goto } from '$app/navigation'

  const onSuccess = () => goto('/dashboard')
</script>

{#each providers as p (p.name)}
  <AuthProvider name={p.name} mode={p.mode ?? 'oauth'} label={p.label} onsuccess={onSuccess} />
{/each}
```

`@kavach/ui` also exports `AuthButton`, `AuthPassword`, `AuthGroup`, `AuthError`,
`AuthResponse`, `AuthHandler`, `LoginCard`, `LoginCardList`, and `AuthPage` for richer layouts.
`npx kavach add auth-page` scaffolds this page for you.

---

## 3. Scopes

OAuth scopes are declared per provider as `scopes: string[]`. They are joined with a space
and passed to the adapter's OAuth call (`options.scopes`).

```js
providers: [
  { name: 'google', label: 'Continue with Google', scopes: ['email', 'profile'] },
  { name: 'github', label: 'Continue with GitHub', scopes: ['read:user', 'user:email'] },
  { name: 'azure', label: 'Continue with Azure', scopes: ['email', 'profile', 'offline_access'] }
]
```

Scopes can also be passed at call time for a one-off elevated request:
`kavach.signIn({ provider: 'google', scopes: ['https://www.googleapis.com/auth/calendar'] })`.
The `AuthProvider` component accepts a `scopes` prop that forwards into that call.

---

## 4. Signing in / out programmatically

Use the shared instance from context — `kavach.signIn(credentials)` — keyed by provider mode:

```js
const kavach = getContext('kavach')

await kavach.signIn({ provider: 'google' }) // oauth
await kavach.signIn({ provider: 'email', email, password }) // password
await kavach.signIn({ provider: 'magic', email }) // otp / magic link
await kavach.signOut()
```

The reactive `authStatus` store (exported from `kavach`) updates on every sign-in/out if you
need to render loading/error state without prop-drilling.

---

## 5. Custom / backend providers (adapters)

Provider **behavior** lives in the adapter, not in your app. A backend adapter exposes
`getAdapter(client)` and implements the `AuthAdapter` interface
(`signIn`, `signUp`, `signOut`, `synchronize`, `onAuthChange`, optional `parseUrlError`,
`capabilities`). To add a provider the built-ins don't cover, extend the adapter — not your
UI:

```js
import { BaseAdapter } from 'kavach' // exported base to subclass

class MyAuthAdapter extends BaseAdapter {
  async signIn({ provider, mode, email, password, scopes }) {
    // branch on mode: 'otp' | 'password' | 'oauth' and call your backend
  }
}
```

The adapter's `signIn` receives the same credential shape `kavach.signIn` was called with, so
the UI and config stay identical regardless of backend.

---

## 6. Theming the components (data-\* attributes)

`@kavach/ui` components ship **no CSS and no CSS variables of their own**. They render a stable
set of `data-*` attributes; you restyle them from your app's global CSS by targeting those
attributes — never by forking the components. The palette/skin/mode layer (`data-skin`,
`data-mode`) comes from **rokkit** at the app root (see the rokkit semantic-styles / skin
skills); nest kavach's attributes under `[data-mode='…']` / `[data-skin='…']` for per-mode or
per-skin styling.

Kavach-rendered attributes:

| Attribute                               | Rendered on                                | Value         |
| --------------------------------------- | ------------------------------------------ | ------------- |
| `data-auth`                             | `AuthProvider` root, `AuthHandler`         | —             |
| `data-auth-provider="<name>"`           | `AuthProvider` root + its OAuth/OTP button | provider name |
| `data-auth-mode="oauth\|otp\|password"` | `AuthProvider` root / submit button        | the mode      |
| `data-item-icon` / `data-item-label`    | provider icon / label `<span>`             | —             |
| `data-auth-page`                        | `AuthPage` root                            | —             |
| `data-other-options`                    | `AuthPage` cached-logins `<details>`       | —             |
| `data-login-card`                       | `LoginCard` root                           | —             |
| `data-provider="<name>"`                | `LoginCard` badge                          | provider name |
| `data-passkey` / `data-remove`          | `LoginCard`                                | —             |
| `data-error`                            | `AuthError` root                           | —             |
| `data-alert`                            | `AuthResponse` root (+ class `hasError`)   | —             |

Also present from `@rokkit/ui` (style these too): `data-button`, `data-style` (`"none"` on OAuth
buttons), `data-size`, `data-variant`, `data-field`, `data-input-icon`.

Override recipe — brand each provider button, dark-mode aware. `--provider-*` are variables
**you** define (kavach exposes none); apply them scoped to rokkit's `data-mode`:

```css
/* app.css / global stylesheet */
[data-auth-provider] [data-item-icon] {
  width: 1.25rem;
  height: 1.25rem;
}
[data-auth-provider] [data-button] {
  width: 100%;
}

[data-auth-provider='github'] {
  --provider-bg: #24292e;
  --provider-text: #fff;
  --provider-border: #24292e;
}
[data-auth-provider='google'] {
  --provider-bg: #fff;
  --provider-text: #3c4043;
  --provider-border: #dadce0;
}

[data-mode='light'] [data-auth] > [data-button][data-style='none'] {
  background: var(--provider-bg);
  color: var(--provider-text);
  border-color: var(--provider-border);
}
[data-mode='dark'] [data-auth-provider] [data-button] {
  background: var(--provider-bg);
  color: var(--provider-text);
  border-color: var(--provider-border);
}
```

The `class` prop is only applied to the OTP `<form>`, not the OAuth button — reach for the
`data-*` selectors above to restyle buttons, not a `class` prop.

---

## Common mistakes

| Mistake                                                     | Why it fails                                             | Fix                                                                        |
| ----------------------------------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------- |
| Calling `supabase.auth.signInWithOAuth(...)` in a component | Bypasses Kavach session sync, cookie, and `onAuthChange` | `kavach.signIn({ provider })` via `getContext('kavach')`                   |
| Hand-building `<button>` login controls                     | Loses icons, mode handling, error/loading state          | `{#each providers}<AuthProvider/>` from `$kavach/providers` + `@kavach/ui` |
| `{ name: 'email' }` with no `mode`                          | Inferred `password`, but you wanted a link               | Set `mode` explicitly (`otp` for magic link, `password` for credentials)   |
| Hardcoding the provider list in the page                    | Drifts from `kavach.config.js`                           | Iterate the generated `providers` from `$kavach/providers`                 |
| Passing scopes as a comma string                            | Adapter expects an array (joined with space)             | `scopes: ['email', 'profile']`                                             |
| Creating a second `createKavach` instance to sign in        | Detached from the layout context/session                 | Reuse the shared `getContext('kavach')` instance                           |

To scaffold a correct sign-in page, run `npx kavach add auth-page`.
