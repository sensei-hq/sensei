<script lang="ts">
    import { afterNavigate } from '$app/navigation';
    import RailMenuButton from '$lib/components/RailMenuButton.svelte';
    import { railDrawer } from '$lib/components/rail-drawer.svelte.js';
    import { page } from '$app/state';
    import SettingsSidebar from './SettingsSidebar.svelte';
    import { appState } from '$lib/appstate.svelte.js';
    import { glyphMode } from '$lib/glyphs';

    let { children } = $props();
    // A navigation closes the Rail's drawer (#420): left open it would cover the page.
    afterNavigate(() => railDrawer.navigated());
</script>

<div class="relative flex h-full min-h-0" data-component="settings-shell">
    <!-- Below md the Rail is a drawer (#420): the menu that opens it. -->
    <div class="md:hidden absolute top-2 left-2 z-20"><RailMenuButton /></div>
    <SettingsSidebar pathname={page.url.pathname} glyphs={glyphMode(appState.config)} />
    <!-- Each settings page is a Screen and owns its one scroll (#293). -->
    <main data-component="settings-main" class="flex-1 min-w-0 min-h-0 overflow-hidden">
        {@render children()}
    </main>
</div>
