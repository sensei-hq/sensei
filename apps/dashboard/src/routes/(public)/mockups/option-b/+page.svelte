<script lang="ts">
  import { ThemeSwitcherToggle } from '@rokkit/app';
</script>

<!-- Option switcher -->
<div class="fixed bottom-5 right-5 z-50 flex items-center gap-1.5 rounded-full border border-surface-z4 bg-surface-z2 px-3 py-1.5 text-xs shadow-lg">
  <a href="/mockups/option-a" class="text-primary-z6 hover:text-primary-z7 transition-colors">← Option A</a>
  <span class="text-surface-z4">·</span>
  <span class="font-semibold text-surface-z8">Option B</span>
</div>

<!-- Sticky Nav -->
<header class="sticky top-0 z-40 border-b border-surface-z3 bg-surface-z1/95 backdrop-blur">
  <div class="mx-auto flex max-w-6xl items-center justify-between px-8 py-3.5">
    <div class="flex items-center gap-7">
      <span class="text-sm font-bold tracking-tight text-surface-z8">sensei</span>
      <nav class="hidden items-center gap-5 text-sm text-surface-z5 sm:flex">
        <a href="#what-you-get" class="hover:text-surface-z8 transition-colors">Features</a>
        <a href="#analytics" class="hover:text-surface-z8 transition-colors">Analytics</a>
        <a href="#libraries" class="hover:text-surface-z8 transition-colors">Libraries</a>
      </nav>
    </div>
    <div class="flex items-center gap-3">
      <ThemeSwitcherToggle />
      <a href="/auth/signin" class="rounded-md bg-primary-z6 px-4 py-1.5 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors">
        Sign in
      </a>
    </div>
  </div>
</header>

<!-- Hero: split layout -->
<section class="mx-auto max-w-6xl px-8 py-16 lg:py-24">
  <div class="grid grid-cols-1 gap-12 lg:grid-cols-2 lg:items-center">

    <!-- Left: copy -->
    <div>
      <div class="mb-4 inline-flex items-center gap-2 rounded-full border border-primary-z3 bg-primary-z1 px-3 py-1 text-xs font-medium text-primary-z7">
        <span class="h-1.5 w-1.5 rounded-full bg-primary-z6"></span>
        AI-assisted development, finally measurable
      </div>
      <h1 class="mb-5 text-4xl font-extrabold leading-tight tracking-tight text-surface-z8 xl:text-5xl">
        Your AI coding<br />assistant, audited.
      </h1>
      <p class="mb-7 text-lg leading-relaxed text-surface-z5">
        Sensei gives you session analytics, first-try-right scores, cost tracking, and library intelligence
        — everything you need to know if AI is actually helping.
      </p>
      <div class="flex flex-wrap items-center gap-3">
        <a href="/auth/signin"
           class="rounded-lg bg-primary-z6 px-6 py-2.5 text-sm font-bold text-white shadow-sm hover:bg-primary-z7 transition-colors">
          Get started free
        </a>
        <a href="#analytics"
           class="rounded-lg border border-surface-z4 px-6 py-2.5 text-sm font-semibold text-surface-z7 hover:bg-surface-z2 transition-colors">
          See the dashboard
        </a>
      </div>
    </div>

    <!-- Right: stats mockup -->
    <div class="rounded-2xl border border-surface-z3 bg-surface-z2 p-5 shadow-xl">
      <!-- Stat cards row -->
      <div class="mb-4 grid grid-cols-2 gap-3">
        {#each [
          { label: 'Sessions',  value: '847',   sub: 'all time',        cls: '' },
          { label: 'Avg FTR',   value: '91%',   sub: 'first-try-right', cls: 'text-success-z6' },
          { label: 'Total cost', value: '$24.30', sub: 'this month',    cls: '' },
          { label: 'Repos',     value: '3',      sub: 'active',         cls: '' },
        ] as s}
          <div class="rounded-lg border border-surface-z3 bg-surface-z1 p-3">
            <div class="text-xl font-bold {s.cls || 'text-surface-z8'}">{s.value}</div>
            <div class="text-xs text-surface-z5">{s.label}</div>
            <div class="text-xs text-surface-z4">{s.sub}</div>
          </div>
        {/each}
      </div>

      <!-- Recent sessions mini-table -->
      <div class="overflow-hidden rounded-lg border border-surface-z3">
        <div class="border-b border-surface-z3 bg-surface-z3/50 px-3 py-2 text-xs font-semibold uppercase tracking-wider text-surface-z5">
          Recent sessions
        </div>
        {#each [
          { task: 'Fix SSR build failures',       ftr: 100, cost: 1.20, when: '2h ago'  },
          { task: 'Add auth OTP flow',             ftr: 95,  cost: 3.40, when: '1d ago'  },
          { task: 'Migrate jsdom → linkedom',      ftr: 82,  cost: 2.80, when: '2d ago'  },
          { task: 'Implement context pack search', ftr: 97,  cost: 1.60, when: '3d ago'  },
        ] as s}
          <div class="flex items-center gap-3 border-b border-surface-z2 last:border-0 px-3 py-2.5 text-xs">
            <span class="flex-1 min-w-0 truncate text-surface-z7">{s.task}</span>
            <span class="shrink-0 font-mono text-surface-z5">${s.cost.toFixed(2)}</span>
            <span class="shrink-0 rounded-full px-2 py-0.5 font-semibold
              {s.ftr >= 95 ? 'bg-success-z1 text-success-z7' : s.ftr >= 80 ? 'bg-warning-z1 text-warning-z7' : 'bg-error-z1 text-error-z7'}">
              {s.ftr}%
            </span>
          </div>
        {/each}
      </div>
    </div>
  </div>
