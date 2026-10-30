<script lang="ts">
    import { page } from '$app/state';
    import { CodeMapHost, provideCodeMapHost } from './codemap-host.svelte.js';

    let { children } = $props();

    // The sidebar lists the diagrams; every Code map lens shares this one map, and
    // every diagram's header shows its workspace numbers — read once, here, when no
    // lens has started the read.
    const host = provideCodeMapHost(new CodeMapHost(() => page.params.id ?? ''));
    $effect(() => {
        const map = host.state;
        if (!map.l0 && !map.loading && !map.error) void map.load(host.port);
    });
</script>

<div class="h-full min-h-0">{@render children()}</div>
