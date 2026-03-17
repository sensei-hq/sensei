<script lang="ts">
  import { Tabs, BreadCrumbs, Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  let activeTab = $state('graph');

  const tabOptions = [
    { value: 'graph', label: 'Code Graph' },
    { value: 'complexity', label: 'Complexity' },
  ];

  const breadcrumbs = $derived([
    { label: 'Repos', href: '/repos' },
    { label: data.repo.name, href: `/repos/${data.repo.id}` },
    { label: 'Code Explorer' },
  ]);

  // Group nodes by package
  const nodesByPackage = $derived(() => {
    const groups = new Map<string, typeof data.graph.nodes>();
    for (const node of data.graph.nodes) {
      const pkg = node.file.split('/')[1] ?? 'root';
      if (!groups.has(pkg)) groups.set(pkg, []);
      groups.get(pkg)!.push(node);
    }
    return groups;
  });

  const nodeMap = $derived(
    new Map(data.graph.nodes.map(n => [n.id, n]))
  );

  const complexityColorClass = (c: number) => {
    if (c <= 5) return 'complexity-low';
    if (c <= 10) return 'complexity-mid';
    return 'complexity-high';
  };

  const kindBadgeClass = (kind: string) => {
    if (kind === 'function') return 'kind-function';
    if (kind === 'class') return 'kind-class';
    return 'kind-module';
  };

  // Complexity table
  const complexityColumns = [
    { name: 'file', label: 'File', sortable: true },
    { name: 'functions', label: 'Functions', sortable: true },
    { name: 'avgComplexity', label: 'Avg Complexity', sortable: true },
    { name: 'maxComplexity', label: 'Max Complexity', sortable: true },
    { name: 'lines', label: 'Lines', sortable: true },
    { name: 'riskLevel', label: 'Risk', sortable: true },
  ];

  const sortedComplexity = $derived(
    [...data.complexity].sort((a, b) => b.avgComplexity - a.avgComplexity)
  );

  const complexitySummary = $derived({
    total: data.complexity.length,
    high: data.complexity.filter(c => c.riskLevel === 'high').length,
    medium: data.complexity.filter(c => c.riskLevel === 'medium').length,
    low: data.complexity.filter(c => c.riskLevel === 'low').length,
  });
</script>

<div class="mb-4">
  <BreadCrumbs items={breadcrumbs} />
</div>

<div class="flex items-center gap-3 mb-6">
  <a href="/repos/{data.repo.id}" class="text-sm text-surface-z5 hover:text-surface-z7 transition-colors">
    ← {data.repo.name}
  </a>
  <span class="text-surface-z3">·</span>
  <h1 class="text-2xl font-semibold text-surface-z8">Code Explorer</h1>
</div>

<Tabs options={tabOptions} bind:value={activeTab} />

<div class="mt-6">
  {#if activeTab === 'graph'}
    <div class="grid gap-6" style="grid-template-columns: repeat(auto-fill, minmax(340px, 1fr))">
      {#each [...nodesByPackage()] as [pkg, nodes]}
        <div class="rounded-lg border border-surface-z3 bg-surface-z1 overflow-hidden">
          <div class="px-4 py-2 border-b border-surface-z3 bg-surface-z2">
            <span class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">packages/{pkg}</span>
          </div>
          <div class="p-3 flex flex-col gap-2">
            {#each nodes as node}
              <div class="p-3 rounded border border-surface-z3 bg-surface-z2">
                <div class="flex items-center justify-between gap-2 mb-1">
                  <span class="font-semibold text-surface-z8 text-sm">{node.name}</span>
                  <div class="flex items-center gap-1.5">
                    <span class="kind-badge {kindBadgeClass(node.kind)}">{node.kind}</span>
                    <span class="complexity-badge {complexityColorClass(node.complexity)}">{node.complexity}</span>
                  </div>
                </div>
                <code class="text-xs text-surface-z4 font-mono block mb-1.5 truncate">{node.file}</code>
                {#if node.deps.length > 0}
                  <div class="flex flex-wrap gap-1">
                    {#each node.deps as depId}
                      {@const dep = nodeMap.get(depId)}
                      {#if dep}
                        <span class="text-xs text-surface-z5">→ {dep.name}</span>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>

  {:else if activeTab === 'complexity'}
    <!-- Summary stat bar -->
    <div class="flex gap-4 mb-4 flex-wrap">
      <div class="p-3 rounded-lg border border-surface-z3 bg-surface-z1 text-center min-w-20">
        <div class="text-xl font-semibold text-primary-z6">{complexitySummary.total}</div>
        <div class="text-xs text-surface-z5 uppercase tracking-wider">Files</div>
      </div>
      <div class="p-3 rounded-lg border border-surface-z3 bg-surface-z1 text-center min-w-20">
        <div class="text-xl font-semibold risk-high-text">{complexitySummary.high}</div>
        <div class="text-xs text-surface-z5 uppercase tracking-wider">High Risk</div>
      </div>
      <div class="p-3 rounded-lg border border-surface-z3 bg-surface-z1 text-center min-w-20">
        <div class="text-xl font-semibold risk-medium-text">{complexitySummary.medium}</div>
        <div class="text-xs text-surface-z5 uppercase tracking-wider">Medium Risk</div>
      </div>
      <div class="p-3 rounded-lg border border-surface-z3 bg-surface-z1 text-center min-w-20">
        <div class="text-xl font-semibold risk-low-text">{complexitySummary.low}</div>
        <div class="text-xs text-surface-z5 uppercase tracking-wider">Low Risk</div>
      </div>
    </div>

    <Table data={sortedComplexity} columns={complexityColumns} />
  {/if}
</div>

<style>
  .kind-badge {
    padding: 1px 6px;
    border-radius: 9999px;
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: capitalize;
  }
  .kind-function { background: #dbeafe; color: #1d4ed8; }
  .kind-class { background: #ede9fe; color: #5b21b6; }
  .kind-module { background: #f1f5f9; color: #475569; }

  .complexity-badge {
    padding: 1px 6px;
    border-radius: 9999px;
    font-size: 0.65rem;
    font-weight: 700;
    font-family: monospace;
  }
  .complexity-low { background: #dcfce7; color: #166534; }
  .complexity-mid { background: #fef3c7; color: #92400e; }
  .complexity-high { background: #fee2e2; color: #991b1b; }

  .risk-high-text { color: #991b1b; }
  .risk-medium-text { color: #92400e; }
  .risk-low-text { color: #166534; }
</style>
