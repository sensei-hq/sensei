import { randomUUID } from "crypto";
import type { ContextPack, Slice, TokenCounter } from "@sensei/shared";

export interface AssembleOptions {
  maxTokens?: number;
  counter: TokenCounter;
  task: string;
  modelId?: string;
  sessionContext?: string[];
}

export class Assembler {
  assemble(slices: Slice[], opts: AssembleOptions): ContextPack {
    const maxTokens = opts.maxTokens ?? 8000;
    const excluded = new Set(opts.sessionContext ?? []);
    const sorted = [...slices].sort((a, b) => b.score - a.score);

    const included: Slice[] = [];
    let totalTokens = 0;

    for (const slice of sorted) {
      if (excluded.has(slice.filePath)) continue;
      if (totalTokens + slice.tokens > maxTokens) continue;
      included.push(slice);
      totalTokens += slice.tokens;
    }

    return {
      id: randomUUID(),
      task: opts.task,
      slices: included,
      totalTokens,
      modelId: opts.modelId,
      createdAt: new Date().toISOString(),
    };
  }
}
