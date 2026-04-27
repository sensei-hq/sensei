<script lang="ts">
  let { title = 'Sensei', width = 720, height = 460, children } = $props<{
    title?: string;
    width?: number;
    height?: number;
    children: any;
  }>();
</script>

<div class="app-frame" style="width: {width}px; height: {height}px; max-width: 100%;">
  <div class="chrome">
    <span class="traffic">
      <span class="dot close"></span>
      <span class="dot minimize"></span>
      <span class="dot maximize"></span>
    </span>
    <div class="title">{title}</div>
    <span style="width: 30px;"></span>
  </div>
  <div class="body">
    {@render children()}
  </div>
</div>

<style>
  .app-frame {
    border-radius: 12px;
    background: var(--paper);
    border: var(--hairline);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: 0 30px 60px -20px rgba(20,18,14,0.18), 0 12px 24px -12px rgba(20,18,14,0.10);
  }
  .chrome {
    height: 32px;
    padding: 0 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: var(--hairline);
    background: var(--paper);
    flex-shrink: 0;
  }
  .traffic {
    display: flex;
    gap: 6px;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .dot.close { background: oklch(0.72 0.14 28); }
  .dot.minimize { background: oklch(0.82 0.13 85); }
  .dot.maximize { background: oklch(0.72 0.11 145); }
  .title {
    flex: 1;
    text-align: center;
    font-size: 11px;
    color: var(--sumi-3);
    letter-spacing: 0.02em;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
</style>
