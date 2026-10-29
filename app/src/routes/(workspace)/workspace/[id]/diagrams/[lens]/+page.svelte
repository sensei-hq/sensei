<script lang="ts">
    import { untrack } from 'svelte';
    import { Screen } from '$lib/components';
    import { CodeMap } from '@sensei/codemap';
    import { codeMapHost } from '../codemap-host.svelte.js';
    import { DiagramHeaderState } from '../diagram-header.svelte.js';
    import DiagramHeader from '../DiagramHeader.svelte';

    let { data } = $props();

    const host = codeMapHost();
    const about = new DiagramHeaderState();
    // The route names the lens; the map follows it. Untracked, so drilling into a
    // module does not re-run the switch and climb back out.
    $effect(() => {
        const [map, lens] = [host.state, data.lens];
        untrack(() => void map.showLens(host.port, lens));
    });
</script>

<!-- Each diagram is its own screen; the map they share lives in the layout. -->
{#key data.lens}
<Screen>
    {#snippet header()}
    <DiagramHeader view={data.view} {about} />
    {/snippet}
    <div class="p-4" data-testid="codemap-screen" data-lens={data.lens}>
        {#key data.workspaceId}
            <CodeMap state={host.state} port={host.port} />
        {/key}
    </div>
</Screen>
{/key}
