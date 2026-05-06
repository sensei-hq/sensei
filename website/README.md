# sensei website

Marketing website for Sensei.

**Live:** https://sensei-hq.github.io/sensei/

## Stack

- [SvelteKit](https://svelte.dev/docs/kit) with `adapter-static`
- [Rokkit UI](https://rokkit.vercel.app) components and themes
- [UnoCSS](https://unocss.dev) for utility classes
- [D3](https://d3js.org) for architecture diagrams

## Dev

```bash
# From monorepo root
make website-dev

# Or directly
bun install
bun run dev
```

## Build

```bash
bun run build
bun run preview
```

`BASE_PATH=/sensei` is set automatically by CI for production builds.

## Deployment

Pushes to `main` trigger the `deploy-website` GitHub Actions workflow, which builds and deploys to GitHub Pages.
