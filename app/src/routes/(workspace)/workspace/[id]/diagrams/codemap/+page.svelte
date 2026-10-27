<script lang="ts">
    import { PageHeader, Screen } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { CodeMap, CodeMapState } from '@sensei/codemap';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { codemapPort } from '../codemap-port.js';

    let { data } = $props();

    const view = DIAGRAM_VIEWS.find((v) => v.id === 'codemap')!;
    const port = codemapPort(senseiApi(appState.port), () => data.workspaceId);
    // One state per workspace: a new workspace opens on its own modules.
    const map = $derived.by(() => {
        void data.workspaceId;
        return new CodeMapState();
    });
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji={view.kanji} icon={view.icon} title="Code map" description={view.question} />
    {/snippet}
    <div class="p-4" data-testid="codemap-screen">
        {#key data.workspaceId}
            <CodeMap state={map} {port} />
        {/key}
    </div>
</Screen>
