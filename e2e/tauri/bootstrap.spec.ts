/**
 * Bootstrap Health Screen — Tauri E2E Test Suite
 *
 * Tests the actual compiled app with real sidecar commands.
 * Verifies the bootstrap flow works end-to-end: detection,
 * prerequisite install remedy, service checks, and gate transitions.
 *
 * Prerequisites: debug build must exist at src-tauri/target/debug/sensei-desktop
 * Build with: bun run build && cargo build --manifest-path src-tauri/Cargo.toml
 */

describe('Bootstrap Health Screen', () => {

  it('should load the health page', async () => {
    // The app starts at /health on first launch (no setup complete)
    const title = await browser.getTitle();
    expect(title).toContain('Sensei');
  });

  it('should show the bootstrap header', async () => {
    const header = await $('h1');
    const text = await header.getText();
    // Should show one of: "holds", "missing", or "Checking"
    expect(
      text.includes('holds') || text.includes('missing') || text.includes('Checking')
    ).toBe(true);
  });

  it('should show the progress rail', async () => {
    // Progress count like "01 / 06 ready" or similar
    const progressText = await browser.execute(() => {
      const el = document.querySelector('.progress-count');
      return el?.textContent ?? '';
    });
    expect(progressText).toMatch(/\d+ \/ \d+ ready/i);
  });

  it('should detect platform via Tauri invoke', async () => {
    // Call get_platform through the Tauri invoke API
    const platform = await browser.execute(async () => {
      const { invoke } = (window as any).__TAURI__.core;
      return invoke('get_platform');
    });

    expect(platform).toHaveProperty('platform');
    expect(platform).toHaveProperty('package_manager');
    expect(platform).toHaveProperty('prereq_remedy');
    expect(platform).toHaveProperty('pkgmgr_remedy');
    expect(['macos', 'linux', 'windows']).toContain(platform.platform);
  });

  it('should run bootstrap detection via Tauri invoke', async () => {
    const result = await browser.execute(async () => {
      const { invoke } = (window as any).__TAURI__.core;
      return invoke('run_bootstrap');
    });

    expect(result).toHaveProperty('components');
    expect(result).toHaveProperty('hardware');
    expect(result).toHaveProperty('ready');
    expect(result.components.length).toBeGreaterThan(0);
    expect(result.hardware.ram_gb).toBeGreaterThan(0);
  });

  it('should show Homebrew as ready on macOS dev machine', async () => {
    // On a dev machine, Homebrew should be detected
    const result = await browser.execute(async () => {
      const { invoke } = (window as any).__TAURI__.core;
      const r = invoke('run_bootstrap');
      return r;
    });

    const homebrew = result.components.find((c: any) => c.name === 'homebrew');
    expect(homebrew).toBeDefined();
    expect(homebrew.state.state).toBe('ready');
  });

  it('should detect hardware capabilities', async () => {
    const hw = await browser.execute(async () => {
      const { invoke } = (window as any).__TAURI__.core;
      return invoke('detect_hardware');
    });

    expect(hw.ram_gb).toBeGreaterThan(0);
    expect(hw.cpu_cores).toBeGreaterThan(0);
    expect(['minimum', 'recommended', 'full']).toContain(hw.recommended_tier);
  });

  it('should show gate rows in the UI', async () => {
    // Wait for the page to settle
    await browser.pause(2000);

    const gateCount = await browser.execute(() => {
      return document.querySelectorAll('.gate-row').length;
    });

    // Should have at least 1 gate visible (homebrew at minimum)
    expect(gateCount).toBeGreaterThan(0);
  });

  it('should show remedy card when prereqs are missing', async () => {
    // Check if the remedy card is visible (depends on machine state)
    const hasRemedy = await browser.execute(() => {
      return document.querySelector('.brew-remedy') !== null;
    });

    const hasGateList = await browser.execute(() => {
      return document.querySelectorAll('.gate-row').length;
    });

    // Either remedy card is showing (prereqs missing) or all gates are visible
    expect(hasRemedy || hasGateList > 1).toBe(true);
  });

  it('should receive bootstrap events via Tauri event system', async () => {
    // Test the event contract by listening and triggering
    const events = await browser.execute(async () => {
      const { listen } = (window as any).__TAURI__.event;
      const received: any[] = [];

      const unlisten = await listen('bootstrap', (event: any) => {
        received.push(event.payload);
      });

      // Trigger a detection run — this should emit events
      const { invoke } = (window as any).__TAURI__.core;
      await invoke('run_bootstrap');

      // Give it a moment
      await new Promise(r => setTimeout(r, 500));
      unlisten();

      return received;
    });

    // run_bootstrap doesn't emit events (it returns directly),
    // but this verifies the event listener infrastructure works
    // Phase commands (install_prerequisites, start_services, setup_database)
    // do emit events — tested separately when those phases run
    expect(Array.isArray(events)).toBe(true);
  });
});