</section>

<!-- Terminal strip -->
<section class="border-y border-surface-z3 bg-surface-z2 py-10">
  <div class="mx-auto max-w-6xl px-8">
    <p class="mb-4 text-xs font-semibold uppercase tracking-widest text-primary-z6">See it in action</p>
    <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-5 font-mono text-xs leading-relaxed overflow-x-auto shadow-inner">
      <div class="mb-3 flex items-center gap-1.5">
        <span class="h-2.5 w-2.5 rounded-full bg-error-z5"></span>
        <span class="h-2.5 w-2.5 rounded-full bg-warning-z5"></span>
        <span class="h-2.5 w-2.5 rounded-full bg-success-z5"></span>
        <span class="ml-2 text-surface-z4">sensei — zsh</span>
      </div>
      <div class="space-y-1 text-surface-z5">
        <p><span class="text-primary-z5">$</span> <span class="text-surface-z8">sensei init</span></p>
        <p class="pl-2 text-surface-z4">✓ Connected to repo: dashboard</p>
        <p class="pl-2 text-surface-z4">✓ Supabase session store configured</p>
        <p class="pl-2 text-surface-z4">✓ Watching .sensei/sessions/</p>
        <p class="mt-3"><span class="text-primary-z5">$</span> <span class="text-surface-z8">sensei report --last 7d</span></p>
        <p class="pl-2 text-surface-z4 mt-1">Sessions: <span class="text-surface-z7">24</span> &nbsp;·&nbsp; Avg FTR: <span class="text-success-z5">91%</span> &nbsp;·&nbsp; Spend: <span class="text-surface-z7">$24.30</span></p>
      </div>

      <!-- Table using CSS grid for reliable column alignment -->
      <div class="mt-3 pl-2">
        <div class="grid text-surface-z4 border-b border-surface-z3 pb-1 mb-1"
             style="grid-template-columns: 1fr 4rem 4rem 5rem">
          <span>Task</span>
          <span class="text-right">FTR</span>
          <span class="text-right">Turns</span>
          <span class="text-right">Cost</span>
        </div>
        {#each [
          { task: 'Fix SSR build failures',       ftr: '100%', turns: 4,  cost: '$1.20' },
          { task: 'Add auth OTP flow',             ftr:  '95%', turns: 7,  cost: '$3.40' },
          { task: 'Migrate jsdom → linkedom',      ftr:  '82%', turns: 12, cost: '$2.80' },
          { task: 'Implement context pack search', ftr:  '97%', turns: 5,  cost: '$1.60' },
          { task: 'Benchmark skill variants',      ftr:  '88%', turns: 9,  cost: '$0.90' },
        ] as row}
          <div class="grid py-0.5"
               style="grid-template-columns: 1fr 4rem 4rem 5rem">
            <span class="text-surface-z6 truncate pr-4">{row.task}</span>
            <span class="text-right
              {row.ftr === '100%' || parseInt(row.ftr) >= 95 ? 'text-success-z5'
               : parseInt(row.ftr) >= 80 ? 'text-warning-z5'
               : 'text-error-z5'}">{row.ftr}</span>
            <span class="text-right text-surface-z5">{row.turns}</span>
            <span class="text-right text-surface-z5">{row.cost}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>
