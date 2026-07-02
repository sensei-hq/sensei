<script lang="ts">
  import { getContext } from 'svelte';
  import { page } from '$app/state';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { ProjectListItem } from '$lib/types.js';

  interface ProjectCtx {
    readonly value: ProjectListItem | null;
    readonly loading: boolean;
  }
  const ctx = getContext<ProjectCtx>('project');

  const projectId = $derived(page.params.id ?? '');

  let editing = $state(false);
  let saving = $state(false);
  let saveError = $state<string | null>(null);

  // Local editable copy — flushed to the daemon on save. Read-only fields
  // (id, timestamps, stack derived from indexing) stay off this form.
  let form = $state({
    name: '',
    client: '',
    goal: '',
    preferred_acp: '',
    maturity: '',
  });

  // Hydrate the form from context when the project loads.
  $effect(() => {
    if (ctx.value) {
      form = {
        name: ctx.value.name ?? '',
        client: ctx.value.client ?? '',
        goal: ctx.value.goal ?? '',
        preferred_acp: ctx.value.preferred_acp ?? '',
        maturity: ctx.value.maturity ?? '',
      };
    }
  });

  async function save() {
    saving = true;
    saveError = null;
    const api = senseiApi(appState.port);
    try {
      // Skip empty strings so the patch stays minimal — the daemon merges the
      // JSONB patch into the existing settings row.
      const patch: Record<string, string> = {};
      for (const [k, v] of Object.entries(form)) {
        if (v && v.length > 0) patch[k] = v;
      }
      await api.updateProject(projectId, patch);
      editing = false;
    } catch (err: unknown) {
      saveError = err instanceof Error ? err.message : String(err);
    } finally {
      saving = false;
    }
  }

  function revert() {
    if (ctx.value) {
      form = {
        name: ctx.value.name ?? '',
        client: ctx.value.client ?? '',
        goal: ctx.value.goal ?? '',
        preferred_acp: ctx.value.preferred_acp ?? '',
        maturity: ctx.value.maturity ?? '',
      };
    }
    editing = false;
    saveError = null;
  }

  const project = $derived(ctx.value);
</script>

<div class="max-w-[720px] mx-auto px-12 py-10 pb-16">
  <div class="flex items-center justify-between mb-6">
    <h1 class="display text-xl font-normal m-0">About</h1>
    {#if !editing}
      <button
        class="px-3 py-1.5 rounded-md text-xs border border-paper-edge bg-transparent text-ink-soft cursor-pointer"
        onclick={() => (editing = true)}
        disabled={ctx.loading || !project}
      >Edit</button>
    {/if}
  </div>

  {#if ctx.loading}
    <p class="text-sm text-ink-soft">Loading…</p>
  {:else if !project}
    <p class="text-sm text-ink-soft">Project not found.</p>
  {:else}
    {#if !editing}
      <div class="grid grid-cols-[140px_1fr] gap-y-3 gap-x-4">
        {#each [['Name', project.name], ['Client', project.client ?? '—'], ['Goal', project.goal ?? '—'], ['Preferred assistant', project.preferred_acp ?? '—'], ['Maturity', project.maturity ?? '—']] as [label, value]}
          <span class="text-xs text-ink-mute uppercase tracking-wide">{label}</span>
          <span class="text-sm text-ink">{value}</span>
        {/each}
      </div>
    {:else}
      <form
        onsubmit={(e) => {
          e.preventDefault();
          save();
        }}
        class="flex flex-col gap-4"
      >
        {#each [
          { key: 'name', label: 'Name', placeholder: 'Project name' },
          { key: 'client', label: 'Client', placeholder: 'Client / organisation' },
          { key: 'goal', label: 'Goal', placeholder: 'Short goal or purpose' },
          { key: 'preferred_acp', label: 'Preferred assistant', placeholder: 'claude-code | zed | ...' },
          { key: 'maturity', label: 'Maturity', placeholder: 'spike | mvp | mature | archived' },
        ] as field}
          <label class="flex flex-col gap-1">
            <span class="text-xs text-ink-mute uppercase tracking-wide">{field.label}</span>
            <input
              type="text"
              placeholder={field.placeholder}
              class="px-3 py-2 border border-paper-edge rounded-md bg-paper text-ink text-sm outline-none"
              bind:value={form[field.key as keyof typeof form]}
            />
          </label>
        {/each}

        {#if saveError}
          <p class="text-xs text-danger m-0">Save failed: {saveError}</p>
        {/if}

        <div class="flex gap-2">
          <button
            type="submit"
            disabled={saving}
            class="px-4 py-2 rounded-md text-sm bg-primary text-on-primary border-none cursor-pointer"
          >{saving ? 'Saving…' : 'Save'}</button>
          <button
            type="button"
            onclick={revert}
            disabled={saving}
            class="px-4 py-2 rounded-md text-sm bg-transparent text-ink-soft border border-paper-edge cursor-pointer"
          >Cancel</button>
        </div>
      </form>
    {/if}
  {/if}
</div>
