# In-App Mockups Design

> **Status:** approved
> **Date:** 2026-03-17

## Goal

A skill that builds UI design alternatives as real pages inside the existing app (`/mockups/a`, `/mockups/b`, ...) using the project's actual frameworks, component libraries, and styling tools — rather than standalone HTML prototypes. Once a design is chosen, the winner is promoted to its final location and the mockup scaffolding is removed.

## Problem

Claude Code's built-in visual companion generates self-contained HTML mockups. When the project is already a visual framework (SvelteKit, React, etc.), this creates double work:

1. Design iteration happens in HTML → tokens spent discussing/changing
2. Approved design is re-implemented in the real framework → more tokens + potential drift

This is wasteful. The HTML mockup is thrown away and the real implementation starts from scratch.

## Solution

Build mockups directly in the working app. Since the app is already running, alternatives are immediately visible in a browser without any extra tooling. The final step is move-and-cleanup, not re-implement.

---

## Architecture

**Mockup pages** — created under a temporary `mockups/` route in the app:
- SvelteKit: `src/routes/mockups/a/+page.svelte`, `src/routes/mockups/b/+page.svelte`, ...
- React (Next.js): `app/mockups/a/page.tsx`, `app/mockups/b/page.tsx`, ...
- Other routers: equivalent pattern — a `mockups/` prefix that keeps alternatives isolated

**No new framework** — uses the same components, libs, and styling the rest of the app uses. For this project: Rokkit components, UnoCSS/Tailwind utilities, and SvelteKit conventions.

**Promotion** — once the user approves an alternative:
1. Move the winning page to its final route
2. Update any imports or component references
3. Delete the `mockups/` directory and all alternatives

---

## Skill Procedure

### Phase 1: Setup

1. Detect framework from project (check `package.json`, directory structure)
2. Identify UI libs and styling tools in use
3. Ask: how many alternatives? (default: 2 — `a` and `b`)
4. Ask: what is this mockup for? (route name, component name, or description)
5. Create the `mockups/` route structure if it doesn't exist

### Phase 2: Build Alternatives

For each alternative:
1. Create the route file using the project's framework pattern
2. Implement the design using the real component library and styling
3. Make it functional enough to evaluate (real components, not placeholders)
4. Each alternative explores a meaningfully different approach — not just color variations

### Phase 3: Review

1. Report the URLs to visit (e.g., `http://localhost:5173/mockups/a`, `/mockups/b`)
2. Wait for user feedback
3. Iterate on any alternative based on feedback — this is the design loop
4. Repeat until user picks one

### Phase 4: Promote

Once user says "use a" (or "b", etc.):
1. Move winning files to the intended final location
2. Update imports, route references
3. Delete `src/routes/mockups/` (or framework equivalent)
4. Report what was moved where

---

## Skill File Structure

```
name: building-app-mockups
description: Use when designing new UI pages, components, or layouts in a
visual framework (SvelteKit, React, Next.js) — builds alternatives directly
as real app pages at /mockups/a, /mockups/b using the project's actual
components and styling, so design decisions happen in the real app, not in
throwaway HTML. Also use before any significant layout change to compare
options side-by-side without committing to one.
```

---

## Scope

- Applies to: SvelteKit, React (Next.js, Remix, Vite), Vue, Nuxt — any framework with a file-based router
- Does NOT apply to: backend code, pure logic changes, mobile apps, static sites without routing
- Mockup pages are always temporary — they get deleted after promotion

---

## Files Created/Modified

| File | Action |
|------|--------|
| `skills/building-app-mockups/SKILL.md` | Create |
