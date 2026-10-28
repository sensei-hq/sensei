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
    class="w-[220px] shrink-0 flex flex-col gap-4 overflow-auto border-r border-paper-edge bg-paper px-3 py-5"
>
    {@render header?.()}
    <List {items} {fields} {value} collapsible={false} />
    {#if footer}
        <div class="flex-1"></div>
        {@render footer()}
    {/if}
</aside>
