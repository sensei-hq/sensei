<script lang="ts">
    import { List } from '@rokkit/ui';
    import { Eyebrow } from '$lib/components';
    import { buildNavItems, resolveActiveHref } from './settings-nav';
    import { glyphFields, type GlyphMode } from '$lib/glyphs';

    interface Props {
        /** Current route, passed from the layout (keeps this component pure). */
        pathname: string;
        /** Kanji or icons (#133); the layout reads it from the preference. */
        glyphs?: GlyphMode;
    }
    let { pathname, glyphs = 'kanji' }: Props = $props();

    const items = buildNavItems();
    const activeHref = $derived(resolveActiveHref(pathname));

    // `glyphFields` says which property fills the List's icon slot (#133).
    const fields = $derived(glyphFields(glyphs));
</script>

<aside
    data-component="settings-sidebar"
    class="w-[220px] shrink-0 flex flex-col gap-4 overflow-auto border-r border-paper-edge bg-paper px-3 py-5"
>
    <div class="px-2">
        <Eyebrow>Settings</Eyebrow>
    </div>

    <!-- The main sidebar's List, at its density (user, 2026-10-27). -->
    <List {items} {fields} value={activeHref} collapsible={false} />
</aside>