</section>

<!-- What you get -->
<section id="what-you-get" class="border-y border-surface-z3 bg-surface-z2 py-16">
  <div class="mx-auto max-w-6xl px-8">
    <div class="mb-2 text-xs font-semibold uppercase tracking-widest text-primary-z6">What you get</div>
    <h2 class="mb-10 text-3xl font-bold text-surface-z8">The complete picture of your AI coding practice</h2>
    <div class="grid grid-cols-1 gap-6 sm:grid-cols-3">
      {#each [
        {
          icon: '📊',
          title: 'Session Analytics',
          body: 'Every Claude Code session recorded. FTR scores, turn counts, durations, and API cost — aggregated by repo, model, and time period.',
        },
        {
          icon: '📚',
          title: 'Library Intelligence',
          body: 'Auto-index docs for your exact library versions. Claude gets accurate answers instead of hallucinated APIs, boosting FTR dramatically.',
        },
        {
          icon: '🎯',
          title: 'Skill Benchmarking',
          body: 'A/B test prompt skills against each other. Quantify which instructions make Claude better and retire what doesn\'t work.',
        },
      ] as c}
        <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-6">
          <div class="mb-3 text-2xl">{c.icon}</div>
          <h3 class="mb-2 font-semibold text-surface-z8">{c.title}</h3>
          <p class="text-sm leading-relaxed text-surface-z5">{c.body}</p>
        </div>
      {/each}
    </div>
  </div>
</section>

<!-- Deep-dive 1: Analytics -->
<section id="analytics" class="py-16">
  <div class="mx-auto max-w-6xl px-8">
    <div class="grid grid-cols-1 gap-12 lg:grid-cols-2 lg:items-center">
      <div>
        <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Session analytics</div>
        <h2 class="mb-4 text-2xl font-bold text-surface-z8">See every session at a glance</h2>
        <p class="mb-5 text-surface-z5 leading-relaxed">
          Track the tasks that took too many turns, cost too much, or needed back-and-forth revision.
          Sensei surfaces patterns so you can improve context, not just retry.
        </p>
        <ul class="space-y-2 text-sm">
          {#each [
            'First-try-right rate per repo and model',
            'Turn counts and session duration',
            'Per-session and aggregate API costs',
            'Trend charts across time periods',
          ] as item}
            <li class="flex items-start gap-2 text-surface-z6">
              <span class="mt-0.5 text-success-z6">✓</span>
              {item}
            </li>
          {/each}
        </ul>
      </div>
      <!-- Session table mockup -->
      <div class="overflow-hidden rounded-xl border border-surface-z3 bg-surface-z2 shadow-lg">
        <div class="grid grid-cols-4 border-b border-surface-z3 bg-surface-z3/60 px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">
          <span class="col-span-2">Task</span>
          <span class="text-center">FTR</span>
          <span class="text-right">Cost</span>
        </div>
        {#each [
          { task: 'Fix SSR build failures',       ftr: 100, cost: '$1.20' },
          { task: 'Add auth OTP flow',             ftr: 95,  cost: '$3.40' },
          { task: 'Migrate jsdom → linkedom',      ftr: 82,  cost: '$2.80' },
          { task: 'Implement context search',      ftr: 97,  cost: '$1.60' },
          { task: 'Benchmark skill variants',      ftr: 88,  cost: '$0.90' },
        ] as s}
          <div class="grid grid-cols-4 items-center border-b border-surface-z2 last:border-0 px-4 py-3 text-xs">
            <span class="col-span-2 truncate text-surface-z7">{s.task}</span>
            <div class="flex justify-center">
              <span class="rounded-full px-2 py-0.5 font-semibold
                {s.ftr >= 95 ? 'bg-success-z1 text-success-z7' : s.ftr >= 80 ? 'bg-warning-z1 text-warning-z7' : 'bg-error-z1 text-error-z7'}">
                {s.ftr}%
              </span>
            </div>
            <span class="text-right font-mono text-surface-z5">{s.cost}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>
