<script lang="ts">
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const planBadgeClass = (plan: string) => {
    if (plan === 'team') return 'badge-plan badge-plan-team';
    if (plan === 'enterprise') return 'badge-plan badge-plan-enterprise';
    return 'badge-plan badge-plan-free';
  };
</script>

<div class="flex items-center justify-between mb-6">
  <h1 class="text-2xl font-semibold text-surface-z8">Organizations</h1>
  <span class="text-sm text-surface-z5">{data.orgs.length} org{data.orgs.length !== 1 ? 's' : ''}</span>
</div>

{#if data.orgs.length === 0}
  <div class="flex flex-col items-center justify-center py-16 text-center gap-3">
    <p class="text-surface-z5">No organizations yet.</p>
    <p class="text-sm text-surface-z4">Create one or ask your team admin to invite you.</p>
  </div>
{:else}
  <div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(280px, 1fr))">
    {#each data.orgs as org}
      <a
        href="/orgs/{org.id}"
        class="flex flex-col p-4 rounded-lg border border-surface-z3 bg-surface-z1 no-underline transition-colors hover:border-primary-z5 hover:bg-surface-z2 min-h-28"
      >
        <div class="flex items-start justify-between gap-2 mb-1.5">
          <span class="font-semibold text-surface-z8 text-base leading-snug">{org.name}</span>
          <span class={planBadgeClass(org.plan)}>{org.plan}</span>
        </div>

        <p class="text-sm text-surface-z5 mb-3 leading-snug">{org.description}</p>

        <div class="flex gap-3 text-sm mt-auto">
          <span class="text-surface-z4">members <span class="font-medium text-surface-z7">{org.memberCount}</span></span>
          <span class="text-surface-z3">·</span>
          <span class="text-surface-z4">repos <span class="font-medium text-surface-z7">{org.repoCount}</span></span>
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
  .badge-plan-free {
    background: #f1f5f9;
    color: #475569;
  }
  .badge-plan-team {
    background: #dbeafe;
    color: #1d4ed8;
  }
  .badge-plan-enterprise {
    background: #ede9fe;
    color: #5b21b6;
  }
</style>
