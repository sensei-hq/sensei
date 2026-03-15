import Anthropic from "@anthropic-ai/sdk";
import type { ModelBackend, FileAnalysis, ExtractionInstructions } from "@sensei/shared";

export interface ClaudeBackendOptions {
  model?: string; // default: 'claude-sonnet-4-6'
}

export class ClaudeBackend implements ModelBackend {
  readonly name = "claude";
  private client: Anthropic | null = null;
  private readonly model: string;

  constructor(opts: ClaudeBackendOptions = {}) {
    this.model = opts.model ?? "claude-sonnet-4-6";
  }

  async init(): Promise<void> {
    const key = process.env.ANTHROPIC_API_KEY;
    if (!key) throw new Error("ANTHROPIC_API_KEY is not set — export it before running sensei setup --agent claude");
    this.client = new Anthropic({ apiKey: key });
  }

  async isAvailable(): Promise<boolean> {
    return Boolean(process.env.ANTHROPIC_API_KEY);
  }

  async generate(prompt: string): Promise<string> {
    if (!this.client) throw new Error("ClaudeBackend not initialized — call init() first");
    const message = await this.client.messages.create({
      model: this.model,
      max_tokens: 4096,
      messages: [{ role: "user", content: prompt }],
    });
    const block = message.content[0];
    if (!block || block.type !== "text") return "";
    return block.text;
  }

  async embed(_text: string): Promise<number[]> {
    throw new Error("ClaudeBackend does not support embed");
  }

  async extract(_content: string, _instructions: ExtractionInstructions): Promise<FileAnalysis> {
    throw new Error("ClaudeBackend does not support extract");
  }
}
