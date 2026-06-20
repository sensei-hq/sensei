<script lang="ts">
    import { page } from '$app/state';
    import { Eyebrow } from '$lib/components';

    interface Props {
        port: number;
    }
    let { port }: Props = $props();

    const NAV_ITEMS = [
        { href: '/',            kanji: '家', label: 'Today' },
        { href: '/projects',    kanji: '場', label: 'Projects' },
        { href: '/sessions',    kanji: '刻', label: 'Sessions' },
        { href: '/learnings',   kanji: '憶', label: 'Learnings' },
        { href: '/insights',    kanji: '學', label: 'Insights' },
        { href: '/libraries',   kanji: '書', label: 'Libraries' },
        { href: '/instruments', kanji: '具', label: 'Instruments' },
    ];

    const BOTTOM_ITEMS = [
        { href: '/knowledge-sources', kanji: '連', label: 'Sources' },
        { href: '/logs',     kanji: '録', label: 'Logs' },
        { href: '/settings', kanji: '設', label: 'Settings' },
    ];

    let collapsed = $state(false);
    const widthClass = $derived(collapsed ? 'w-[52px]' : 'w-[220px]');

    function isActive(href: string): boolean {
        return (
            page.url.pathname === href ||
            page.url.pathname.startsWith(href + '/')
        );
    }
</script>

<aside
    data-component="observatory-sidebar"
    class="border-r border-paper-mute px-3.5 py-6 bg-paper-mute flex flex-col gap-5 overflow-auto transition-[width] duration {widthClass}"
>
    <div class="flex items-baseline gap-2 px-1.5">
        <span class="kanji text-xl text-accent">先</span>
        {#if !collapsed}
            <span class="font-display text-base">Sensei</span>
            <button
                class="collapse-btn ml-auto bg-none border-none text-ink-soft cursor-pointer text-sm px-1.5 py-0.5 rounded-md"
                onclick={() => (collapsed = true)}
                aria-label="Collapse sidebar"
            >‹</button>
        {/if}
    </div>

    {#snippet navItem(item: typeof NAV_ITEMS[number], isCollapsed: boolean)}
        {@const active = isActive(item.href)}
        <a
            href={item.href}
            class="nav-item flex items-center py-2 rounded-md text-sm text-ink-mute no-underline transition-colors duration-fast hover:bg-paper-mute"
            class:justify-center={isCollapsed}
            class:gap-2.5={!isCollapsed}
            class:px-2.5={!isCollapsed}
            class:active
            title={isCollapsed ? item.label : undefined}
        >
            <span
                class="kanji text-sm w-3.5 text-ink-soft"
                class:nav-kanji-active={active}
            >{item.kanji}</span>
            {#if !isCollapsed}
                <span>{item.label}</span>
            {/if}
        </a>
    {/snippet}

    {#if collapsed}
        <nav class="flex flex-col gap-px">
            {#each NAV_ITEMS as item (item.href)}
                {@render navItem(item, true)}
            {/each}
        </nav>

        <div class="mt-auto pt-2.5 border-t border-paper-mute">
            <button
                class="collapse-btn bg-none border-none text-ink-soft cursor-pointer text-sm px-1.5 py-0.5 rounded-md"
                onclick={() => (collapsed = false)}
                aria-label="Expand sidebar"
            >›</button>
        </div>
    {:else}
        <div class="flex flex-col gap-0.5">
            <p class="px-2.5 pb-2 m-0"><Eyebrow>Observatory</Eyebrow></p>
            <nav class="flex flex-col gap-px">
                {#each NAV_ITEMS as item (item.href)}
                    {@render navItem(item, false)}
                {/each}
            </nav>
        </div>

        <div class="flex flex-col gap-0.5 mt-auto">
            <nav class="flex flex-col gap-px">
                {#each BOTTOM_ITEMS as item (item.href)}
                    {@render navItem(item, false)}
                {/each}
            </nav>
        </div>

        <div class="pt-2.5 border-t border-paper-mute">
            <span class="font-mono text-xs text-ink-soft">daemon · port {port}</span>
        </div>
    {/if}
</aside>

<style>
    /* Active nav state styles — kept as scoped CSS against class:active
       and class:nav-kanji-active bindings rather than Tailwind class:*
       chains (color + bg on the same property conflict, similar to the
       WizardRail rationale). */
    .nav-item.active {
        background: var(--paper-mute);
        color: var(--ink);
    }
    .nav-kanji-active {
        color: var(--accent);
    }
    .collapse-btn:hover {
        background: var(--paper-mute);
        color: var(--ink-mute);
    }
</style>
