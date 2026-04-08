<script lang="ts">
  import { page } from '$app/stores';

  let { data } = $props();

  // Determine current step based on what's detected server-side.
  // Step 0: Install CLI
  // Step 1: Login (sensei login)
  // Step 2: Init a repo (sensei init) — detected: hasRepos
  // Step 3: Setup MCP (sensei setup --mcp) — detected: hasSessions
  // Step 4: Setup collector (sensei setup --hooks)
  let currentStep = $derived(
    data.hasSessions ? 4 :
    data.hasRepos    ? 3 :
                       0
  );

  const steps = [
    { id: 0, title: 'Install CLI',       short: 'Install' },
    { id: 1, title: 'Log in',            short: 'Login'   },
    { id: 2, title: 'Init a repo',       short: 'Init'    },
    { id: 3, title: 'Setup MCP',         short: 'MCP'     },
    { id: 4, title: 'Start collector',   short: 'Collect' },
  ];

  let activeStep = $state(currentStep);
  $effect(() => { activeStep = currentStep; });

  // Org creation form
  let showOrgForm  = $state(false);
  let orgName      = $state('');
  let orgSlug      = $state('');
  let orgError     = $state('');
  let orgCreating  = $state(false);

  $effect(() => {
    // Auto-generate slug from name
    orgSlug = orgName.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
  });

  async function createOrg() {
    if (!orgName.trim()) return;
    orgCreating = true;
    orgError = '';
    try {
      const res = await fetch('/api/orgs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: orgName.trim(), slug: orgSlug }),
      });
      if (!res.ok) {
        const body = await res.json();
        orgError = body.message ?? 'Failed to create organization';
        return;
      }
      const org = await res.json();
      window.location.href = `/orgs/${org.slug}`;
    } finally {
      orgCreating = false;
    }
  }

  // Poll for repo/session changes (every 5s while on this page)
  $effect(() => {
    if (data.hasSessions) return; // already done
    const interval = setInterval(async () => {
      const res = await fetch($page.url.href, { headers: { 'x-sveltekit-invalidate': '1' } });
      if (res.ok) {
        // SvelteKit invalidation will re-run the load, updating data
        // For simplicity we trigger a soft reload check
        const { invalidate } = await import('$app/navigation');
        await invalidate('app:setup');
      }
    }, 5000);
    return () => clearInterval(interval);
  });

  const loginUrl = `/connect/cli?redirect_uri=${encodeURIComponent('http://localhost:7788/callback')}`;
</script>

