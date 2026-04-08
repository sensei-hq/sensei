<script lang="ts">
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  let showCreateForm = $state(false);
  let orgName        = $state('');
  let orgSlug        = $state('');
  let creating       = $state(false);
  let createError    = $state('');

  $effect(() => {
    orgSlug = orgName.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
  });

  async function createOrg() {
    if (!orgName.trim()) return;
    creating = true;
    createError = '';
    try {
      const res = await fetch('/api/orgs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: orgName.trim(), slug: orgSlug }),
      });
      if (!res.ok) {
        const body = await res.json();
        createError = body.message ?? 'Failed to create organization';
        return;
      }
      const org = await res.json();
      window.location.href = `/orgs/${org.slug}`;
    } finally {
      creating = false;
    }
  }

  const planBadgeClass = (plan: string) => {
    if (plan === 'team')       return 'badge-plan badge-plan-team';
    if (plan === 'enterprise') return 'badge-plan badge-plan-enterprise';
    return 'badge-plan badge-plan-free';
  };
</script>

<div class="flex items-center justify-between mb-6">
  <div>
    <h1 class="text-2xl font-semibold text-surface-z8">Organizations</h1>
    <p class="text-sm text-surface-z5 mt-0.5">{data.orgs.length} org{data.orgs.length !== 1 ? 's' : ''}</p>
  </div>
  <button
    onclick={() => { showCreateForm = !showCreateForm; }}
    class="px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z7 hover:border-primary-z5 hover:bg-surface-z2 transition-colors"
  >
    {showCreateForm ? 'Cancel' : '+ New organization'}
  </button>
</div>

{#if showCreateForm}
  <div class="mb-6 rounded-xl border border-surface-z3 bg-surface-z2 p-5">
    <h2 class="mb-4 text-sm font-semibold text-surface-z8">Create organization</h2>
    <div class="flex flex-col gap-3 max-w-sm">
      <div>
        <label class="mb-1 block text-xs font-medium text-surface-z6" for="new-org-name">Name</label>
        <input
          id="new-org-name"
          type="text"
          bind:value={orgName}
          placeholder="Acme Corp"
          class="w-full rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2 text-sm text-surface-z8 placeholder-surface-z4 focus:border-primary-z5 focus:outline-none"
        />
      </div>
      <div>
        <label class="mb-1 block text-xs font-medium text-surface-z6" for="new-org-slug">Slug</label>
        <input
          id="new-org-slug"
          type="text"
          bind:value={orgSlug}
          placeholder="acme-corp"
          class="w-full rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2 font-mono text-sm text-surface-z8 placeholder-surface-z4 focus:border-primary-z5 focus:outline-none"
        />
        <p class="mt-1 text-xs text-surface-z4">Used in URLs. Letters, numbers, hyphens only.</p>
      </div>
      {#if createError}
        <p class="text-xs text-error-z6">{createError}</p>
      {/if}
      <button
        onclick={createOrg}
        disabled={creating || !orgName.trim()}
        class="rounded-lg bg-primary-z6 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-z7 disabled:opacity-50 transition-colors self-start"
      >
        {creating ? 'Creating…' : 'Create organization'}
      </button>
    </div>
  </div>
{/if}

{#if data.orgs.length === 0}
  <div class="flex flex-col items-center justify-center py-16 text-center gap-3">
    <p class="text-surface-z5">No organizations yet.</p>
    <p class="text-sm text-surface-z4">Create one above or ask your team admin to invite you.</p>
  </div>
{:else}
  <div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(280px, 1fr))">
    {#each (data.orgs as { id: string; name: string; slug: string; plan: string; role: string }[]) as org}
      <a
        href="/orgs/{org.slug}"
        class="flex flex-col p-4 rounded-lg border border-surface-z3 bg-surface-z1 no-underline transition-colors hover:border-primary-z5 hover:bg-surface-z2 min-h-24"
      >
        <div class="flex items-start justify-between gap-2 mb-2">
          <span class="font-semibold text-surface-z8 text-base leading-snug">{org.name}</span>
          <span class={planBadgeClass(org.plan ?? '')}>{org.plan}</span>
        </div>
        <div class="mt-auto flex items-center gap-2 text-xs text-surface-z5">
          <span class="font-mono">{org.slug}</span>
          <span class="text-surface-z3">·</span>
          <span class="capitalize">{org.role}</span>
        </div>
      </a>
    {/each}
  </div>
{/if}

<style>
  .badge-plan {
    padding: 2px 8px;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: capitalize;
    flex-shrink: 0;
  }
  .badge-plan-free       { background: #f1f5f9; color: #475569; }
  .badge-plan-team       { background: #dbeafe; color: #1d4ed8; }
  .badge-plan-enterprise { background: #ede9fe; color: #5b21b6; }
</style>
