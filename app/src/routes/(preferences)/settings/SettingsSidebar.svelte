<script lang="ts">
    import { Eyebrow, Rail } from '$lib/components';
    import { buildNavItems, resolveActiveHref } from './settings-nav';
    import type { GlyphMode } from '$lib/glyphs';

    interface Props {
        /** Current route, passed from the layout (keeps this component pure). */
        pathname: string;
        /** Kanji or icons (#133); the layout reads it from the preference. */
        glyphs?: GlyphMode;
    }
    let { pathname, glyphs = 'kanji' }: Props = $props();

    const items = buildNavItems();
    const activeHref = $derived(resolveActiveHref(pathname));
</script>

<Rail component="settings-sidebar" label="Settings" {items} value={activeHref} {glyphs}>
    {#snippet header()}
        <div class="px-2">
            <Eyebrow>Settings</Eyebrow>
        </div>
    {/snippet}
</Rail>
