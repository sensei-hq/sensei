<script lang="ts">
    import { List } from '@rokkit/ui';
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

    const items = $derived(buildNavItems({ projectCount }));
    const activeHref = $derived(resolveActiveHref(pathname));

    // List's default rendering handles icon + label + badge. Only remap the two
    // keys that differ from defaults: kanji → icon, text → label. href / value /
    // badge / children / type keep their default field names.
    const fields = { icon: 'kanji', label: 'text' };
</script>

<aside
    data-component="observatory-sidebar"
    class="w-[220px] shrink-0 flex flex-col gap-4 overflow-auto border-r border-paper-edge bg-paper px-3 py-5"
>
    <div class="px-1">
        <Wordmark />
    </div>

    <div class="px-2">
        <Eyebrow>Observatory</Eyebrow>
    </div>

    <List {items} {fields} value={activeHref} collapsible={false} />

    <div class="flex-1"></div>

    <div class="border-t border-paper-edge pt-2 text-xs leading-relaxed">
        <span class="font-mono text-ink-mute">daemon · running</span><br />
        <span class="font-mono text-ink-faint">port {port}</span>
    </div>
</aside>
