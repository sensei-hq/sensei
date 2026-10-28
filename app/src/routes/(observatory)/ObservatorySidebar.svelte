<script lang="ts">
    import { Eyebrow, Rail, Wordmark } from '$lib/components';
    import { buildNavItems, resolveActiveHref } from './observatory-nav';
    import type { GlyphMode } from '$lib/glyphs';

    interface Props {
        /** Daemon port, shown in the footer. */
        port: number;
        /** Current route, passed from the layout (keeps this component pure). */
        pathname: string;
        /** Cached workspace count for the Workspaces badge; omit until loaded. */
        workspaceCount?: number;
        /** Kanji or icons (#133); the layout reads it from the preference. */
        glyphs?: GlyphMode;
    }
    let { port, pathname, workspaceCount, glyphs = 'kanji' }: Props = $props();

    const items = $derived(buildNavItems({ workspaceCount }));
    const activeHref = $derived(resolveActiveHref(pathname));
</script>

<Rail component="observatory-sidebar" label="Observatory" {items} value={activeHref} {glyphs}>
    {#snippet header()}
        <div class="px-1">
            <Wordmark />
        </div>
        <div class="px-2">
            <Eyebrow>Observatory</Eyebrow>
        </div>
    {/snippet}
    {#snippet footer()}
        <div class="border-t border-paper-edge pt-2 text-xs leading-relaxed">
            <span class="font-mono text-ink-soft">daemon · running</span><br />
            <span class="font-mono text-ink-faint">port {port}</span>
        </div>
    {/snippet}
</Rail>
