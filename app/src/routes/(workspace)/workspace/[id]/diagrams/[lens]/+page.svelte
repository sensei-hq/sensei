<script lang="ts">
    import { untrack } from 'svelte';
    import { PageHeader, Screen } from '$lib/components';
    import { CodeMap } from '@sensei/codemap';
    import { codeMapHost } from '../codemap-host.svelte.js';

    let { data } = $props();

    const host = codeMapHost();
    // The route names the lens; the map follows it. Untracked, so drilling into a
    // module does not re-run the switch and climb back out.
    $effect(() => {
        const [map, lens] = [host.state, data.lens];
        untrack(() => void map.showLens(host.port, lens));
    });
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji={data.view.kanji} icon={data.view.icon} title={data.view.label} description={data.view.question} />
    {/snippet}
    <div class="p-4" data-testid="codemap-screen" data-lens={data.lens}>
        {#key data.workspaceId}
            <CodeMap state={host.state} port={host.port} />
        {/key}
    </div>
</Screen>
