<script lang="ts">
  import { page } from '$app/stores';

  let { data, children } = $props();

  const SECTIONS = [
    { id: 'overview',      kanji: '観', label: 'Overview',      href: () => `/project/${data.projectId}/overview` },
    { id: 'sessions',      kanji: '刻', label: 'Sessions',      href: () => `/project/${data.projectId}/sessions` },
    { id: 'memories',      kanji: '憶', label: 'Memories',      href: () => `/project/${data.projectId}/memories` },
    { id: 'traceability',  kanji: '跡', label: 'Traceability',  href: () => `/project/${data.projectId}/traceability` },
    { id: 'libraries',     kanji: '書', label: 'Libraries',     href: () => `/project/${data.projectId}/libraries` },
    { id: 'instruments',   kanji: '具', label: 'Instruments',   href: () => `/project/${data.projectId}/instruments` },
    { id: 'patterns',      kanji: '型', label: 'Patterns',      href: () => `/project/${data.projectId}/patterns` },
    { id: 'impact',        kanji: '響', label: 'Impact',        href: () => `/project/${data.projectId}/impact` },
    { id: 'about',         kanji: '事', label: 'About',         href: () => `/project/${data.projectId}/about` },
  ];

  function isActive(sectionId: string): boolean {
    return $page.url.pathname.includes(`/${sectionId}`);
  }

  let kanji = $derived(data.project?.icon?.value ?? '場');
  let ftr = $derived(Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100));
</script>

<div class="project-shell">
  <!-- 2px shu accent stripe (PerspectiveChrome) -->
  <div class="accent-stripe"></div>

  <!-- Titlebar / drag region -->
  <div class="titlebar drag-region">
    <span class="proj-kanji">{kanji}</span>
    <span class="proj-name">{data.project?.name ?? '…'}</span>
    <span class="proj-sub">· project window</span>
  </div>

  <div class="shell-body">
    <aside class="proj-sidebar">
      <div class="sidebar-stats">
        <span class="stat-value">{ftr}%</span>
        <span class="stat-label">FTR 14d</span>
      </div>

      <nav class="proj-nav">
        {#each SECTIONS as section (section.id)}
          {@const active = isActive(section.id)}
          <a href={section.href()} class="proj-nav-item" class:active>
            <span class="kanji">{section.kanji}</span>
            <span class="label">{section.label}</span>
          </a>
        {/each}
      </nav>
    </aside>

    <main class="proj-content">
      {@render children()}
    </main>
  </div>
</div>

<style>
  .project-shell { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }
  .accent-stripe { height: 2px; background: var(--shu, #c0392b); flex-shrink: 0; }
  .titlebar { height: 36px; display: flex; align-items: center; gap: 8px; padding: 0 16px; flex-shrink: 0; }
  .proj-kanji { font-size: 18px; color: var(--shu, #c0392b); }
  .proj-name { font-size: 14px; font-weight: 600; }
  .proj-sub { font-size: 11px; opacity: 0.5; }
  .shell-body { display: flex; flex: 1; overflow: hidden; }
  .proj-sidebar { width: 180px; flex-shrink: 0; border-right: 1px solid var(--border); display: flex; flex-direction: column; padding: 12px 0; }
  .sidebar-stats { padding: 8px 16px 16px; }
  .stat-value { font-size: 22px; font-weight: 700; display: block; }
  .stat-label { font-size: 11px; opacity: 0.5; }
  .proj-nav { display: flex; flex-direction: column; }
  .proj-nav-item { display: flex; align-items: center; gap: 10px; padding: 7px 16px; text-decoration: none; color: inherit; font-size: 13px; }
  .proj-nav-item.active { background: var(--surface-2); color: var(--shu, #c0392b); }
  .proj-nav-item .kanji { width: 18px; text-align: center; }
  .proj-content { flex: 1; overflow-y: auto; }
</style>
