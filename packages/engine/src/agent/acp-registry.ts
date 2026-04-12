import type { AcpAdapter } from "./acp-adapter.js";
import { ClaudeAdapter } from "./claude-adapter.js";
import { CursorAdapter } from "./cursor-adapter.js";
import { WindsurfAdapter } from "./windsurf-adapter.js";
import { ZedAdapter } from "./zed-adapter.js";
import { KiroAdapter } from "./kiro-adapter.js";
import { OpenCodeAdapter } from "./opencode-adapter.js";

/**
 * Registry of all supported ACP adapters.
 *
 * Use `AcpRegistry.all` to get every adapter instance.
 * Use `AcpRegistry.detected()` to get only those detected on the current machine.
 * Use `AcpRegistry.get(id)` to retrieve a specific adapter by ID.
 */
export class AcpRegistry {
  static readonly all: AcpAdapter[] = [
    new ClaudeAdapter(),
    new CursorAdapter(),
    new WindsurfAdapter(),
    new ZedAdapter(),
    new KiroAdapter(),
    new OpenCodeAdapter(),
  ];

  /** Returns adapters whose `detect()` returns true (i.e. the ACP is installed). */
  static async detected(): Promise<AcpAdapter[]> {
    const results = await Promise.all(
      AcpRegistry.all.map(async adapter => ({ adapter, ok: await adapter.detect() })),
    );
    return results.filter(r => r.ok).map(r => r.adapter);
  }

  /** Returns the adapter with the given ID, or undefined if not found. */
  static get(id: string): AcpAdapter | undefined {
    return AcpRegistry.all.find(a => a.id === id);
  }
}
