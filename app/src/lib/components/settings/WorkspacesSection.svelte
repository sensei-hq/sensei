<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
  // Settings · Workspaces (#271). Presentation only: what to list, saving a role or
  // a rename, and the error states live in workspaces-section-state.svelte.ts.
  import { Select } from '@rokkit/ui';
  import { apiBase } from '$lib/api.js';
  import { glyphMode } from '$lib/glyphs';
  import { appState } from '$lib/appstate.svelte.js';
  import { ScreenState } from '$lib/components';
  import { ROLES } from '$lib/setup/types.js';
  import WorkspaceGlyph from '../../../routes/(observatory)/workspaces/WorkspaceGlyph.svelte';
  import { workspaceIcon } from '../../../routes/(observatory)/workspaces/buckets.js';
  import {
    derivedRoles,
    repositoriesOf,
    type WorkspacesApi,
    type WorkspacesSectionState,
    roleOptions,
  } from './workspaces-section-state.svelte.js';

  let { state, api }: { state: WorkspacesSectionState; api: WorkspacesApi } = $props();

  // Each role with its mark, kanji or icon (#133), so the picker shows the mark it applies.
  const glyphs = $derived(glyphMode(appState.config));
  const roleItems = $derived(roleOptions(glyphs));
  const roleById = new Map(ROLES.map((r) => [r.id, r]));
</script>

{#snippet card(workspace: (typeof state.workspaces)[number])}
  {@const repos = repositoriesOf(workspace)}
  <div class="rounded-lg p-4 border border-paper-edge bg-paper-mute" data-testid={`workspace-card-${workspace.id}`}>
    <div class="flex items-center gap-3">
      <div class="w-10 h-10 flex items-center justify-center rounded-full bg-paper-soft border border-paper-edge kanji text-2xl text-accent">
        <WorkspaceGlyph icon={workspaceIcon(workspace, apiBase(appState.port), glyphMode(appState.config))} />
      </div>
      <div class="min-w-0 flex-1">
        <input
          value={workspace.name}
          onchange={(e) => state.rename(api, workspace.id, (e.target as HTMLInputElement).value)}
          aria-label="Workspace name"
          class="display text-xl font-normal bg-transparent border-none outline-none w-full p-0 text-ink"
        />
        <div class="font-mono text-xs text-ink-mute mt-0.5" data-repo-count>
          {repos.length} {repos.length === 1 ? 'repository' : 'repositories'}
        </div>
      </div>
    </div>

    {#if repos.length > 0}
      <div class="grid grid-cols-1 md:grid-cols-2 gap-2 mt-3">
        {#each repos as repo (repo.id)}
          {@const derived = derivedRoles(workspace, repo.id)}
          <div class="flex items-center gap-2 p-2 rounded-md bg-paper-soft border border-paper-edge" data-repo={repo.name}>
            <div class="min-w-0 flex-1">
              <div class="text-sm text-ink truncate">{repo.name}</div>
              <div class="font-mono text-xs text-ink-faint truncate" title={repo.path}>{repo.path}</div>
              {#if derived.length > 0}
                <div class="flex flex-wrap gap-x-2 text-xs text-ink-soft mt-0.5" data-derived-roles={derived.join(' ')} title="Inferred from its packages' manifests">
                  {#each derived as id (id)}
                    {@const role = roleById.get(id)}
                    {#if role}
                      <span><Kanji char={role.kanji} icon={role.icon} size="inherit" tone="inherit" class="text-accent align-[-2px]" /> {role.label}</span>
                    {/if}
                  {/each}
                </div>
              {/if}
            </div>
            <Select
              size="sm"
              items={roleItems}
              value={repo.role ?? ''}
              onchange={(v) => state.setRole(api, repo.id, v as string)}
            />
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

{#if state.loading}
  <ScreenState status="loading" />
{:else if state.error}
  <ScreenState status="error" error={state.error} onretry={() => state.load(api)} />
{:else if state.workspaces.length === 0}
  <ScreenState
    status="empty"
    kanji="組"
    title="No workspaces yet"
    description="Add a folder to watch under Settings · Roots, and its repositories appear here."
  />
{:else}
  {#if state.roleError || state.renameError}
    <p class="text-xs text-danger mb-3" role="alert" data-save-error>{state.roleError ?? state.renameError}</p>
  {/if}

  <p class="text-sm text-ink-mute leading-normal mt-0 mb-4">
    A workspace's repositories are its git checkouts and subtrees. Set each one's role; it is saved
    as you choose it.
  </p>

  {#if state.placed.length === 0 && state.query.trim() !== ''}
    <p class="text-sm text-ink-soft" data-no-match>No workspace or repository matches “{state.query}”.</p>
  {/if}
  <div class="flex flex-col gap-3" data-placed>
    {#each state.placed as workspace (workspace.id)}
      {@render card(workspace)}
    {/each}
  </div>

  {#if state.unplaced.length > 0}
    <details class="mt-8" data-unplaced>
      <summary class="text-sm text-ink-mute cursor-pointer">
        {state.unplaced.length}
        {state.unplaced.length === 1 ? 'workspace has' : 'workspaces have'} no repository on this machine
      </summary>
      <p class="text-xs text-ink-soft mt-2 mb-3 leading-normal">
        Their checkouts were removed or never scanned here. Their history stays; repository metrics
        need a git checkout, so they have none until one is added.
      </p>
      <div class="flex flex-col gap-3">
        {#each state.unplaced as workspace (workspace.id)}
          {@render card(workspace)}
        {/each}
      </div>
    </details>
  {/if}
{/if}
