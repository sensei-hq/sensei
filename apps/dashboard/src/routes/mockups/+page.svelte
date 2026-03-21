<script lang="ts">
  import { ThemeSwitcherToggle } from '@rokkit/app';

  const mockups = [
    {
      href: '/mockups/option-a',
      label: 'Option A',
      description: 'Minimal, developer-focused landing. Terminal snippet, feature grid, how-it-works steps.',
      tag: 'landing',
    },
    {
      href: '/mockups/option-b',
      label: 'Option B',
      description: 'Visual, marketing-style landing. Split hero with stats mockup, deep-dive feature sections.',
      tag: 'landing',
    },
  ];
</script>

<div class="min-h-screen bg-surface-z1 text-surface-z8">
  <!-- Header -->
  <header class="border-b border-surface-z3 px-8 py-4 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <a href="/" class="text-xs text-surface-z4 hover:text-surface-z6 transition-colors">← app</a>
      <span class="text-surface-z3">/</span>
      <span class="text-sm font-semibold text-surface-z8">mockups</span>
    </div>
    <ThemeSwitcherToggle />
  </header>

  <!-- Content -->
  <main class="mx-auto max-w-3xl px-8 py-12">
    <p class="mb-1 font-mono text-xs uppercase tracking-widest text-surface-z4">design exploration</p>
    <h1 class="mb-2 text-2xl font-bold text-surface-z8">Mockups</h1>
    <p class="mb-10 text-sm text-surface-z5">
      Design alternatives built directly in SvelteKit. Approve one, move the route, delete the rest.
    </p>

    <div class="space-y-3">
      {#each mockups as m}
        <a
          href={m.href}
          class="group flex items-center gap-5 rounded-xl border border-surface-z3 bg-surface-z2 px-5 py-4
                 hover:border-primary-z4 hover:bg-surface-z2 transition-all"
        >
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2 mb-1">
              <span class="text-sm font-semibold text-surface-z8 group-hover:text-primary-z7 transition-colors">
                {m.label}
              </span>
              <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-xs text-surface-z5">{m.tag}</span>
            </div>
            <p class="text-xs text-surface-z5">{m.description}</p>
          </div>
          <svg class="h-4 w-4 shrink-0 text-surface-z4 group-hover:text-primary-z6 group-hover:translate-x-0.5 transition-all"
               viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M3 8h10M9 4l4 4-4 4"/>
          </svg>
        </a>
      {/each}
    </div>
  </main>
</div>
