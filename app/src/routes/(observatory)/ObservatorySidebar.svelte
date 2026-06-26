<script lang="ts">
    import { List, Toggle } from '@rokkit/ui';
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

    // Field mapping for List's default rendering: kanji → icon, text → label,
    // plus href (→ <a>), value (active-route match), badge, children (groups),
    // type (separators). List handles icon + label + badge — no item/group snippets.
    const fields = {
        value: 'value',
        href: 'href',
        icon: 'kanji',
        label: 'text',
        badge: 'badge',
        children: 'children',
        type: 'type',
    };

    // All|Focus segmented control (rokkit Toggle); its value is the focus flag.
    // Toggle reads proxy.label → the default 'label' field, so key on `label`.
    const DENSITY = [
        { label: 'All', value: false },
        { label: 'Focus', value: true },
    ];
</script>

<aside
    data-component="observatory-sidebar"
    class="w-[220px] shrink-0 flex flex-col gap-4 overflow-auto border-r border-paper-edge bg-paper px-3 py-5"
>
    <div class="px-1">
        <Wordmark />
    </div>

    <div class="flex items-center gap-2 px-2">
        <Eyebrow>Observatory</Eyebrow>
        <span class="flex-1"></span>
        <Toggle options={DENSITY} bind:value={focus} size="sm" />
    </div>

    <List {items} {fields} value={activeHref} collapsible={false} />

    <div class="flex-1"></div>

    <div class="border-t border-paper-edge pt-2 text-xs leading-relaxed">
        <span class="font-mono text-ink-mute">daemon · running</span><br />
        <span class="font-mono text-ink-faint">port {port}</span>
    </div>
</aside>
