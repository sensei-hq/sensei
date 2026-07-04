<script lang="ts">
    import { page } from '$app/state';
    import SettingsSidebar from './SettingsSidebar.svelte';

    let { children } = $props();
</script>

<div class="flex h-full min-h-0" data-component="settings-shell">
    <SettingsSidebar pathname={page.url.pathname} />
    <div class="flex-1 overflow-auto">
        {@render children()}
    </div>
</div>
