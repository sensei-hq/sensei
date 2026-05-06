<script lang="ts">
  // Observatory — "Today" daily view
  // Placeholder until the full mockup is implemented.
</script>

<div class="max-w-[720px] mx-auto px-12 py-12 pb-16">
  <div class="mb-10">
    <p class="text-2xs tracking-loose uppercase text-surface-z6 m-0 mb-2">
      {new Date().toLocaleDateString('en-US', { weekday: 'short', day: 'numeric', month: 'short' })}
    </p>
    <h1 class="display text-3xl font-normal m-0 tracking-tight">The observatory is open.</h1>
  </div>

  <div class="grid grid-cols-[auto_1fr] gap-7 px-8.5 py-8 pb-7.5 bg-surface-z2 border border-surface-z3 rounded-lg mb-8">
    <span class="kanji text-7xl text-primary-z5 opacity-55 leading-none">観</span>
    <div>
      <p class="display text-2xl font-normal m-0 mb-3 tracking-tight">Still listening.</p>
      <p class="text-sm text-surface-z7 leading-relaxed m-0 max-w-[520px]">
        Sensei is watching your sessions. A few early signals are forming,
        but nothing confident enough to teach yet.
      </p>
    </div>
  </div>

  <p class="text-xs text-surface-z6 leading-reading max-w-[480px]">
    Start a session with your assistant. Sensei watches in silence,
    learns the shape of each project, and later begins to teach.
  </p>
</div>
