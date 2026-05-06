/**
 * Tests for bootstrap state helpers and type logic.
 * These are pure functions — no DOM, no Tauri, no network.
 */
import { describe, it, expect } from 'vitest';
import {
  stateLabel, isReady, isFailed, errorMessage,
  type ComponentStatus, type ComponentState, type BootstrapResult,
} from './bootstrap.js';

// ── Factory helpers ──────────────────────────────────────────────────────────

function ready(name: string, version = '1.0'): ComponentStatus {
  return { name, state: { state: 'ready' }, version, detail: null };
}

function failed(name: string, error: string): ComponentStatus {
  return { name, state: { state: 'failed', error }, version: null, detail: null };
}

function skipped(name: string): ComponentStatus {
  return { name, state: { state: 'skipped' }, version: null, detail: null };
}

function detecting(name: string): ComponentStatus {
  return { name, state: { state: 'detecting' }, version: null, detail: null };
}

function installing(name: string): ComponentStatus {
  return { name, state: { state: 'installing' }, version: null, detail: null };
}

function pulling(name: string, pct: number, size: number): ComponentStatus {
  return { name, state: { state: 'pulling', progress_pct: pct, size_mb: size }, version: null, detail: null };
}

const defaultHardware = {
  ram_gb: 16, cpu_cores: 8, gpu: 'Apple M2', metal_support: true,
  recommended_tier: 'recommended' as const,
};

// ── State label ──────────────────────────────────────────────────────────────

describe('stateLabel', () => {
  it('returns ready for ready state', () => {
    expect(stateLabel({ state: 'ready' })).toBe('ready');
  });

  it('returns failed for failed state', () => {
    expect(stateLabel({ state: 'failed', error: 'oops' })).toBe('failed');
  });

  it('returns detecting for detecting state', () => {
    expect(stateLabel({ state: 'detecting' })).toBe('detecting');
  });

  it('returns pulling for pulling state', () => {
    expect(stateLabel({ state: 'pulling', progress_pct: 50, size_mb: 1024 })).toBe('pulling');
  });
});

// ── isReady ──────────────────────────────────────────────────────────────────

describe('isReady', () => {
  it('true for ready state', () => {
    expect(isReady({ state: 'ready' })).toBe(true);
  });

  it('false for failed state', () => {
    expect(isReady({ state: 'failed', error: 'x' })).toBe(false);
  });

  it('false for detecting state', () => {
    expect(isReady({ state: 'detecting' })).toBe(false);
  });

  it('false for skipped state', () => {
    expect(isReady({ state: 'skipped' })).toBe(false);
  });
});

// ── isFailed ─────────────────────────────────────────────────────────────────

describe('isFailed', () => {
  it('true for failed state', () => {
    expect(isFailed({ state: 'failed', error: 'nope' })).toBe(true);
  });

  it('false for ready state', () => {
    expect(isFailed({ state: 'ready' })).toBe(false);
  });

  it('false for skipped state', () => {
    expect(isFailed({ state: 'skipped' })).toBe(false);
  });
});

// ── errorMessage ─────────────────────────────────────────────────────────────

describe('errorMessage', () => {
  it('returns error string for failed state', () => {
    expect(errorMessage({ state: 'failed', error: 'port in use' })).toBe('port in use');
  });

  it('returns null for ready state', () => {
    expect(errorMessage({ state: 'ready' })).toBeNull();
  });

  it('returns null for detecting state', () => {
    expect(errorMessage({ state: 'detecting' })).toBeNull();
  });
});

// ── Full bootstrap result scenarios ──────────────────────────────────────────

describe('BootstrapResult scenarios', () => {
  it('all-ready system', () => {
    const result: BootstrapResult = {
      components: [
        ready('homebrew', '4.4.2'),
        ready('sensei', '0.9.4'),
        ready('postgresql@17', '17.2'),
        ready('ollama', '0.6.2'),
        ready('database', 'schema-42'),
        ready('daemon', '0.9.4'),
      ],
      hardware: defaultHardware,
      ready: true,
    };
    expect(result.ready).toBe(true);
    expect(result.components.every(c => isReady(c.state))).toBe(true);
  });

  it('daemon down — single failure', () => {
    const result: BootstrapResult = {
      components: [
        ready('homebrew', '4.4.2'),
        ready('sensei', '0.9.4'),
        ready('postgresql@17', '17.2'),
        ready('ollama', '0.6.2'),
        ready('database', 'schema-42'),
        failed('daemon', 'not reachable on port 7744'),
      ],
      hardware: defaultHardware,
      ready: false,
    };
    expect(result.ready).toBe(false);
    const daemonComp = result.components.find(c => c.name === 'daemon')!;
    expect(isFailed(daemonComp.state)).toBe(true);
    expect(errorMessage(daemonComp.state)).toBe('not reachable on port 7744');
  });

  it('fresh install — nothing installed', () => {
    const result: BootstrapResult = {
      components: [
        failed('homebrew', 'not installed'),
        failed('sensei', 'homebrew not installed'),
        failed('postgresql@17', 'homebrew not installed'),
        failed('ollama', 'homebrew not installed'),
        failed('database', 'postgresql not reachable (pg_isready failed)'),
        failed('daemon', 'not reachable on port 7744'),
      ],
      hardware: { ram_gb: 16, cpu_cores: 8, gpu: null, metal_support: false, recommended_tier: 'recommended' },
      ready: false,
    };
    expect(result.ready).toBe(false);
    expect(result.components.every(c => isFailed(c.state))).toBe(true);
  });

  it('ollama skipped — still ready', () => {
    const result: BootstrapResult = {
      components: [
        ready('homebrew', '4.4.2'),
        ready('sensei', '0.9.4'),
        ready('postgresql@17', '17.2'),
        skipped('ollama'),
        ready('database', 'schema-42'),
        ready('daemon', '0.9.4'),
      ],
      hardware: defaultHardware,
      ready: true,
    };
    expect(result.ready).toBe(true);
  });

  it('mid-install state — detecting and installing', () => {
    const result: BootstrapResult = {
      components: [
        ready('homebrew', '4.4.2'),
        installing('sensei'),
        detecting('postgresql@17'),
        detecting('ollama'),
        detecting('database'),
        detecting('daemon'),
      ],
      hardware: defaultHardware,
      ready: false,
    };
    expect(result.ready).toBe(false);
    const senseiComp = result.components.find(c => c.name === 'sensei')!;
    expect(stateLabel(senseiComp.state)).toBe('installing');
  });

  it('model pull in progress', () => {
    const comp = pulling('gemma3:27b', 45, 16384);
    expect(stateLabel(comp.state)).toBe('pulling');
    expect(comp.state.state).toBe('pulling');
    if (comp.state.state === 'pulling') {
      expect(comp.state.progress_pct).toBe(45);
      expect(comp.state.size_mb).toBe(16384);
    }
  });
});

// ── Hardware tier ────────────────────────────────────────────────────────────

describe('hardware tiers', () => {
  it('minimum tier for 8GB', () => {
    const hw = { ram_gb: 8, cpu_cores: 4, gpu: null, metal_support: false, recommended_tier: 'minimum' as const };
    expect(hw.recommended_tier).toBe('minimum');
  });

  it('recommended tier for 16GB', () => {
    expect(defaultHardware.recommended_tier).toBe('recommended');
  });

  it('full tier for 32GB+', () => {
    const hw = { ram_gb: 64, cpu_cores: 16, gpu: 'Apple M3 Max', metal_support: true, recommended_tier: 'full' as const };
    expect(hw.recommended_tier).toBe('full');
  });
});
