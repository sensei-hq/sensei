<script lang="ts">
  import { ThemeSwitcherToggle } from '@rokkit/app';
</script>

<!-- Option switcher -->
<div class="fixed bottom-5 right-5 z-50 flex items-center gap-1.5 rounded-full border border-surface-z4 bg-surface-z2 px-3 py-1.5 text-xs shadow-lg">
  <span class="font-semibold text-surface-z8">Option A</span>
  <span class="text-surface-z4">·</span>
  <a href="/mockups/option-b" class="text-primary-z6 hover:text-primary-z7 transition-colors">Option B →</a>
</div>

<!-- Nav -->
<header class="sticky top-0 z-40 border-b border-surface-z3 bg-surface-z1/90 backdrop-blur">
  <div class="mx-auto flex max-w-5xl items-center justify-between px-6 py-3">
    <span class="text-sm font-bold tracking-tight text-surface-z8">sensei</span>
    <nav class="flex items-center gap-5 text-sm text-surface-z6">
      <a href="#features" class="hover:text-surface-z8 transition-colors">Features</a>
      <a href="#how" class="hover:text-surface-z8 transition-colors">How it works</a>
      <ThemeSwitcherToggle />
      <a href="/auth/signin" class="rounded-md bg-primary-z6 px-3 py-1.5 text-white hover:bg-primary-z7 transition-colors">Sign in</a>
    </nav>
  </div>
</header>

<!-- Hero -->
<section class="mx-auto max-w-5xl px-6 py-20">
  <div class="max-w-2xl">
    <p class="mb-3 font-mono text-xs uppercase tracking-widest text-primary-z6">AI coding · measurable · accountable</p>
    <h1 class="mb-5 text-4xl font-bold leading-tight tracking-tight text-surface-z8 sm:text-5xl">
      Know exactly how well<br />your AI is coding.
    </h1>
    <p class="mb-8 text-lg text-surface-z5">
      Sensei tracks every Claude Code session — first-try-right scores, cost, turns, and drift —
      so you stop guessing and start improving.
    </p>
    <div class="flex items-center gap-3">
      <a href="/auth/signin" class="rounded-md bg-primary-z6 px-5 py-2.5 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors">
        Get started
      </a>
      <a href="#how" class="rounded-md border border-surface-z4 px-5 py-2.5 text-sm font-semibold text-surface-z7 hover:bg-surface-z2 transition-colors">
        See how it works
      </a>
    </div>
  </div>

  <!-- Terminal snippet -->
  <div class="mt-14 rounded-xl border border-surface-z3 bg-surface-z2 p-5 font-mono text-xs leading-relaxed overflow-x-auto">
    <div class="mb-3 flex items-center gap-1.5">
      <span class="h-2.5 w-2.5 rounded-full bg-error-z5"></span>
      <span class="h-2.5 w-2.5 rounded-full bg-warning-z5"></span>
      <span class="h-2.5 w-2.5 rounded-full bg-success-z5"></span>
    </div>
    <div class="space-y-1 text-surface-z5">
      <p><span class="text-primary-z6">$</span> <span class="text-surface-z8">sensei init</span></p>
      <p class="text-surface-z4">  ✓ Connected to repo: dashboard</p>
      <p class="text-surface-z4">  ✓ Supabase session store configured</p>
      <p class="text-surface-z4">  ✓ Watching .sensei/sessions/</p>
      <p class="mt-3"><span class="text-primary-z6">$</span> <span class="text-surface-z8">sensei report --last 7d</span></p>
      <p class="text-surface-z4 mt-1">  Sessions: 24 &nbsp; Avg FTR: <span class="text-success-z6">91%</span> &nbsp; Spend: $24.30</p>
      <p class="text-surface-z4">  ┌───────────────────────────────────────────┬───────┬───────┐</p>
      <p class="text-surface-z4">  │ Task                                      │  FTR  │  Cost │</p>
      <p class="text-surface-z4">  ├───────────────────────────────────────────┼───────┼───────┤</p>
      <p class="text-surface-z4">  │ Fix SSR build failures                    │ <span class="text-success-z6">100%</span>  │ $1.20 │</p>
      <p class="text-surface-z4">  │ Add auth OTP flow                         │  <span class="text-success-z6">95%</span>  │ $3.40 │</p>
      <p class="text-surface-z4">  │ Migrate jsdom → linkedom                  │  <span class="text-warning-z6">82%</span>  │ $2.80 │</p>
      <p class="text-surface-z4">  └───────────────────────────────────────────┴───────┴───────┘</p>
    </div>
  </div>
