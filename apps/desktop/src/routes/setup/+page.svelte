<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  type Step = 'welcome' | 'mcp' | 'import' | 'indexing' | 'done';
  let step = $state<Step>('welcome');

  // MCP step
  let mcpCopied = $state(false);
  let mcpDetected = $state(false);

  const mcpSnippet = `"sensei": {
  "command": "~/.sensei/bin/mcp",
  "args": []
}`;

  function copyMcp() {
    navigator.clipboard.writeText(mcpSnippet);
    mcpCopied = true;
    setTimeout(() => {
      mcpCopied = false;
      // Simulate detection after copy
      setTimeout(() => { mcpDetected = true; }, 1500);
    }, 1000);
  }

  // Import step
  let repoPath = $state('');
  let repoName = $state('');

  function pickFolder() {
    // In real Tauri app: invoke('pick_folder')
    repoPath = '~/Developer/my-project';
    repoName = 'my-project';
  }

  function onPathInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    repoPath = val;
    repoName = val.split('/').filter(Boolean).pop() ?? '';
  }

  // Indexing simulation
  let indexProgress = $state(0);
  let indexPhase = $state(0);

  const phases = [
    { label: 'Scanning files',       detail: '' },
    { label: 'Extracting symbols',   detail: '' },
    { label: 'Building graph',       detail: '' },
    { label: 'Detecting libraries',  detail: '' },
  ];

  const phaseResults = [
    '3,241 files',
    '8,432 symbols',
    '142 nodes · 5 communities',
    '6 libraries indexed',
  ];

  let indexInterval: ReturnType<typeof setInterval> | null = null;

  function startIndexing() {
    step = 'indexing';
    indexProgress = 0;
    indexPhase = 0;
    indexInterval = setInterval(() => {
      indexProgress = Math.min(100, indexProgress + 2 + Math.random() * 3);
      indexPhase = Math.min(3, Math.floor(indexProgress / 26));
      if (indexProgress >= 100) {
        clearInterval(indexInterval!);
        setTimeout(() => { step = 'done'; }, 400);
      }
    }, 80);
  }

  function finish() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('sensei:setup_complete', '1');
      localStorage.setItem('sensei:repos', JSON.stringify([{ name: repoName || 'my-project', path: repoPath || '~/Developer/my-project' }]));
    }
    goto('/projects');
  }

  onMount(() => {
    return () => { if (indexInterval) clearInterval(indexInterval); };
  });
</script>

