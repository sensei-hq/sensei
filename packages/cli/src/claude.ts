import Anthropic from "@anthropic-ai/sdk";

const MODEL = "claude-opus-4-6";

export interface ClaudeUsage {
  tokensIn: number;
  tokensOut: number;
}

export interface ClaudeResult {
  text: string;
  usage: ClaudeUsage;
}

export async function callClaude(prompt: string): Promise<ClaudeResult> {
  const client = new Anthropic();
  const stream = client.messages.stream({
    model: MODEL,
    max_tokens: 16384,
    thinking: { type: "adaptive" },
    messages: [{ role: "user", content: prompt }],
  });
  const message = await stream.finalMessage();
  const text = message.content.find(b => b.type === "text");
  return {
    text: text?.text ?? "",
    usage: {
      tokensIn: message.usage.input_tokens,
      tokensOut: message.usage.output_tokens,
    },
  };
}
