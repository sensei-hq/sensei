<script lang="ts">
    import { page } from '$app/state';
    import SettingsSidebar from './SettingsSidebar.svelte';
    import { appState } from '$lib/appstate.svelte.js';
    import { glyphMode } from '$lib/glyphs';

    let { children } = $props();
</script>

<div class="flex h-full min-h-0" data-component="settings-shell">
    <SettingsSidebar pathname={page.url.pathname} glyphs={glyphMode(appState.config)} />
    <!-- Each settings page is a Screen and owns its one scroll (#293). -->
    <main data-component="settings-main" class="flex-1 min-w-0 min-h-0 overflow-hidden">
        {@render children()}
    </main>
</div>
