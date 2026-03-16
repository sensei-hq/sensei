/**
 * Embedding-only ModelBackend using @xenova/transformers.
 * Model: Xenova/all-MiniLM-L6-v2 (384-dim, ~23MB ONNX, downloads on first use).
 * Only embed() is implemented — other ModelBackend methods throw.
 */
import type { ModelBackend, FileAnalysis, ExtractionInstructions } from "@sensei/shared";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let pipelinePromise: Promise<any> | null = null;

async function getEmbeddingPipeline(): Promise<any> {
  if (!pipelinePromise) {
    // Lazy import — cache the Promise so concurrent callers share one initialization
    pipelinePromise = import("@xenova/transformers").then(({ pipeline }) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (pipeline as any)("feature-extraction", "Xenova/all-MiniLM-L6-v2")
    );
  }
  return pipelinePromise;
}

export class TransformersBackend implements ModelBackend {
  readonly name = "transformers";

  async init(): Promise<void> {
    await getEmbeddingPipeline();
  }

  async isAvailable(): Promise<boolean> {
    return true;
  }

  async embed(text: string): Promise<number[]> {
    const extractor = await getEmbeddingPipeline();
    const result = await extractor(text, { pooling: "mean", normalize: true });
    return Array.from(result.data);
  }

  async generate(_prompt: string): Promise<string> {
    throw new Error("TransformersBackend: generate() not supported");
  }

  async extract(_content: string, _instructions: ExtractionInstructions): Promise<FileAnalysis> {
    throw new Error("TransformersBackend: extract() not supported");
  }
}
