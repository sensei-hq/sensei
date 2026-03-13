<script lang="ts">
  import 'uno.css'
  import '../app.css'
  import { vibe } from '@rokkit/states'
  import { themable } from '@rokkit/actions'
  import { ThemeSwitcherToggle } from '@rokkit/app'

  let { children } = $props()

  const navItems = [
    { href: '/', label: 'Dashboard' },
    { href: '/stats', label: 'Stats' },
    { href: '/repos', label: 'Repos' },
    { href: '/libraries', label: 'Libraries' },
    { href: '/references', label: 'References' },
    { href: '/benchmarks', label: 'Benchmarks' },
  ]
</script>

<svelte:head>
  <title>Sensei</title>
</svelte:head>
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

<div class="flex h-screen w-screen flex-col overflow-hidden">
  <header class="flex shrink-0 items-center justify-between border-b border-surface-z3 bg-surface-z2 px-6 py-3">
    <div class="flex items-center gap-8">
      <span class="text-lg font-semibold text-primary-z6">sensei</span>
      <nav class="flex gap-4">
        {#each navItems as item}
          <a href={item.href} class="text-sm text-surface-z6 transition-colors hover:text-surface-z8">
            {item.label}
          </a>
        {/each}
      </nav>
    </div>
    <ThemeSwitcherToggle />
  </header>

  <main class="min-h-0 flex-1 overflow-auto p-6">
    {@render children()}
  </main>
</div>
