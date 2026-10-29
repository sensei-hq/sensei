<script lang="ts">
    import { page } from '$app/state';
    import { CodeMapHost, provideCodeMapHost } from './codemap-host.svelte.js';

    let { children } = $props();

    // The sidebar lists the diagrams; every Code map lens shares this one map.
    provideCodeMapHost(new CodeMapHost(() => page.params.id ?? ''));
</script>

<div class="h-full min-h-0">{@render children()}</div>
