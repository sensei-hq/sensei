<script lang="ts">
    import { List } from '@rokkit/ui';
    import type { ProxyItem } from '@rokkit/states';
    import { Eyebrow, Wordmark } from '$lib/components';
    import { buildNavItems, resolveActiveHref } from './observatory-nav';

    interface Props {
        /** Daemon port, shown in the footer. */
        port: number;
        /** Current route, passed from the layout (keeps this component pure). */
        pathname: string;
        /** Cached project count for the Projects badge; omit until loaded. */
        projectCount?: number;
    }
    let { port, pathname, projectCount }: Props = $props();

    // Focus tames the rail to anchors + "Needs you" — just what needs a decision.
    let focus = $state(false);

    const items = $derived(buildNavItems({ focus, projectCount }));
    const activeHref = $derived(resolveActiveHref(pathname));

    // List reads these keys off each entry; `value` mirrors `href` so the
    // current route lights up via List's value-match. `type` enables separators.
    const fields = {
        value: 'value',
        href: 'href',
        text: 'text',
        badge: 'badge',
        children: 'children',
        type: 'type',
    };

    const SEGMENTS = [
        { value: false, label: 'All' },
        { value: true, label: 'Focus' },
    ];
</script>

<aside
    data-component="observatory-sidebar"
    class="w-[220px] shrink-0 flex flex-col gap-4 overflow-auto border-r border-paper-edge bg-paper px-3 py-5"
>
    <div class="px-1">
        <Wordmark />
    </div>

    <div>
        <div class="flex items-center gap-2 px-2 pb-2">
            <Eyebrow>Observatory</Eyebrow>
            <span class="flex-1"></span>
            <div class="flex rounded bg-paper-mute p-0.5">
                {#each SEGMENTS as seg (seg.label)}
                    {@const on = seg.value === focus}
                    <button
                        type="button"
                        class="rounded-sm px-1.5 py-0.5 text-xs {on
                            ? 'bg-paper text-ink'
                            : 'text-ink-mute'}"
                        aria-pressed={on}
                        onclick={() => (focus = seg.value)}
                    >{seg.label}</button>
                {/each}
            </div>
        </div>

        <List {items} {fields} value={activeHref} collapsible={false} class="gap-px">
            {#snippet itemContent(proxy: ProxyItem)}
                {@const active = proxy.value === activeHref}
                <span
                    class="kanji w-3.5 shrink-0 text-center text-sm {active
                        ? 'text-accent'
                        : 'text-ink-mute'}">{proxy.get('kanji')}</span
                >
                <span class="inline-flex flex-1 items-center gap-1.5">
                    {proxy.get('text')}
                    {#if proxy.get('alert')}
                        <span
                            class="h-1.5 w-1.5 shrink-0 rounded-full bg-danger"
                            title="needs attention"
                        ></span>
                    {/if}
                </span>
                {#if proxy.get('badge') != null}
                    <span class="font-mono text-xs text-ink-mute">{proxy.get('badge')}</span>
                {/if}
            {/snippet}

            {#snippet groupContent(proxy: ProxyItem)}
                <span class="text-ink-faint">{proxy.get('text')}</span>
            {/snippet}
        </List>
    </div>

    <div class="flex-1"></div>

    <div class="border-t border-paper-edge pt-2 text-xs leading-relaxed">
        <span class="font-mono text-ink-mute">daemon · running</span><br />
        <span class="font-mono text-ink-faint">port {port}</span>
    </div>
</aside>

<style>
    /* Geometry only. The rokkit List owns the <a data-list-item> wrappers, so a
       utility class can't reach them to round the hover/active wash — this is the
       one case utilities can't cover. Colour (hover/active/at-rest) comes from the
       zen-sumi list theme; the rail sits on `bg-paper` so the theme's at-rest
       active (paper-soft) and hover (paper-mute) washes read clearly. */
    aside :global([data-list-item]) {
        border-radius: 6px;
    }
</style>
