import { existsSync } from "fs";
import { homedir } from "os";
import { join } from "path";

const MODEL = "Xenova/all-MiniLM-L6-v2";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let pipelineInstance: any = null;

async function getPipeline() {
  if (pipelineInstance) return pipelineInstance;
  const { pipeline } = await import("@xenova/transformers");
  pipelineInstance = await pipeline("feature-extraction", MODEL, { quantized: true });
  return pipelineInstance;
}

export async function embed(text: string): Promise<number[]> {
  const pipe = await getPipeline();
  const output = await pipe(text, { pooling: "mean", normalize: true });
  return Array.from(output.data as Float32Array);
}

export async function isAvailable(): Promise<boolean> {
  try {
    const { env } = await import("@xenova/transformers");
    const cacheDir = env.cacheDir ?? join(homedir(), ".cache", "xenova");
    // MODEL = "Xenova/all-MiniLM-L6-v2" — join splits the path correctly on all platforms
    return existsSync(join(cacheDir, MODEL));
  } catch {
    return false;
  }
}

export async function ensureReady(): Promise<void> {
  await getPipeline();
}
