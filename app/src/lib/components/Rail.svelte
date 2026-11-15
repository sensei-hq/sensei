<script lang="ts">
    import type { Snippet } from 'svelte';
    import { List } from '@rokkit/ui';
    import { glyphFields, type GlyphMode } from '$lib/glyphs';

    // The one sidebar (user, 2026-10-28): the main window's, the settings
    // window's and the workspace window's are this component, so they share one
    // frame, one rokkit List and one glyph rule — and cannot drift apart.
    interface Props {
        /** `data-component` hook for each sidebar's own specs and e2e. */
        component: string;
        /** Accessible name of the navigation. */
        label: string;
        /** The List's items: groups of links, kanji + icon + text + href. */
        items: unknown[];
        /** The current entry's href. */
        value: string;
        /** Kanji or icons (#133). */
        glyphs?: GlyphMode;
        header?: Snippet;
        footer?: Snippet;
    }
    let { component, label, items, value, glyphs = 'kanji', header, footer }: Props = $props();

    const fields = $derived(glyphFields(glyphs));
</script>

<aside
    data-component={component}
    data-rail
    aria-label={label}
    class="w-[220px] shrink-0 flex flex-col gap-4 overflow-hidden border-r border-paper-edge bg-paper px-3 py-5"
>
    <!-- The header and footer hold their place; only the nav list scrolls. -->
    {#if header}<div class="shrink-0">{@render header()}</div>{/if}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="flex-1 min-h-0 overflow-y-auto -mx-1 px-1" tabindex="0" data-rail-nav>
        <List {items} {fields} {value} collapsible={false} />
    </div>
    {#if footer}<div class="shrink-0">{@render footer()}</div>{/if}
</aside>
