<script lang="ts">
  import { goto } from '$app/navigation';
  import { AuthProvider } from '@kavach/ui';
  import { providers } from '$kavach/providers';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';

  function onSuccess() {
    goto('/home');
  }
</script>

<div class="flex min-h-screen" use:themable={vibe}>

  <!-- Left panel: branding (hidden on mobile) -->
  <div class="hidden lg:flex lg:w-1/2 flex-col justify-between bg-surface-z1 border-r border-surface-z0 p-12">
    <!-- Logo -->
    <div class="flex items-center gap-3">
      <div class="flex h-9 w-9 items-center justify-center rounded-xl bg-primary-z6 text-lg font-bold text-white">⬡</div>
      <span class="text-xl font-bold text-surface-z8">sensei</span>
    </div>

    <!-- Value props -->
    <div>
      <h1 class="text-4xl font-black text-surface-z8 leading-tight mb-4">
        AI-powered<br>codebase intelligence
      </h1>
      <p class="text-surface-z5 text-lg mb-10">
        Track first-try-right scores, understand your repos, and coach your team to write better code with AI.
      </p>

      <ul class="space-y-5">
        {#each [
          { icon: '📊', title: 'First-Try-Right analytics', desc: 'See what percentage of AI-assisted tasks succeed on the first attempt, per developer and per repo.' },
          { icon: '🧠', title: 'Context-aware indexing',    desc: 'Semantic search across your entire codebase — symbols, docs, and memory — always in context.' },
          { icon: '🤝', title: 'Team-level insights',       desc: 'Compare FTR across contributors, spot knowledge gaps, and onboard new developers faster.' },
        ] as item}
          <li class="flex items-start gap-4">
            <span class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-surface-z2 text-xl border border-surface-z0">{item.icon}</span>
            <div>
              <p class="font-semibold text-surface-z8">{item.title}</p>
              <p class="mt-0.5 text-sm text-surface-z5">{item.desc}</p>
            </div>
          </li>
        {/each}
      </ul>
    </div>

    <!-- Testimonial -->
    <div class="rounded-xl border border-surface-z0 bg-surface-z2 px-6 py-5">
      <p class="text-sm text-surface-z5 italic">"sensei gave our team visibility we never had before — we went from 61% to 84% first-try-right in 60 days."</p>
      <div class="mt-3 flex items-center gap-2.5">
        <div class="flex h-7 w-7 items-center justify-center rounded-full bg-success-z6 text-xs font-bold text-white">AC</div>
        <div>
          <p class="text-xs font-semibold text-surface-z7">Alice Chen</p>
          <p class="text-xs text-surface-z4">Engineering Lead, Acme Corp</p>
        </div>
      </div>
    </div>
  </div>

  <!-- Right panel: login form -->
  <div class="flex flex-1 flex-col items-center justify-center px-6 py-12 bg-surface-z0">

    <!-- Mobile logo -->
    <div class="mb-8 flex items-center gap-2.5 lg:hidden">
      <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary-z6 text-base font-bold text-white">⬡</div>
      <span class="text-xl font-bold text-surface-z8">sensei</span>
    </div>

    <div class="w-full max-w-sm">
      <div class="mb-8">
        <h2 class="text-2xl font-black text-surface-z8">Welcome back</h2>
        <p class="mt-1 text-sm text-surface-z5">Sign in to your sensei account</p>
      </div>

      <div class="flex flex-col gap-4 rounded-2xl border border-surface-z3 bg-surface-z1 p-6 shadow-sm">
        {#each providers as p (p.name)}
          <AuthProvider
            name={p.name}
            mode={p.mode ?? 'oauth'}
            onsuccess={onSuccess}
            label={p.label}
          />
        {/each}
      </div>

      <p class="mt-6 text-center text-xs text-surface-z4">
        Don't have an account?
        <a href="/" class="font-medium text-primary-z6 hover:text-primary-z7">Learn more →</a>
      </p>
    </div>
  </div>
</div>
