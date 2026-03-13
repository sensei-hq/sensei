import { get_encoding, type Tiktoken } from "tiktoken";

export interface TokenCounter {
  readonly name: string;
  count(text: string): number;
}

export class EstimateTokenCounter implements TokenCounter {
  readonly name = "estimate";
  count(text: string): number {
    return Math.ceil(text.length / 4);
  }
}

export class OpenAITokenCounter implements TokenCounter {
  readonly name = "openai";
  private enc: Tiktoken | null = null;

  count(text: string): number {
    if (!this.enc) this.enc = get_encoding("cl100k_base");
    return this.enc.encode(text).length;
  }
}

export class AnthropicTokenCounter implements TokenCounter {
  readonly name = "anthropic";
  private enc: Tiktoken | null = null;

  count(text: string): number {
    // Claude uses a BPE tokenizer compatible with cl100k_base (~95% accuracy)
    if (!this.enc) this.enc = get_encoding("cl100k_base");
    return this.enc.encode(text).length;
  }
}

export function createTokenCounter(modelId?: string): TokenCounter {
  if (!modelId) return new EstimateTokenCounter();
  if (modelId.startsWith("claude-")) return new AnthropicTokenCounter();
  if (modelId.startsWith("gpt-") || modelId.startsWith("o1-") || modelId.startsWith("o3-")) {
    return new OpenAITokenCounter();
  }
  return new EstimateTokenCounter();
}
