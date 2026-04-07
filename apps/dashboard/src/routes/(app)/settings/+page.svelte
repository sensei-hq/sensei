<script lang="ts">
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const userInitial = $derived(
    data.user ? (data.user.name ?? data.user.email ?? '?').charAt(0).toUpperCase() : '?'
  );

  const planBadgeClass = (plan: string) => {
    if (plan === 'team') return 'badge badge-team';
    if (plan === 'enterprise') return 'badge badge-enterprise';
    return 'badge badge-free';
  };

  const roleBadgeClass = (role: string) => {
    if (role === 'admin' || role === 'owner') return 'badge badge-admin';
    return 'badge badge-member';
  };
</script>

<h1 class="text-2xl font-semibold text-surface-z8 mb-8">Settings</h1>

<!-- Profile Section -->
<section class="border-b border-surface-z3 pb-6 mb-6">
  <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-4">Profile</h2>
  {#if data.user}
    <div class="flex items-center gap-4">
      <div class="flex h-14 w-14 items-center justify-center rounded-full bg-primary-z6 text-xl font-semibold" style="color: white">
        {userInitial}
      </div>
      <div>
        <div class="flex items-center gap-2 mb-0.5">
          <span class="font-semibold text-surface-z8">{data.user.name ?? data.user.email}</span>
        </div>
        <p class="text-sm text-surface-z5">{data.user.email}</p>
      </div>
      <div class="ml-auto">
        <button class="px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z5 cursor-not-allowed opacity-60" disabled>
          Edit Profile
        </button>
      </div>
    </div>
  {:else}
    <p class="text-sm text-surface-z4">Not signed in.</p>
  {/if}
</section>

<!-- Organizations Section -->
<section class="border-b border-surface-z3 pb-6 mb-6">
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Organizations</h2>
    <a
      href="/orgs"
      class="px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z7 hover:border-primary-z5 hover:bg-surface-z2 no-underline transition-colors"
    >
      Create Organization
    </a>
  </div>
  {#if data.orgs.length === 0}
    <p class="text-sm text-surface-z4">No organizations yet.</p>
  {:else}
    <div class="flex flex-col gap-2">
      {#each data.orgs as org}
        <a
          href="/orgs/{org.slug}"
          class="flex items-center justify-between p-3 rounded-lg border border-surface-z3 bg-surface-z1 no-underline transition-colors hover:border-primary-z5 hover:bg-surface-z2"
        >
          <span class="font-medium text-surface-z8">{org.name}</span>
          <div class="flex items-center gap-2">
            <span class={roleBadgeClass(org.role ?? '')}>{org.role}</span>
            <span class={planBadgeClass(org.plan ?? '')}>{org.plan}</span>
          </div>
        </a>
      {/each}
    </div>
  {/if}
</section>

<!-- API Token Section -->
<section class="border-b border-surface-z3 pb-6 mb-6">
  <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-4">API Token</h2>
  {#if data.apiToken}
    <div class="flex items-center justify-between p-4 rounded-lg border border-surface-z3 bg-surface-z1">
      <div>
        <code class="text-sm font-mono text-surface-z8">{data.apiToken.prefix}</code>
        <p class="text-xs text-surface-z4 mt-1">Last generated {new Date(data.apiToken.lastGenerated).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })}</p>
      </div>
      <button class="px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z5 cursor-not-allowed opacity-60" disabled>
        Regenerate
      </button>
    </div>
  {:else}
    <div class="p-4 rounded-lg border border-dashed border-surface-z3 bg-surface-z1 text-sm text-surface-z4">
      No API token generated yet.
    </div>
  {/if}
</section>

<!-- Sensei Daemon Section -->
<section class="pb-6">
  <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-4">Sensei Daemon</h2>
  {#if data.daemonStatus}
    <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1">
      <div class="flex items-center justify-between mb-3">
        <div class="flex items-center gap-2">
          <span class="status-dot" class:status-running={data.daemonStatus.running} class:status-stopped={!data.daemonStatus.running}></span>
          <span class="font-medium text-surface-z8">{data.daemonStatus.running ? 'Running' : 'Stopped'}</span>
        </div>
        <button class="px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z7 hover:bg-surface-z2 transition-colors">
          Restart Daemon
        </button>
      </div>
      <p class="text-xs text-surface-z4 mt-3 font-mono">$ sensei restart</p>
    </div>
  {:else}
    <div class="p-4 rounded-lg border border-dashed border-surface-z3 bg-surface-z1 text-sm text-surface-z4">
      Daemon status unavailable. Run <code class="font-mono">sensei status</code> to check.
    </div>
  {/if}
</section>

<style>
  .badge {
    padding: 2px 8px;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: capitalize;
  }
  .badge-free { background: #f1f5f9; color: #475569; }
  .badge-team { background: #dbeafe; color: #1d4ed8; }
  .badge-enterprise { background: #ede9fe; color: #5b21b6; }
  .badge-admin { background: #dcfce7; color: #166534; }
  .badge-member { background: #f1f5f9; color: #475569; }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }
  .status-running { background: #22c55e; }
  .status-stopped { background: #ef4444; }
</style>
