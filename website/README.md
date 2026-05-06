# Sensei Website

Marketing site for [Sensei](https://github.com/sensei-hq/sensei) — AI Development Intelligence.

**Live:** https://sensei-hq.github.io/sensei/

## Stack

- [SvelteKit](https://svelte.dev/docs/kit) with `adapter-static`
- [Rokkit UI](https://rokkit.vercel.app) components and themes
- [UnoCSS](https://unocss.dev) for utility classes
- [D3](https://d3js.org) for the architecture diagram

## Development

```sh
bun install
bun run dev
```

## Build

```sh
bun run build
bun run preview
```

Set `BASE_PATH=/sensei` for production builds (handled automatically by CI).

## Deployment

Pushes to `main` trigger GitHub Actions which builds the site and deploys to GitHub Pages.

## Known workarounds

- **Vite alias for `@rokkit/themes` CSS** — the npm package doesn't ship `dist/`, so `@import '@rokkit/themes/base.css'` is resolved to source via Vite alias. See [rokkit#136](https://github.com/jerrythomas/rokkit/issues/136).
