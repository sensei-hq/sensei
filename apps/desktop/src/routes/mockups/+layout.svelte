<script lang="ts">
  import { page } from '$app/stores';
  import { ThemeSwitcherToggle } from '@rokkit/app';
  let { children } = $props();
</script>

<div class="flex h-screen overflow-hidden select-none bg-surface-z1">

  <!-- Narrow sidebar — just logo + nav for mockup context-switching -->
  <aside class="flex w-48 shrink-0 flex-col border-r border-surface-z0/50 sidebar-vibrancy">
    <div class="drag-region flex items-end px-4 pb-3 pt-9">
      <div class="no-drag flex items-center gap-2">
        <div class="flex h-6 w-6 items-center justify-center rounded-lg bg-primary-z6 text-xs font-bold text-white">⬡</div>
        <span class="text-sm font-bold tracking-tight text-surface-z8">sensei</span>
        <span class="rounded bg-warning-z2 px-1.5 py-0.5 text-[9px] font-semibold text-warning-z7">mockup</span>
      </div>
    </div>

    <nav class="flex-1 space-y-0.5 px-2 py-3">
      <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Variants</p>
      {#each [
        { href: '/mockups/a', label: 'Three-pane view',   desc: 'List → Pipeline → Detail'  },
        { href: '/mockups/b', label: 'Card workspace',    desc: 'Grid + Graph panel'         },
      ] as item}
        <a
          href={item.href}
          class="no-drag block rounded-xl px-3 py-2.5 transition-colors
                 {$page.url.pathname === item.href ? 'bg-primary-z2' : 'hover:bg-surface-z2/80'}"
        >
          <p class="text-xs font-medium {$page.url.pathname === item.href ? 'text-primary-z8' : 'text-surface-z7'}">{item.label}</p>
          <p class="text-[10px] text-surface-z4">{item.desc}</p>
        </a>
      {/each}

      <div class="pt-4">
        <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Back</p>
        <a
          href="/projects"
          class="no-drag flex items-center gap-2 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z2/80 hover:text-surface-z7"
        >
          <span class="i-solar-arrow-left-bold-duotone text-sm"></span>
          App
        </a>
      </div>
    </nav>

    <div class="border-t border-surface-z0/50 px-3 py-2.5">
      <div class="flex items-center justify-between px-2.5 py-1.5">
        <span class="text-xs text-surface-z4">Theme</span>
        <div class="no-drag"><ThemeSwitcherToggle /></div>
      </div>
    </div>
  </aside>

  <!-- Main -->
  <div class="flex min-w-0 flex-1 flex-col bg-surface-z1 overflow-hidden">
    <div class="drag-region h-7 shrink-0"></div>
    <main class="flex-1 overflow-hidden min-h-0">
      {@render children()}
    </main>
  </div>

</div>
