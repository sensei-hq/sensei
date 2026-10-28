<script lang="ts">
    import { railIcon } from "../settings-nav";
  import { onMount } from 'svelte';
  import { PageHeader, Screen } from '$lib/components';
  import WorkspacesSection from '$lib/components/settings/WorkspacesSection.svelte';
  import { WorkspacesSectionState } from '$lib/components/settings/workspaces-section-state.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';

  // The page owns the state so the header and the search can sit in the
  // screen's fixed header above the list (#271, #293).
  const workspaces = new WorkspacesSectionState();
  const api = senseiApi(appState.port);
  onMount(() => {
    void workspaces.load(api);
  });
</script>

<Screen>
  {#snippet header()}
    <div class="bg-paper" data-testid="settings-workspaces-header">
      <PageHeader kanji="組" icon={railIcon("/settings/workspaces")} eyebrow="Settings" title="Workspaces" />
      <div class="max-w-[820px] mx-auto px-12 pb-3">
        <input
          type="search"
          bind:value={workspaces.query}
          placeholder="Search workspaces or repositories"
          aria-label="Search workspaces or repositories"
          class="w-full text-sm px-3 py-2 rounded-md border border-paper-edge bg-paper-soft text-ink
                 placeholder:text-ink-faint outline-none focus:border-ink-mute"
        />
      </div>
    </div>
  {/snippet}
  <div class="max-w-[820px] mx-auto px-12 pt-4 pb-16" data-testid="settings-workspaces">
    <WorkspacesSection state={workspaces} {api} />
  </div>
</Screen>
