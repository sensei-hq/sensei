/**
 * WebdriverIO config for Tauri E2E tests.
 *
 * Uses tauri-driver as the WebDriver backend, which connects to
 * the actual Tauri app's WebView — real sidecar commands, not mocks.
 */

import { spawn, execFileSync, type ChildProcess } from 'child_process';
import path from 'path';

const APP_BINARY = path.resolve(__dirname, '../../src-tauri/target/debug/sensei-desktop');
const TAURI_DRIVER_PORT = 4444;

let tauriDriver: ChildProcess | null = null;

function findTauriDriver(): string {
  try {
    return execFileSync('which', ['tauri-driver']).toString().trim();
  } catch {
    return path.join(process.env.HOME ?? '', '.cargo/bin/tauri-driver');
  }
}

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./e2e/tauri/*.spec.ts'],
  exclude: [],

  maxInstances: 1,
  capabilities: [{
    // @ts-ignore — tauri custom capability
    'tauri:options': {
      application: APP_BINARY,
    },
  }],

  logLevel: 'warn',
  waitforTimeout: 10_000,
  connectionRetryTimeout: 30_000,
  connectionRetryCount: 3,

  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60_000,
  },

  // Start tauri-driver before tests
  onPrepare: function () {
    const driverPath = process.env.TAURI_DRIVER_PATH || findTauriDriver();

    tauriDriver = spawn(driverPath, ['--port', String(TAURI_DRIVER_PORT)], {
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    // Wait for driver to be ready
    return new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('tauri-driver did not start')), 10_000);

      tauriDriver!.stdout?.on('data', (data: Buffer) => {
        if (data.toString().includes('listening')) {
          clearTimeout(timeout);
          resolve();
        }
      });

      // Resolve after short delay if no "listening" message
      setTimeout(() => {
        clearTimeout(timeout);
        resolve();
      }, 2000);
    });
  },

  // Stop tauri-driver after tests
  onComplete: function () {
    if (tauriDriver) {
      tauriDriver.kill();
      tauriDriver = null;
    }
  },

  hostname: '127.0.0.1',
  port: TAURI_DRIVER_PORT,
};
