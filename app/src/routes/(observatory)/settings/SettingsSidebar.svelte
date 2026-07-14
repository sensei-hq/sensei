<script lang="ts">
    import { List } from '@rokkit/ui';
    import { Eyebrow } from '$lib/components';
    import { buildNavItems, resolveActiveHref } from './settings-nav';

    interface Props {
        /** Current route, passed from the layout (keeps this component pure). */
        pathname: string;
    }
    let { pathname }: Props = $props();

    const items = buildNavItems();
    const activeHref = $derived(resolveActiveHref(pathname));

    // List's default rendering handles icon + label + badge. Only remap the two
    // keys that differ from defaults: kanji → icon, text → label. href / value /
    // children / type keep their default field names.
    const fields = { icon: 'kanji', label: 'text' };
</script>

<aside
    data-component="settings-sidebar"
    class="w-[220px] shrink-0 flex flex-col gap-4 overflow-auto border-r border-paper-edge bg-paper px-3 py-5"
>
    <div class="px-2">
        <Eyebrow>Settings</Eyebrow>
    </div>

    <!-- rokkit List default item padding-block (density-spacing-xs, 4px) renders
         rows almost flush. The mockup sidebars use py-2 (8px) rows — match that
         by bumping the leaf/group vertical padding on THIS List only via the
         documented `class` lever (UnoCSS arbitrary variants land on the root
         <nav data-list>, scoping the change to the settings rail; the
         observatory rail, menus, and selects keep their default density). -->
    <List
        {items}
        {fields}
        value={activeHref}
        collapsible={false}
        class="[&_[data-list-item]]:py-2 [&_[data-list-group]]:py-2"
    />
</aside>