</section>

<!-- Deep-dive 2: Benchmarking -->
<section class="border-y border-surface-z3 bg-surface-z2 py-16">
  <div class="mx-auto max-w-6xl px-8">
    <div class="grid grid-cols-1 gap-12 lg:grid-cols-2 lg:items-center">
      <!-- Benchmark comparison mockup -->
      <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-5 shadow-lg">
        <div class="mb-4 text-xs font-semibold uppercase tracking-wider text-surface-z5">Skill comparison · brainstorming</div>
        <div class="space-y-3">
          {#each [
            { label: 'First-try-right',   a: 84, b: 91 },
            { label: 'Avg turns',         a: 6.2, b: 4.1, lower: true },
            { label: 'Sessions tested',   a: 40, b: 40 },
          ] as row}
            <div>
              <div class="mb-1 flex items-center justify-between text-xs text-surface-z5">
                <span>{row.label}</span>
                <div class="flex gap-4">
                  <span class="text-surface-z4">A: {row.a}{typeof row.a === 'number' && row.a > 10 ? '' : ''}</span>
                  <span class="{row.lower ? (row.b < row.a ? 'text-success-z6' : 'text-warning-z6') : (row.b > row.a ? 'text-success-z6' : 'text-warning-z6')} font-semibold">
                    B: {row.b}
                  </span>
                </div>
              </div>
              <div class="flex gap-1.5">
                <div class="h-2 flex-1 overflow-hidden rounded-full bg-surface-z3">
                  <div class="h-full rounded-full bg-surface-z5"
                       style="width:{typeof row.a === 'number' && row.a <= 100 ? row.a : 80}%"></div>
                </div>
                <div class="h-2 flex-1 overflow-hidden rounded-full bg-surface-z3">
                  <div class="h-full rounded-full bg-success-z5"
                       style="width:{typeof row.b === 'number' && row.b <= 100 ? row.b : 60}%"></div>
                </div>
              </div>
            </div>
          {/each}
        </div>
        <div class="mt-4 flex items-center gap-3 text-xs">
          <span class="flex items-center gap-1.5"><span class="h-2 w-4 rounded-full bg-surface-z5"></span>Variant A (baseline)</span>
          <span class="flex items-center gap-1.5"><span class="h-2 w-4 rounded-full bg-success-z5"></span>Variant B (+7.1%)</span>
        </div>
      </div>
      <div>
        <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Skill benchmarking</div>
        <h2 class="mb-4 text-2xl font-bold text-surface-z8">Measure your prompts, not just your code</h2>
        <p class="mb-5 text-surface-z5 leading-relaxed">
          Run A/B tests on skill variants with real session data. Promote what improves FTR.
          Retire instructions that add turns without value.
        </p>
        <ul class="space-y-2 text-sm">
          {#each [
            'Statistical comparison across real sessions',
            'FTR, turns, and cost deltas side by side',
            'Promote winner with one click',
            'Full history of every variant tested',
          ] as item}
            <li class="flex items-start gap-2 text-surface-z6">
              <span class="mt-0.5 text-success-z6">✓</span>
              {item}
            </li>
          {/each}
        </ul>
      </div>
    </div>
  </div>
</section>

