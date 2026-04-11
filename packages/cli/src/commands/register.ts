import { intro, outro, log, spinner } from "@clack/prompts";
import { AcpRegistry } from "@sensei/engine";
import { installHooks } from "@sensei/collector";

const OTLP_ENDPOINT = "http://localhost:51789";

/**
 * `sensei register [acp-id]`
 *
 * Detects installed ACPs and registers the sensei MCP server, hooks, and settings
 * in each one. If an ACP ID is provided, only that adapter is used.
 *
 * This is the single source of truth for all ACP registration — both the CLI
 * (sensei init, sensei register) and the desktop setup wizard delegate here.
 */
export async function register(acpId?: string): Promise<void> {
  intro("sensei register");

  let adapters = acpId
    ? AcpRegistry.get(acpId) ? [AcpRegistry.get(acpId)!] : []
    : await AcpRegistry.detected();

  if (adapters.length === 0) {
    if (acpId) {
      log.error(`Unknown ACP: ${acpId}. Supported: ${AcpRegistry.all.map(a => a.id).join(", ")}`);
    } else {
      log.warn("No supported ACPs detected on this machine.");
    }
    process.exit(1);
  }

  for (const adapter of adapters) {
    const s = spinner();
    s.start(`Registering with ${adapter.name}`);

    const steps: string[] = [];
    const errors: string[] = [];

    if (adapter.registerMcp) {
      try {
        await adapter.registerMcp();
        steps.push("MCP server");
      } catch (err) {
        errors.push(`MCP: ${err instanceof Error ? err.message : String(err)}`);
      }
    }

    // Claude Code hooks are managed by the collector package
    if (adapter.id === "claude-code") {
      try {
        await installHooks();
        steps.push("hooks");
      } catch (err) {
        errors.push(`hooks: ${err instanceof Error ? err.message : String(err)}`);
      }
    }

    if (adapter.injectSettings) {
      try {
        await adapter.injectSettings({ otlpEndpoint: OTLP_ENDPOINT });
        steps.push("settings");
      } catch (err) {
        errors.push(`settings: ${err instanceof Error ? err.message : String(err)}`);
      }
    }

    s.stop(`${adapter.name}: ${steps.join(", ") || "nothing to configure"}`);

    for (const e of errors) {
      log.warn(`  ${e}`);
    }
  }

  log.success("Restart your ACP to activate the sensei MCP server.");
  outro("Done.");
}