<div class="drag-region flex h-screen flex-col items-center justify-center bg-surface-z1 select-none">

  <!-- Traffic light safe area -->
  <div class="absolute top-0 left-0 right-0 h-10"></div>

  {#if step === 'welcome'}
    <!-- ── Welcome ── -->
    <div class="no-drag flex flex-col items-center gap-6 px-8 text-center max-w-sm">
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-primary-z6 text-2xl font-bold text-white shadow-lg">⬡</div>
      <div>
        <h1 class="text-2xl font-bold text-surface-z9">Welcome to Sensei</h1>
        <p class="mt-2 text-sm text-surface-z5 leading-relaxed">
          Sensei tracks your Claude Code sessions, builds a knowledge graph of your codebase, and helps you work smarter over time.
        </p>
      </div>
      <div class="flex flex-col gap-2 w-full text-sm text-surface-z5">
        {#each ['Connect Claude Code via MCP', 'Import your project folders', 'Track sessions · graph · libraries'] as item}
          <div class="flex items-center gap-2.5 rounded-lg bg-surface-z2 px-4 py-2.5">
            <span class="h-1.5 w-1.5 rounded-full bg-primary-z5 shrink-0"></span>
            {item}
          </div>
        {/each}
      </div>
      <button
        onclick={() => step = 'mcp'}
        class="w-full rounded-xl bg-primary-z6 px-6 py-3 text-sm font-semibold text-white transition-colors hover:bg-primary-z7"
      >Get started →</button>
    </div>

  {:else if step === 'mcp'}
    <!-- ── Activate MCP ── -->
    <div class="no-drag flex flex-col gap-5 px-8 w-full max-w-md">
      <div>
        <p class="text-xs text-surface-z4 mb-1">Step 1 of 2</p>
        <h2 class="text-xl font-bold text-surface-z9">Connect Claude Code</h2>
        <p class="mt-1.5 text-sm text-surface-z5 leading-relaxed">
          Add sensei to Claude Desktop's MCP configuration so it can track your sessions automatically.
        </p>
      </div>

      <div>
        <p class="mb-1.5 text-xs text-surface-z4">
          Add this to <span class="font-mono">~/Library/Application&nbsp;Support/Claude/claude_desktop_config.json</span>
        </p>
        <div class="relative rounded-xl border border-surface-z3 bg-surface-z2 p-4 font-mono text-xs leading-relaxed text-surface-z6">
          <pre class="whitespace-pre-wrap">{mcpSnippet}</pre>
          <button
            onclick={copyMcp}
            class="absolute right-3 top-3 rounded-md border border-surface-z3 bg-surface-z1 px-2.5 py-1 text-[11px] transition-colors hover:bg-surface-z3
                   {mcpCopied ? 'text-success-z6 border-success-z3' : 'text-surface-z5'}"
          >{mcpCopied ? '✓ Copied' : 'Copy'}</button>
        </div>
      </div>

      <div class="flex items-center gap-3 rounded-xl border px-4 py-3 text-sm
                  {mcpDetected ? 'border-success-z3 bg-success-z1 text-success-z7' : 'border-surface-z3 bg-surface-z2 text-surface-z5'}">
        <span class="shrink-0 {mcpDetected ? 'i-solar-check-circle-bold-duotone text-success-z6' : 'i-solar-refresh-bold-duotone text-surface-z4 animate-spin'} text-base"></span>
        {mcpDetected ? 'MCP server detected — Claude Code is connected' : 'Waiting for MCP connection…'}
      </div>

      <div class="flex gap-2">
        <button onclick={() => step = 'welcome'} class="flex-1 rounded-xl border border-surface-z3 py-2.5 text-sm text-surface-z5 transition-colors hover:bg-surface-z2">
          Back
        </button>
        <button
          onclick={() => step = 'import'}
          class="flex-1 rounded-xl py-2.5 text-sm font-semibold transition-colors
                 {mcpDetected ? 'bg-primary-z6 text-white hover:bg-primary-z7' : 'bg-surface-z3 text-surface-z5 hover:bg-surface-z4'}"
        >{mcpDetected ? 'Continue →' : 'Skip for now →'}</button>
      </div>
    </div>

  {:else if step === 'import'}
    <!-- ── Import project ── -->
    <div class="no-drag flex flex-col gap-5 px-8 w-full max-w-md">
      <div>
        <p class="text-xs text-surface-z4 mb-1">Step 2 of 2</p>
        <h2 class="text-xl font-bold text-surface-z9">Import a project</h2>
        <p class="mt-1.5 text-sm text-surface-z5">
          Choose a repository folder. Sensei will scan its files, extract symbols, and build a knowledge graph.
        </p>
      </div>

      <!-- Folder picker -->
      <div class="flex gap-2">
        <input
          value={repoPath}
          oninput={onPathInput}
          placeholder="~/Developer/my-project"
          class="min-w-0 flex-1 rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-2.5 font-mono text-sm text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4"
        />
        <button
          onclick={pickFolder}
          aria-label="Browse folders"
          class="rounded-xl border border-surface-z3 bg-surface-z2 px-3.5 py-2.5 text-sm text-surface-z6 transition-colors hover:bg-surface-z3"
        >
          <span class="i-solar-folder-open-bold-duotone text-base"></span>
        </button>
      </div>

      {#if repoName}
        <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3 flex items-center gap-3">
          <span class="i-solar-code-square-bold-duotone text-xl text-primary-z6"></span>
          <div>
            <p class="text-sm font-semibold text-surface-z8">{repoName}</p>
            <p class="text-xs text-surface-z4">{repoPath}</p>
          </div>
        </div>
      {/if}

      <div class="flex gap-2">
        <button onclick={() => step = 'mcp'} class="flex-1 rounded-xl border border-surface-z3 py-2.5 text-sm text-surface-z5 transition-colors hover:bg-surface-z2">
          Back
        </button>
        <button
          onclick={startIndexing}
          disabled={!repoPath}
          class="flex-1 rounded-xl py-2.5 text-sm font-semibold transition-colors
                 {repoPath ? 'bg-primary-z6 text-white hover:bg-primary-z7' : 'bg-surface-z3 text-surface-z4 cursor-not-allowed'}"
        >Import →</button>
      </div>

      <button onclick={finish} class="text-center text-xs text-surface-z4 hover:text-surface-z6 transition-colors">
        Skip — I'll add projects later
      </button>
    </div>

  {:else if step === 'indexing'}
    <!-- ── Indexing ── -->
    <div class="no-drag flex flex-col gap-6 px-8 w-full max-w-sm">
      <div class="text-center">
        <h2 class="text-xl font-bold text-surface-z9">Indexing {repoName || 'project'}…</h2>
        <p class="mt-1 text-sm text-surface-z5">Building your knowledge graph</p>
      </div>

      <!-- Progress bar -->
      <div class="h-1.5 w-full rounded-full bg-surface-z3 overflow-hidden">
        <div
          class="h-full rounded-full bg-primary-z6 transition-all duration-200"
          style="width: {indexProgress}%"
        ></div>
      </div>

      <!-- Phase steps -->
      <div class="space-y-3">
        {#each phases as phase, i}
          {@const done = i < indexPhase || indexProgress >= 100}
          {@const active = i === indexPhase && indexProgress < 100}
          <div class="flex items-center gap-3 text-sm {done ? 'text-surface-z6' : active ? 'text-surface-z7' : 'text-surface-z3'}">
            {#if done}
              <span class="i-solar-check-circle-bold-duotone text-success-z5 text-base shrink-0"></span>
            {:else if active}
              <span class="i-solar-refresh-bold-duotone text-primary-z6 text-base shrink-0 animate-spin"></span>
            {:else}
              <span class="h-4 w-4 rounded-full border border-surface-z3 shrink-0"></span>
            {/if}
            <span class="flex-1">{phase.label}</span>
            {#if done}
              <span class="text-xs text-surface-z4">{phaseResults[i]}</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>

  {:else}
    <!-- ── Done ── -->
    <div class="no-drag flex flex-col items-center gap-6 px-8 text-center max-w-sm">
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-success-z5 text-2xl">✓</div>
      <div>
        <h2 class="text-xl font-bold text-surface-z9">Ready to go</h2>
        <p class="mt-1.5 text-sm text-surface-z5">
          {repoName || 'Your project'} has been indexed and added to Sensei.
        </p>
      </div>

      <!-- Summary -->
      <div class="grid grid-cols-2 gap-2 w-full text-sm">
        {#each [
          { label: 'Symbols',     value: '8,432' },
          { label: 'Communities', value: '5'     },
          { label: 'Libraries',   value: '6'     },
          { label: 'Files',       value: '3,241' },
        ] as stat}
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3 text-center">
            <p class="text-xl font-bold text-surface-z9">{stat.value}</p>
            <p class="text-xs text-surface-z4">{stat.label}</p>
          </div>
        {/each}
      </div>

      <button
        onclick={finish}
        class="w-full rounded-xl bg-primary-z6 px-6 py-3 text-sm font-semibold text-white transition-colors hover:bg-primary-z7"
      >Open Sensei →</button>
    </div>
  {/if}

  <!-- Step dots (mcp + import steps only) -->
  {#if step === 'mcp' || step === 'import'}
    <div class="absolute bottom-6 flex gap-1.5">
      {#each ['mcp', 'import'] as s}
        <div class="h-1.5 rounded-full transition-all {step === s ? 'w-4 bg-primary-z6' : 'w-1.5 bg-surface-z3'}"></div>
      {/each}
    </div>
  {/if}

</div>
