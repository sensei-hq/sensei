// Stub — will be replaced by a real implementation in a later task.
// This module is fully mocked in tests via vi.mock("./embedder.js").

export async function embed(_text: string): Promise<number[]> {
  throw new Error("embedder not yet implemented");
}

export async function isAvailable(): Promise<boolean> {
  return false;
}

export async function ensureReady(): Promise<void> {
  throw new Error("embedder not yet implemented");
}
