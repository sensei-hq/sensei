<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getSolutionById } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';

  let solution = $derived(getSolutionById($page.params.id as string));
  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));
  let loading = $state(true);
  let showGapsOnly = $state(false);

  type DocType = 'requirement' | 'design' | 'api-spec' | 'changelog' | 'operations' | 'overview' | 'doc';

  interface TracedDoc {
    path: string;
    title: string;
    type: DocType;
    coveredFiles: string[];
    mentionedFns: string[];
    repoLabel: string;
    repoId: string;
  }

  interface CodeFile {
    path: string;
    isTest: boolean;
    repoLabel: string;
  }

  let allDocs = $state<TracedDoc[]>([]);
  let allCodeFiles = $state<CodeFile[]>([]);

  // Build trace chains
  let traceRows = $derived(buildTraceRows());

  interface TraceRow {
    requirement: TracedDoc | null;
    design: TracedDoc | null;
    codeFiles: string[];
    testCount: number;
    hasGap: boolean;
    label: string;
  }

  function buildTraceRows(): TraceRow[] {
    const requirements = allDocs.filter(d => d.type === 'requirement');
    const designs = allDocs.filter(d => d.type === 'design');
    const testFiles = new Set(allCodeFiles.filter(f => f.isTest).map(f => f.path));
    const rows: TraceRow[] = [];

    // Start from requirements
    for (const req of requirements) {
      // Find designs that cover the same files
      const reqFiles = new Set(req.coveredFiles);
      const relatedDesigns = designs.filter(d => d.coveredFiles.some(f => reqFiles.has(f)));
      const design = relatedDesigns[0] ?? null;

      // Merge covered files from requirement + design
      const allCovered = new Set([...req.coveredFiles, ...(design?.coveredFiles ?? [])]);
      const codeFiles = [...allCovered].filter(f => !testFiles.has(f));

      // Count test files that are related (match the code file paths with test patterns)
      const tests = codeFiles.filter(cf => {
        const base = cf.replace(/\.[^.]+$/, '');
        return testFiles.has(`${base}.spec.ts`) || testFiles.has(`${base}.test.ts`) ||
               testFiles.has(`${base}.spec.tsx`) || testFiles.has(`${base}.test.tsx`) ||
               testFiles.has(`${base}.spec.js`) || testFiles.has(`${base}.test.js`);
      });

      rows.push({
        requirement: req,
        design,
        codeFiles,
        testCount: tests.length,
        hasGap: codeFiles.length === 0 || tests.length === 0 || !design,
        label: req.title,
      });
    }

    // Designs without requirements
    const usedDesigns = new Set(rows.filter(r => r.design).map(r => r.design!.path));
    for (const design of designs) {
      if (usedDesigns.has(design.path)) continue;
      const codeFiles = design.coveredFiles.filter(f => !testFiles.has(f));
      rows.push({
        requirement: null,
        design,
        codeFiles,
        testCount: 0,
        hasGap: true,
        label: design.title,
      });
    }

    return showGapsOnly ? rows.filter(r => r.hasGap) : rows;
  }

  // Stats
  let totalDocs = $derived(allDocs.length);
  let reqCount = $derived(allDocs.filter(d => d.type === 'requirement').length);
  let designCount = $derived(allDocs.filter(d => d.type === 'design').length);
  let gapCount = $derived(traceRows.filter(r => r.hasGap).length);
  let coveragePercent = $derived(traceRows.length > 0 ? Math.round((traceRows.filter(r => !r.hasGap).length / traceRows.length) * 100) : 0);

  const TYPE_CLS: Record<string, string> = {
    requirement: 'bg-primary-z2 text-primary-z7',
    design: 'bg-info-z2 text-info-z7',
    'api-spec': 'bg-secondary-z2 text-secondary-z7',
    changelog: 'bg-surface-z3 text-surface-z5',
    operations: 'bg-warning-z2 text-warning-z7',
    overview: 'bg-surface-z3 text-surface-z6',
    doc: 'bg-surface-z3 text-surface-z5',
  };

  async function loadTrace() {
    if (!solution) return;
    loading = true;
    const api = senseiApi(port);

    // Load trace data from each repo in the solution
    const docs: TracedDoc[] = [];
    const files: CodeFile[] = [];

    await Promise.all(solution.repos.map(async (repo) => {
      try {
        const res = await fetch(`http://127.0.0.1:${port}/api/trace?repoId=${encodeURIComponent(repo.repoId)}&repoPath=${encodeURIComponent(repo.path)}`);
        if (!res.ok) return;
        const data = await res.json() as { docs: any[]; codeFiles: any[] };
        for (const d of data.docs) {
          docs.push({ ...d, repoLabel: repo.label ?? repo.path.split('/').at(-1), repoId: repo.repoId });
        }
        for (const f of data.codeFiles) {
          files.push({ ...f, repoLabel: repo.label ?? repo.path.split('/').at(-1) });
        }
      } catch { /* ignore */ }
    }));

    allDocs = docs;
    allCodeFiles = files;
    loading = false;
  }

  onMount(() => { loadTrace(); });
</script>

