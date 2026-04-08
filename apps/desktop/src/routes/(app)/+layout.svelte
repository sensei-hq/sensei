<script lang="ts">
  import { page } from '$app/stores';
  import { ThemeSwitcherToggle } from '@rokkit/app';

  let { children } = $props();

  const navItems = [
    { icon: 'i-solar-planets-bold-duotone',          label: 'Projects',   href: '/projects'  },
    { icon: 'i-solar-lightbulb-bold-duotone',        label: 'Ideas',      href: '/ideas'     },
    { icon: 'i-solar-history-bold-duotone',          label: 'Sessions',   href: '/sessions'  },
    { icon: 'i-solar-graph-up-bold-duotone',         label: 'Graph',      href: '/graph'     },
  ];

  const bottomItems = [
    { icon: 'i-solar-settings-minimalistic-bold-duotone', label: 'Settings', href: '/settings' },
  ];
</script>

<div class="flex h-screen overflow-hidden select-none bg-surface-z1">

  <!-- ══ SIDEBAR ════════════════════════════════════════════════════════ -->
  <!-- Traffic light inset on macOS: title bar is ~28px, we pad 12px extra -->
  <aside class="
    flex w-52 shrink-0 flex-col border-r border-surface-z0/50
    sidebar-vibrancy
  ">

    <!-- Traffic-light safe area + app logo -->
    <div class="drag-region flex items-end justify-between px-4 pb-3 pt-9">
      <div class="no-drag flex items-center gap-2">
        <div class="flex h-6 w-6 items-center justify-center rounded-lg bg-primary-z6 text-xs font-bold text-white">⬡</div>
        <span class="text-sm font-bold tracking-tight text-surface-z8">sensei</span>
      </div>
    </div>

    <!-- Nav -->
    <nav class="flex-1 space-y-0.5 px-2 py-2">
      <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Workspace</p>
      {#each navItems as item}
        <a
          href={item.href}
          class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors no-drag
                 {$page.url.pathname.startsWith(item.href)
                   ? 'bg-primary-z2 font-medium text-primary-z7'
                   : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
        >
          <span class="text-base {item.icon}"></span>
          {item.label}
        </a>
      {/each}

      <div class="pt-4">
        <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Toolbox</p>
        {#each [
          { icon: 'i-solar-cpu-bold-duotone',       label: 'Claude Code' },
          { icon: 'i-solar-library-bold-duotone',   label: 'Libraries'   },
          { icon: 'i-solar-chat-square-bold-duotone', label: 'Prompt Bar'  },
        ] as item}
          <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors no-drag hover:bg-surface-z3/60 hover:text-surface-z7">
            <span class="text-base {item.icon}"></span>
            {item.label}
          </button>
        {/each}
      </div>

      <!-- Mockup routes (dev only) -->
      <div class="pt-4">
        <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Mockups</p>
        {#each [
          { label: 'Three-pane view', href: '/mockups/a' },
          { label: 'Card workspace',  href: '/mockups/b' },
        ] as item}
          <a
            href={item.href}
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors no-drag
                   {$page.url.pathname === item.href
                     ? 'bg-primary-z2 font-medium text-primary-z7'
                     : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
          >
            <span class="h-1.5 w-1.5 rounded-full bg-surface-z4"></span>
            {item.label}
          </a>
        {/each}
      </div>
    </nav>

    <!-- Bottom -->
    <div class="border-t border-surface-z0/50 px-3 py-2.5 space-y-0.5">
      <div class="flex items-center justify-between px-2.5 py-1.5">
        <span class="text-xs text-surface-z4">Theme</span>
        <div class="no-drag"><ThemeSwitcherToggle /></div>
      </div>
      {#each bottomItems as item}
        <a
          href={item.href}
          class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors no-drag hover:bg-surface-z3/60 hover:text-surface-z7"
        >
          <span class="text-base {item.icon}"></span>
          {item.label}
        </a>
      {/each}
    </div>
  </aside>

  <!-- ══ MAIN COLUMN ════════════════════════════════════════════════════ -->
  <div class="flex min-w-0 flex-1 flex-col bg-surface-z1 overflow-hidden">
    <!-- Title bar drag region (traffic lights are on the sidebar side) -->
    <div class="drag-region h-7 shrink-0"></div>

    <!-- Content -->
    <main class="flex-1 overflow-y-auto min-h-0">
      {@render children()}
    </main>
  </div>

</div>