<!-- Deep-dive 3: Libraries -->
<section id="libraries" class="py-16">
  <div class="mx-auto max-w-6xl px-8">
    <div class="grid grid-cols-1 gap-12 lg:grid-cols-2 lg:items-center">
      <div>
        <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Library intelligence</div>
        <h2 class="mb-4 text-2xl font-bold text-surface-z8">Stop Claude from hallucinating your dependencies</h2>
        <p class="mb-5 text-surface-z5 leading-relaxed">
          Sensei indexes documentation for your exact library versions so Claude always has accurate,
          up-to-date context — no more outdated APIs, no more made-up function signatures.
        </p>
        <ul class="space-y-2 text-sm">
          {#each [
            'Auto-fetch docs from llms.txt endpoints',
            'Versioned — pinned to your exact lockfile',
            'Included in context packs automatically',
            'Works with kavach, rokkit, and any custom lib',
          ] as item}
            <li class="flex items-start gap-2 text-surface-z6">
              <span class="mt-0.5 text-success-z6">✓</span>
              {item}
            </li>
          {/each}
        </ul>
      </div>
      <!-- Library list mockup -->
      <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4 shadow-lg">
        <div class="mb-3 text-xs font-semibold uppercase tracking-wider text-surface-z5">Indexed libraries</div>
        <div class="space-y-2">
          {#each [
            { name: 'kavach',      version: '1.0.0-next.37', sections: 12, cat: 'auth' },
            { name: '@rokkit/ui',  version: '1.0.0-next.145', sections: 28, cat: 'ui'  },
            { name: '@supabase/supabase-js', version: '2.99.1', sections: 34, cat: 'db' },
            { name: 'sveltekit',   version: '2.50.2',         sections: 19, cat: 'framework' },
            { name: 'unocss',      version: '66.6.6',          sections: 11, cat: 'css' },
          ] as lib}
            <div class="flex items-center gap-3 rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2.5">
              <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-surface-z3 text-xs font-bold text-surface-z7">
                {lib.name.replace('@', '').charAt(0).toUpperCase()}
              </span>
              <div class="flex-1 min-w-0">
                <p class="text-xs font-medium text-surface-z8 truncate">{lib.name}</p>
                <p class="text-xs text-surface-z4">{lib.version} · {lib.sections} sections</p>
              </div>
              <span class="shrink-0 rounded bg-surface-z3 px-1.5 py-0.5 text-xs capitalize text-surface-z5">
                {lib.cat}
              </span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
</section>

<!-- CTA block (dark) -->
<section class="bg-surface-z8 py-16">
  <div class="mx-auto max-w-6xl px-8 text-center">
    <h2 class="mb-4 text-3xl font-bold text-surface-z1">Ready to see your real FTR?</h2>
    <p class="mb-8 text-surface-z4">Connect your first repo in minutes. No credit card required.</p>
    <div class="flex flex-wrap items-center justify-center gap-4">
      <a href="/auth/signin"
         class="rounded-lg bg-primary-z5 px-8 py-3 text-sm font-bold text-white hover:bg-primary-z6 transition-colors">
        Sign in to get started
      </a>
      <a href="/mockups/option-a"
         class="rounded-lg border border-surface-z5 px-8 py-3 text-sm font-semibold text-surface-z3 hover:border-surface-z4 hover:text-surface-z2 transition-colors">
        See Option A
      </a>
    </div>
  </div>
</section>

<!-- Footer -->
<footer class="border-t border-surface-z3 py-6">
  <div class="mx-auto max-w-6xl px-8 flex items-center justify-between text-xs text-surface-z4">
    <span>sensei · AI coding analytics</span>
    <nav class="flex gap-5">
      <a href="#what-you-get" class="hover:text-surface-z6 transition-colors">Features</a>
      <a href="#analytics" class="hover:text-surface-z6 transition-colors">Analytics</a>
      <a href="#libraries" class="hover:text-surface-z6 transition-colors">Libraries</a>
      <a href="/auth/signin" class="hover:text-surface-z6 transition-colors">Sign in</a>
    </nav>
  </div>
</footer>
