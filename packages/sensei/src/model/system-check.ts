import { execSync } from "child_process";
import { existsSync } from "fs";
import { platform, homedir } from "os";
import { join } from "path";
import type { SetupStatus } from "./types.js";

export const OLLAMA_MODEL = "llama3.2:3b";
export const OLLAMA_MODEL_SIZE_GB = 2.0;
export const ONNX_MODEL_ID = "Xenova/all-MiniLM-L6-v2";
export const ONNX_MODEL_SIZE_MB = 22;
export const OLLAMA_BASE_URL = "http://127.0.0.1:11434";

interface CheckDeps {
  execFn?: (cmd: string) => string;
  fetchFn?: (url: string) => Promise<Response>;
}

export async function checkSystemRequirements(deps: CheckDeps = {}): Promise<SetupStatus> {
  const usingRealExec = deps.execFn === undefined;
  const exec = deps.execFn ?? ((cmd) => execSync(cmd, { encoding: "utf-8" }) as string);
  const fetchFn = deps.fetchFn ?? ((url) => fetch(url, { signal: AbortSignal.timeout(2000) }));

  // 1. Ollama binary
  let ollamaBinary = false;
  try {
    const out = exec("which ollama").trim();
    ollamaBinary = out.length > 0;
  } catch {
    // Only fall back to filesystem check when using the real exec (not injected test doubles)
    if (usingRealExec) {
      ollamaBinary = existsSync("/usr/local/bin/ollama") || existsSync("/opt/homebrew/bin/ollama");
    }
  }

  // 2. Ollama service + model list
  let ollamaRunning = false;
  let ollamaModel = false;
  if (ollamaBinary) {
    try {
      const res = await fetchFn(`${OLLAMA_BASE_URL}/api/tags`);
      if (res.ok) {
        ollamaRunning = true;
        const data = await res.json() as { models?: { name: string }[] };
        ollamaModel = (data.models ?? []).some(
          m => m.name.startsWith(OLLAMA_MODEL.split(":")[0])
        );
      }
    } catch { /* service down */ }
  }

  // 3. ONNX model cache
  const onnxCacheDir = join(
    homedir(), ".cache", "huggingface", "hub", "models--Xenova--all-MiniLM-L6-v2"
  );
  const onnxModel = existsSync(onnxCacheDir);

  // 4. Disk + RAM
  const diskFreeGB = getDiskFreeGB(exec);
  const { total: ramTotalGB, available: ramAvailableGB } = getRamGB(exec);

  return {
    ollamaBinary,
    ollamaRunning,
    ollamaModel,
    ollamaModelName: OLLAMA_MODEL,
    onnxModel,
    diskFreeGB,
    ramTotalGB,
    ramAvailableGB,
  };
}

export function getDiskFreeGB(
  exec: (cmd: string) => string = (c) => execSync(c, { encoding: "utf-8" }) as string
): number {
  try {
    if (platform() === "win32") {
      const out = exec("wmic logicaldisk get freespace /value");
      const match = out.match(/FreeSpace=(\d+)/);
      return match ? parseInt(match[1]) / 1e9 : 0;
    }
    if (platform() === "linux") {
      // --output=avail gives a single-column output: header "Avail" + value in KB
      const out = exec(`df -k --output=avail "${homedir()}"`);
      const lines = out.trim().split("\n");
      const freeKB = parseInt(lines[lines.length - 1].trim());
      return Number.isFinite(freeKB) ? freeKB / 1_000_000 : 0;
    }
    // macOS: df -k column 3 is Available (reliable on macOS)
    const out = exec(`df -k "${homedir()}"`);
    const lines = out.trim().split("\n");
    const last = lines[lines.length - 1].split(/\s+/);
    const freeKB = parseInt(last[3] ?? "0");
    return Number.isFinite(freeKB) ? freeKB / 1_000_000 : 0;
  } catch {
    return 0;
  }
}

export function getRamGB(
  exec: (cmd: string) => string = (c) => execSync(c, { encoding: "utf-8" }) as string
): { total: number; available: number } {
  try {
    if (platform() === "darwin") {
      const total = parseInt(exec("sysctl -n hw.memsize").trim()) / 1e9;
      const pagesStr = exec("sysctl -n vm.page_free_count").trim();
      const available = (parseInt(pagesStr) * 16384) / 1e9;
      return {
        total: Number.isFinite(total) ? Math.round(total * 10) / 10 : 0,
        available: Number.isFinite(available) ? Math.round(available * 10) / 10 : 0,
      };
    }
    if (platform() === "linux") {
      const meminfo = exec("cat /proc/meminfo");
      const total = parseInt(meminfo.match(/MemTotal:\s+(\d+)/)?.[1] ?? "0") / 1_000_000;
      const available = parseInt(meminfo.match(/MemAvailable:\s+(\d+)/)?.[1] ?? "0") / 1_000_000;
      return {
        total: Number.isFinite(total) ? Math.round(total * 10) / 10 : 0,
        available: Number.isFinite(available) ? Math.round(available * 10) / 10 : 0,
      };
    }
  } catch { /* ignore */ }
  return { total: 0, available: 0 };
}
