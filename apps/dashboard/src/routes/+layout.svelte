<script lang="ts">
  import '../app.css';
  let { children } = $props();

  const navItems = [
    { href: '/', label: 'Dashboard' },
    { href: '/stats', label: 'Stats' },
    { href: '/repos', label: 'Repos' },
    { href: '/libraries', label: 'Libraries' },
    { href: '/references', label: 'References' },
    { href: '/benchmarks', label: 'Benchmarks' },
  ];
</script>

<svelte:head>
  <title>Sensei</title>
</svelte:head>

<div class="flex h-screen w-screen overflow-hidden bg-surface-z1 text-surface-z8">
  <nav class="w-48 flex-shrink-0 flex flex-col gap-1 p-4 bg-surface-z2 border-r border-surface-z3">
    <div class="text-lg font-semibold text-primary-z6 mb-4 px-2">sensei</div>
    {#each navItems as item}
      <a href={item.href} class="px-2 py-1.5 rounded text-sm text-surface-z6 hover:bg-surface-z3 hover:text-surface-z8">
        {item.label}
      </a>
    {/each}
  </nav>
  <main class="flex-1 overflow-auto p-6">
    {@render children()}
  </main>
</div>
