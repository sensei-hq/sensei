<script lang="ts">
  import { Button } from '@rokkit/ui';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';
</script>

<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

<!-- Landing page -->
<div class="min-h-screen overflow-y-auto bg-surface-z1 text-surface-z8">

  <!-- Nav -->
  <nav class="sticky top-0 z-50 flex items-center justify-between border-b border-surface-z0 bg-surface-z1/95 px-8 h-14">
    <div class="flex items-center gap-2 font-bold text-base select-none">
      <span class="text-lg">⬡</span>
      <span>sensei</span>
    </div>
    <div class="flex items-center gap-2">
      <Button href="/auth" label="Sign in" style="outline" size="sm" />
      <Button href="/auth" label="Get started" variant="primary" size="sm" />
    </div>
  </nav>

  <!-- Hero -->
  <div class="mx-auto max-w-3xl px-8 py-24 text-center">
    <div class="mb-6 inline-flex items-center gap-2 rounded-full border border-primary-z3 bg-primary-z1 px-3 py-1 text-xs font-semibold text-primary-z7">
      🧠 AI Development Intelligence
    </div>
    <h1 class="mb-5 text-5xl font-extrabold leading-tight tracking-tight">
      Ship faster.<br>
      <span class="bg-gradient-to-r from-primary-z6 to-secondary-z5 bg-clip-text text-transparent">Measure everything.</span>
    </h1>
    <p class="mb-9 mx-auto max-w-xl text-lg leading-relaxed text-surface-z5">
      Sensei captures how your team uses AI — context quality, session costs, first-try-right rates — and feeds that intelligence back into your workflow.
    </p>
    <div class="flex flex-wrap items-center justify-center gap-3">
      <Button href="/auth" label="Start free trial" variant="primary" size="lg" />
      <Button href="/mockups/platform" label="View demo →" style="outline" size="lg" />
    </div>
  </div>

  <!-- Stats strip -->
  <div class="border-y border-surface-z0 bg-surface-z2 py-10">
    <div class="mx-auto grid max-w-4xl grid-cols-2 gap-8 px-8 text-center sm:grid-cols-4">
      {#each [
        { n: '62%',   l: 'avg FTR improvement'    },
        { n: '$0.18', l: 'median task cost'        },
        { n: '4.2×',  l: 'faster with context'    },
        { n: '38%',   l: 'token savings via cache' },
      ] as stat}
        <div>
          <div class="text-3xl font-extrabold text-surface-z8">{stat.n}</div>
          <div class="mt-1 text-xs font-medium uppercase tracking-wider text-surface-z4">{stat.l}</div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Features -->
  <div class="mx-auto max-w-5xl px-8 py-20">
    <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">What sensei does</div>
    <h2 class="mb-3 text-3xl font-bold tracking-tight">The observability layer for AI-assisted development</h2>
    <p class="mb-12 max-w-md text-surface-z5">Like Code Climate for code quality — sensei gives you the metrics to understand and improve how your team uses AI.</p>
    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
      {#each [
        { ico: '🎯', h: 'First-Try-Right Scoring',  p: 'Track how often AI completes tasks on the first attempt. Per-repo, per-agent, per-team. Trending over time.' },
        { ico: '💰', h: 'Cost Intelligence',        p: 'Token-level cost per task, session, and repo. See exactly where money is spent and what drives cache hit rates.' },
        { ico: '📦', h: 'Context Packing',          p: 'Smart token-budgeted context using BM25 + semantic search. Right code, right size, every request.' },
        { ico: '🔄', h: 'Session Continuity',       p: 'Automatic crash recovery and progress snapshots. Never lose context mid-task.' },
        { ico: '📚', h: 'Doc Drift Detection',      p: 'Traceability matrix linking code to docs. Flags stale documentation automatically when code changes.' },
        { ico: '🧩', h: 'Plugin Ecosystem',         p: 'Skills, commands, and hooks that inject guardrails into every Claude Code session. Zero-friction.' },
      ] as f}
        <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-6">
          <div class="mb-4 text-2xl">{f.ico}</div>
          <h3 class="mb-2 text-sm font-semibold text-surface-z8">{f.h}</h3>
          <p class="text-sm leading-relaxed text-surface-z5">{f.p}</p>
        </div>
      {/each}
    </div>
  </div>

  <!-- How it works -->
  <div class="border-t border-surface-z0">
    <div class="mx-auto max-w-5xl px-8 py-20">
      <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">How it works</div>
      <h2 class="mb-3 text-3xl font-bold tracking-tight">Four steps to AI observability</h2>
      <p class="mb-14 text-surface-z5">From local setup to org-level insight in minutes.</p>
      <div class="relative grid grid-cols-2 gap-8 md:grid-cols-4">
        <div class="absolute left-[12.5%] right-[12.5%] top-6 hidden h-px bg-gradient-to-r from-primary-z5 to-secondary-z4 md:block"></div>
        {#each [
          { n: 1, h: 'Index your repo',    p: 'sensei init scans symbols, docs, and call graphs into your shared knowledge base.' },
          { n: 2, h: 'Log in and connect', p: 'Link local sensei to your org account. Sessions sync automatically via your setup token.' },
          { n: 3, h: 'Work normally',      p: 'MCP tools and Claude Code plugin capture metrics passively. No behavior change needed.' },
          { n: 4, h: 'See the insights',   p: 'Dashboard shows FTR trends, cost breakdown, context quality, and team velocity.' },
        ] as step}
          <div class="relative z-10 text-center">
            <div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full border-2 border-primary-z5 bg-surface-z2 text-base font-bold text-primary-z6">{step.n}</div>
            <h3 class="mb-2 text-sm font-semibold text-surface-z8">{step.h}</h3>
            <p class="text-xs leading-relaxed text-surface-z5">{step.p}</p>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <!-- Footer -->
  <div class="border-t border-surface-z0 py-8 text-center text-xs text-surface-z4">
    <span class="font-bold text-surface-z6">⬡ sensei</span> · AI Development Intelligence
  </div>

</div>
