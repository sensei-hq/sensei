<script lang="ts">
  import { wizardState, familyIsConfigured } from '$lib/wizard-state.svelte.js';

  const assistantsConfigured = $derived(
    wizardState.assistants.assistants.filter(familyIsConfigured).length,
  );
  const rootsCount = $derived(wizardState.roots.roots.length);
  const projectsCount = $derived(wizardState.projects.projects.length);
  const foldersCount = $derived(
    wizardState.projects.projects.reduce((n, p) => n + p.folders.length, 0),
  );
  const libsWrapped = $derived(wizardState.libraries.libs.filter(l => l.enabled).length);
  const mcpsSelected = $derived(wizardState.instruments.mcps.filter(m => m.selected).length);
  const mcpsInstalled = $derived(wizardState.instruments.mcps.filter(m => m.installed).length);

  const summary = $derived([
    {
      kanji: '連',
      label: assistantsConfigured === 0 ? 'No assistants' : `${assistantsConfigured} assistant${assistantsConfigured === 1 ? '' : 's'}`,
      sub: assistantsConfigured === 0
        ? 'install Claude/Cursor/etc. to enable sessions'
        : 'configured with sensei',
    },
    {
      kanji: '庵',
      label: rootsCount === 0 ? 'No roots' : `${rootsCount} root${rootsCount === 1 ? '' : 's'}`,
      sub: foldersCount > 0
        ? `${foldersCount} folder${foldersCount === 1 ? '' : 's'} across ${projectsCount} project${projectsCount === 1 ? '' : 's'}`
        : 'no folders scanned yet',
    },
    {
      kanji: '書',
      label: libsWrapped === 0 ? 'No libraries' : `${libsWrapped} librar${libsWrapped === 1 ? 'y' : 'ies'}`,
      sub: libsWrapped === 0 ? 'nothing to wrap yet' : 'docs & code indexed by sensei',
    },
    {
      kanji: '器',
      label: mcpsSelected === 0 ? 'No instruments' : `${mcpsSelected} instrument${mcpsSelected === 1 ? '' : 's'}`,
      sub: mcpsInstalled > 0
        ? `${mcpsInstalled} already installed in your assistants`
        : 'recommended for your stack',
    },
  ]);
</script>

<div class="max-w-[720px]">
  <h1
    class="display text-4xl font-light leading-tight m-0 mb-8 tracking-tight"
  >
    The observatory<br />
    <span class="text-primary-z5">is ready.</span>
  </h1>

  <p class="text-base text-surface-z7 leading-loose max-w-[560px] m-0 mb-10">
    Sensei is watching. Work normally with your AI assistants — sensei
    learns from every session and begins teaching after it has enough data.
  </p>

  <!-- What was configured -->
  <div class="text-xs uppercase tracking-wider text-surface-z6 mb-3">
    What you set up
  </div>
  <div
    data-testid="done-summary"
    class="grid grid-cols-2 gap-3 mb-10"
  >
    {#each summary as s}
      <div class="flex items-start gap-3 bg-surface-z2 border border-surface-z3 rounded-md p-4">
        <span class="kanji text-2xl text-primary-z6 leading-none mt-1">{s.kanji}</span>
        <div class="min-w-0">
          <div class="text-sm text-surface-z9 font-medium">{s.label}</div>
          <div class="text-xs text-surface-z6 mt-0.5">{s.sub}</div>
        </div>
      </div>
    {/each}
  </div>

  <!-- What happens next -->
  <div class="text-xs uppercase tracking-wider text-surface-z6 mb-3">
    What happens next
  </div>
  <div class="grid grid-cols-3 gap-4 py-5 border-t border-b border-surface-z2">
    {#each [
      { kanji: '観', label: 'Observing', sub: 'sessions tracked in real time' },
      { kanji: '師', label: 'Teaching', sub: 'after ~3 sessions per project' },
      { kanji: '静', label: 'Local', sub: 'nothing leaves your machine' },
    ] as s}
      <div>
        <div class="kanji text-2xl text-primary-z5 mb-2">{s.kanji}</div>
        <div class="display text-base">{s.label}</div>
        <div class="text-xs text-surface-z6 mt-1">{s.sub}</div>
      </div>
    {/each}
  </div>
</div>