<div class="flex min-h-screen flex-col bg-surface-z1">

  <!-- Header -->
  <header class="border-b border-surface-z3 bg-surface-z2 px-6 py-4">
    <div class="mx-auto flex max-w-2xl items-center justify-between">
      <div class="flex items-center gap-2.5">
        <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary-z6 text-sm font-bold text-white">⬡</div>
        <span class="font-bold text-surface-z8">sensei</span>
      </div>
      <div class="flex items-center gap-3 text-sm text-surface-z5">
        <span>Signed in as <strong class="text-surface-z7">{data.userName}</strong></span>
        {#if data.orgName}
          <span class="rounded-full bg-surface-z3 px-2 py-0.5 text-xs">{data.orgName}</span>
        {/if}
        <a href="/home" class="text-surface-z4 hover:text-surface-z7">Skip →</a>
      </div>
    </div>
  </header>

  <!-- Body -->
  <div class="mx-auto w-full max-w-2xl flex-1 px-6 py-10">

    <div class="mb-8 text-center">
      <h1 class="text-2xl font-bold text-surface-z8">Let's get you set up</h1>
      <p class="mt-1 text-sm text-surface-z5">Follow these steps to start tracking AI-assisted dev work.</p>
    </div>

    <!-- Step progress bar -->
    <div class="mb-10 flex items-center">
      {#each steps as step, i}
        <div class="relative flex flex-1 flex-col items-center gap-1.5">
          {#if i < steps.length - 1}
            <div class="absolute left-1/2 right-0 top-3.5 h-px
                        {i < currentStep ? 'bg-primary-z6' : 'bg-surface-z3'}"></div>
          {/if}
          <button
            onclick={() => { activeStep = step.id; }}
            class="relative z-10 flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold transition-colors
                   {i < currentStep
                     ? 'border-2 border-success-z6 bg-success-z1 text-success-z7'
                     : i === currentStep
                       ? 'border-2 border-primary-z6 bg-primary-z1 text-primary-z7'
                       : 'border-2 border-surface-z3 bg-surface-z1 text-surface-z4'}"
          >
            {i < currentStep ? '✓' : i + 1}
          </button>
          <span class="text-center text-[10px]
                       {i === activeStep ? 'text-surface-z7 font-medium' : 'text-surface-z4'}">{step.short}</span>
        </div>
      {/each}
    </div>

    <!-- Active step content -->
    <div class="rounded-2xl border border-surface-z3 bg-surface-z2 p-6">

      {#if activeStep === 0}
        <h2 class="mb-1 text-base font-semibold text-surface-z8">Step 1 — Install the CLI</h2>
        <p class="mb-4 text-sm text-surface-z5">Install sensei globally on your machine.</p>
        <div class="space-y-3">
          <div>
            <p class="mb-1 text-xs font-medium text-surface-z5 uppercase tracking-wide">macOS / Linux</p>
            <div class="rounded-lg border border-surface-z3 bg-surface-z1 px-4 py-3 font-mono text-sm text-success-z6">
              npm install -g @sensei/cli
            </div>
          </div>
          <div>
            <p class="mb-1 text-xs font-medium text-surface-z5 uppercase tracking-wide">Or with Bun</p>
            <div class="rounded-lg border border-surface-z3 bg-surface-z1 px-4 py-3 font-mono text-sm text-success-z6">
              bun add -g @sensei/cli
            </div>
          </div>
        </div>
        <p class="mt-4 text-xs text-surface-z4">Verify with: <code class="font-mono">sensei --version</code></p>
        <button
          onclick={() => { activeStep = 1; }}
          class="mt-5 rounded-lg bg-primary-z6 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors"
        >
          Installed — next →
        </button>

      {:else if activeStep === 1}
        <h2 class="mb-1 text-base font-semibold text-surface-z8">Step 2 — Log in</h2>
        <p class="mb-4 text-sm text-surface-z5">Connect your CLI to this account.</p>
        <div class="rounded-lg border border-surface-z3 bg-surface-z1 px-4 py-3 font-mono text-sm text-success-z6 mb-4">
          sensei login
        </div>
        <p class="text-xs text-surface-z5 mb-4">
          This opens a browser window. If it doesn't auto-open, click the button below:
        </p>
        <a
          href={loginUrl}
          target="_blank"
          rel="noopener noreferrer"
          class="inline-flex items-center gap-2 rounded-lg border border-primary-z5 bg-primary-z1 px-4 py-2 text-sm font-medium text-primary-z7 hover:bg-primary-z2 transition-colors"
        >
          <span>Open login page</span>
          <span class="text-xs">↗</span>
        </a>
        <p class="mt-4 text-xs text-surface-z4">Credentials saved to <code class="font-mono">~/.config/sensei/credentials.yaml</code></p>
        <button
          onclick={() => { activeStep = 2; }}
          class="mt-5 block rounded-lg bg-primary-z6 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors"
        >
          Logged in — next →
        </button>

      {:else if activeStep === 2}
        <div class="flex items-start justify-between mb-1">
          <h2 class="text-base font-semibold text-surface-z8">Step 3 — Init a repo</h2>
          {#if data.hasRepos}
            <span class="rounded-full bg-success-z1 border border-success-z3 px-2 py-0.5 text-xs text-success-z7">✓ Detected</span>
          {:else}
            <span class="rounded-full bg-surface-z3 px-2 py-0.5 text-xs text-surface-z5">Waiting…</span>
          {/if}
        </div>
        <p class="mb-4 text-sm text-surface-z5">Index your first codebase. Run this inside any project folder.</p>
        <div class="rounded-lg border border-surface-z3 bg-surface-z1 px-4 py-3 font-mono text-sm text-success-z6 mb-3">
          cd your-project && sensei init
        </div>
        <p class="text-xs text-surface-z4 mb-4">
          This scans the repo, extracts symbols, and registers it with your account.
          {#if !data.hasRepos}
            The page will update automatically when it detects a new repo.
          {/if}
        </p>
        {#if data.hasRepos}
          <button
            onclick={() => { activeStep = 3; }}
            class="rounded-lg bg-primary-z6 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors"
          >
            Continue →
          </button>
        {/if}

      {:else if activeStep === 3}
        <h2 class="mb-1 text-base font-semibold text-surface-z8">Step 4 — Setup MCP</h2>
        <p class="mb-4 text-sm text-surface-z5">Register sensei as an MCP server in Claude Code (or your IDE).</p>
        <div class="rounded-lg border border-surface-z3 bg-surface-z1 px-4 py-3 font-mono text-sm text-success-z6 mb-3">
          sensei setup --mcp
        </div>
        <p class="text-xs text-surface-z4 mb-4">
          This writes to <code class="font-mono">~/.claude/mcp.json</code> and adds the OTEL endpoint to
          <code class="font-mono">~/.claude/settings.json</code>. Restart Claude Code after.
        </p>
        <button
          onclick={() => { activeStep = 4; }}
          class="rounded-lg bg-primary-z6 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors"
        >
          Done — next →
        </button>

      {:else if activeStep === 4}
        <div class="flex items-start justify-between mb-1">
          <h2 class="text-base font-semibold text-surface-z8">Step 5 — Start the collector</h2>
          {#if data.hasSessions}
            <span class="rounded-full bg-success-z1 border border-success-z3 px-2 py-0.5 text-xs text-success-z7">✓ Active</span>
          {:else}
            <span class="rounded-full bg-surface-z3 px-2 py-0.5 text-xs text-surface-z5">Waiting…</span>
          {/if}
        </div>
        <p class="mb-4 text-sm text-surface-z5">Install hooks so Claude tool calls are tracked automatically.</p>
        <div class="rounded-lg border border-surface-z3 bg-surface-z1 px-4 py-3 font-mono text-sm text-success-z6 mb-3">
          sensei setup --hooks
        </div>
        <p class="text-xs text-surface-z4 mb-4">
          Installs hook scripts and registers a background daemon.
          {#if !data.hasSessions}
            Once you run a task in Claude Code, sessions will appear here automatically.
          {/if}
        </p>
        {#if data.hasSessions}
          <a
            href="/home"
            class="inline-block rounded-lg bg-success-z6 px-5 py-2 text-sm font-semibold text-white hover:bg-success-z7 transition-colors"
          >
            You're all set — go to dashboard →
          </a>
        {/if}
      {/if}

    </div>

    <!-- Org setup section (below wizard) -->
    {#if !data.orgName}
      <div class="mt-8 rounded-2xl border border-dashed border-surface-z4 bg-surface-z2 p-6">
        <div class="flex items-start justify-between gap-4">
          <div>
            <p class="text-sm font-semibold text-surface-z8">Add your team</p>
            <p class="mt-0.5 text-xs text-surface-z5">Create an organization to invite teammates and share analytics.</p>
          </div>
          <button
            onclick={() => { showOrgForm = !showOrgForm; }}
            class="shrink-0 rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-1.5 text-xs font-medium text-surface-z7 hover:border-primary-z5 hover:bg-surface-z2 transition-colors"
          >
            {showOrgForm ? 'Cancel' : 'Create org'}
          </button>
        </div>
        {#if showOrgForm}
          <div class="mt-4 space-y-3">
            <div>
              <label class="mb-1 block text-xs font-medium text-surface-z6" for="org-name">Organization name</label>
              <input
                id="org-name"
                type="text"
                bind:value={orgName}
                placeholder="Acme Corp"
                class="w-full rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2 text-sm text-surface-z8 placeholder-surface-z4 focus:border-primary-z5 focus:outline-none"
              />
            </div>
            <div>
              <label class="mb-1 block text-xs font-medium text-surface-z6" for="org-slug">Slug</label>
              <input
                id="org-slug"
                type="text"
                bind:value={orgSlug}
                placeholder="acme-corp"
                class="w-full rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2 font-mono text-sm text-surface-z8 placeholder-surface-z4 focus:border-primary-z5 focus:outline-none"
              />
            </div>
            {#if orgError}
              <p class="text-xs text-error-z6">{orgError}</p>
            {/if}
            <button
              onclick={createOrg}
              disabled={orgCreating || !orgName.trim()}
              class="rounded-lg bg-primary-z6 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-z7 disabled:opacity-50 transition-colors"
            >
              {orgCreating ? 'Creating…' : 'Create organization'}
            </button>
          </div>
        {/if}
      </div>
    {/if}

  </div>

</div>