</section>

<!-- Features -->
<section id="features" class="border-t border-surface-z3 bg-surface-z2 py-16">
  <div class="mx-auto max-w-5xl px-6">
    <h2 class="mb-10 text-2xl font-bold text-surface-z8">Everything you need to manage AI-assisted development</h2>
    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
      {#each [
        { title: 'First-Try-Right Score',    body: 'Track whether Claude got it right without back-and-forth. The only metric that actually measures coding quality.' },
        { title: 'Session Cost Tracking',    body: 'Precise per-session Claude API costs. See where spend is going — by task, repo, or model.' },
        { title: 'Context Pack Management',  body: 'Curate the context Claude sees. Keep packs lean and targeted — better context = better FTR.' },
        { title: 'Library Intelligence',     body: 'Index third-party library docs. Claude knows your exact versions, not hallucinated APIs.' },
        { title: 'Doc Drift Detection',      body: 'Alerts when code drifts from specs. Keeps designs and implementations in sync automatically.' },
        { title: 'Skill Benchmarking',       body: 'Compare skill variants head-to-head. Retire the ones that hurt FTR, promote what works.' },
      ] as f}
        <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-5">
          <h3 class="mb-2 text-sm font-semibold text-surface-z8">{f.title}</h3>
          <p class="text-sm text-surface-z5">{f.body}</p>
        </div>
      {/each}
    </div>
  </div>
</section>

<!-- How it works -->
<section id="how" class="py-16">
  <div class="mx-auto max-w-5xl px-6">
    <h2 class="mb-10 text-2xl font-bold text-surface-z8">How it works</h2>
    <div class="grid grid-cols-1 gap-8 sm:grid-cols-3">
      {#each [
        { step: '01', title: 'Connect your repo',   body: 'Run sensei init in your project. It hooks into Claude Code sessions and your Supabase instance.' },
        { step: '02', title: 'Code as usual',        body: 'Sensei silently records every session — tasks, turns, cost, and whether you needed to redo work.' },
        { step: '03', title: 'Review and improve',   body: 'Open the dashboard. See patterns, refine context packs, compare model options, track improvement.' },
      ] as s}
        <div class="flex gap-4">
          <span class="mt-0.5 shrink-0 font-mono text-xs font-bold text-primary-z5">{s.step}</span>
          <div>
            <h3 class="mb-1.5 text-sm font-semibold text-surface-z8">{s.title}</h3>
            <p class="text-sm text-surface-z5">{s.body}</p>
          </div>
        </div>
      {/each}
    </div>
  </div>
</section>

<!-- CTA -->
<section class="border-t border-surface-z3 bg-surface-z2 py-16">
  <div class="mx-auto max-w-5xl px-6 text-center">
    <h2 class="mb-4 text-2xl font-bold text-surface-z8">Start measuring what matters</h2>
    <p class="mb-7 text-surface-z5">Connect your first repo in under two minutes.</p>
    <a href="/auth/signin" class="inline-block rounded-md bg-primary-z6 px-6 py-3 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors">
      Sign in to get started
    </a>
  </div>
</section>

<!-- Footer -->
<footer class="border-t border-surface-z3 py-6">
  <div class="mx-auto max-w-5xl px-6 flex items-center justify-between text-xs text-surface-z4">
    <span>sensei · AI coding analytics</span>
    <nav class="flex gap-4">
      <a href="#features" class="hover:text-surface-z6 transition-colors">Features</a>
      <a href="#how" class="hover:text-surface-z6 transition-colors">How it works</a>
      <a href="/auth/signin" class="hover:text-surface-z6 transition-colors">Sign in</a>
    </nav>
  </div>
</footer>