{#if solution}
  <div class="flex-1 overflow-y-auto px-6 py-5 space-y-6">

    <!-- Header + filter -->
    <div class="flex items-center justify-between">
      <div>
        <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide">Traceability</h3>
        <p class="text-[10px] text-surface-z3 mt-0.5">Requirements → Design → Code → Tests</p>
      </div>
      <label class="flex items-center gap-2 text-xs text-surface-z5">
        <input type="checkbox" bind:checked={showGapsOnly} class="rounded" />
        Gaps only
      </label>
    </div>

    {#if loading}
      <p class="text-sm text-surface-z4 text-center py-12">Loading traceability data…</p>
    {:else if allDocs.length === 0}
      <div class="text-center py-12">
        <span class="text-4xl i-solar-document-text-bold-duotone text-surface-z3"></span>
        <p class="text-sm text-surface-z4 mt-3">No documentation found.</p>
        <p class="text-xs text-surface-z3 mt-1">Add .md/.mdx files to your repos and re-index to see traceability.</p>
      </div>
    {:else}

      <!-- Stats -->
      <div class="grid grid-cols-5 gap-3">
        <div class="rounded-lg bg-surface-z2 p-3">
          <p class="text-[10px] text-surface-z4 uppercase">Docs</p>
          <p class="mt-1 text-lg font-semibold text-surface-z8">{totalDocs}</p>
        </div>
        <div class="rounded-lg bg-surface-z2 p-3">
          <p class="text-[10px] text-surface-z4 uppercase">Requirements</p>
          <p class="mt-1 text-lg font-semibold text-primary-z6">{reqCount}</p>
        </div>
        <div class="rounded-lg bg-surface-z2 p-3">
          <p class="text-[10px] text-surface-z4 uppercase">Designs</p>
          <p class="mt-1 text-lg font-semibold text-info-z6">{designCount}</p>
        </div>
        <div class="rounded-lg bg-surface-z2 p-3">
          <p class="text-[10px] text-surface-z4 uppercase">Coverage</p>
          <p class="mt-1 text-lg font-semibold {coveragePercent >= 80 ? 'text-success-z6' : coveragePercent >= 50 ? 'text-warning-z6' : 'text-error-z6'}">{coveragePercent}%</p>
        </div>
        <div class="rounded-lg bg-surface-z2 p-3">
          <p class="text-[10px] text-surface-z4 uppercase">Gaps</p>
          <p class="mt-1 text-lg font-semibold {gapCount > 0 ? 'text-warning-z6' : 'text-success-z6'}">{gapCount}</p>
        </div>
      </div>

      <!-- Trace table -->
      {#if traceRows.length > 0}
        <div class="rounded-lg border border-surface-z0/50 overflow-hidden">
          <!-- Header -->
          <div class="grid grid-cols-4 gap-px bg-surface-z3/50 text-[10px] font-semibold text-surface-z4 uppercase tracking-wide">
            <div class="bg-surface-z2 px-3 py-2">Requirement</div>
            <div class="bg-surface-z2 px-3 py-2">Design</div>
            <div class="bg-surface-z2 px-3 py-2">Code</div>
            <div class="bg-surface-z2 px-3 py-2">Tests</div>
          </div>

          <!-- Rows -->
          {#each traceRows as row}
            <div class="grid grid-cols-4 gap-px bg-surface-z3/30 {row.hasGap ? 'ring-1 ring-warning-z3/50' : ''}">
              <!-- Requirement -->
              <div class="bg-surface-z1 px-3 py-2.5 min-h-10">
                {#if row.requirement}
                  <p class="text-xs text-surface-z7 truncate">{row.requirement.title}</p>
                  <p class="text-[10px] text-surface-z3 truncate">{row.requirement.path}</p>
                {:else}
                  <span class="text-[10px] text-warning-z5">missing</span>
                {/if}
              </div>
              <!-- Design -->
              <div class="bg-surface-z1 px-3 py-2.5 min-h-10">
                {#if row.design}
                  <p class="text-xs text-surface-z7 truncate">{row.design.title}</p>
                  <p class="text-[10px] text-surface-z3 truncate">{row.design.path}</p>
                {:else}
                  <span class="text-[10px] text-warning-z5">missing</span>
                {/if}
              </div>
              <!-- Code -->
              <div class="bg-surface-z1 px-3 py-2.5 min-h-10">
                {#if row.codeFiles.length > 0}
                  <p class="text-xs text-surface-z7">{row.codeFiles.length} file{row.codeFiles.length === 1 ? '' : 's'}</p>
                  <p class="text-[10px] text-surface-z3 truncate">{row.codeFiles[0]}</p>
                {:else}
                  <span class="text-[10px] text-warning-z5">no code linked</span>
                {/if}
              </div>
              <!-- Tests -->
              <div class="bg-surface-z1 px-3 py-2.5 min-h-10">
                {#if row.testCount > 0}
                  <span class="text-xs text-success-z6">{row.testCount} tested</span>
                {:else}
                  <span class="text-[10px] text-error-z5">0 tests</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-xs text-surface-z4 text-center py-8">No traceable requirements or designs found.</p>
      {/if}

      <!-- All docs by type -->
      <div>
        <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">All Documents</h3>
        <div class="space-y-1">
          {#each allDocs as doc}
            <div class="flex items-center gap-2 rounded-lg bg-surface-z2/50 px-3 py-2 text-xs">
              <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {TYPE_CLS[doc.type] ?? TYPE_CLS.doc}">{doc.type}</span>
              <span class="text-surface-z7 truncate flex-1">{doc.title}</span>
              <span class="text-surface-z3 truncate max-w-40">{doc.path}</span>
              {#if doc.coveredFiles.length > 0}
                <span class="text-[10px] text-surface-z4">{doc.coveredFiles.length} files</span>
              {/if}
              <span class="rounded bg-surface-z3 px-1 py-0.5 text-[9px] text-surface-z5">{doc.repoLabel}</span>
            </div>
          {/each}
        </div>
      </div>

    {/if}
  </div>
{/if}
