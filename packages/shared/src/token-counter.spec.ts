import { describe, it, expect } from "vitest";
import { createTokenCounter, EstimateTokenCounter, OpenAITokenCounter, AnthropicTokenCounter } from "./token-counter.js";

describe("EstimateTokenCounter", () => {
  it("estimates tokens as ceil(length / 4)", () => {
    const counter = new EstimateTokenCounter();
    expect(counter.count("hello")).toBe(Math.ceil("hello".length / 4));
    expect(counter.count("")).toBe(0);
  });

  it("name is estimate", () => {
    expect(new EstimateTokenCounter().name).toBe("estimate");
  });
});

describe("OpenAITokenCounter", () => {
  it("counts tokens for a known string", () => {
    const counter = new OpenAITokenCounter();
    const count = counter.count("hello world");
    expect(count).toBeGreaterThan(0);
    expect(count).toBeLessThan(10);
  });

  it("name is openai", () => {
    expect(new OpenAITokenCounter().name).toBe("openai");
  });
});

describe("AnthropicTokenCounter", () => {
  it("counts tokens for a known string", () => {
    const counter = new AnthropicTokenCounter();
    const count = counter.count("hello world");
    expect(count).toBeGreaterThan(0);
    expect(count).toBeLessThan(10);
  });

  it("name is anthropic", () => {
    expect(new AnthropicTokenCounter().name).toBe("anthropic");
  });
});

describe("createTokenCounter", () => {
  it("returns EstimateTokenCounter when no modelId", () => {
    expect(createTokenCounter().name).toBe("estimate");
  });

  it("returns AnthropicTokenCounter for claude- prefix", () => {
    expect(createTokenCounter("claude-sonnet-4-6").name).toBe("anthropic");
  });

  it("returns OpenAITokenCounter for gpt- prefix", () => {
    expect(createTokenCounter("gpt-4o").name).toBe("openai");
  });

  it("returns OpenAITokenCounter for o1- prefix", () => {
    expect(createTokenCounter("o1-mini").name).toBe("openai");
  });

  it("returns EstimateTokenCounter for unknown modelId", () => {
    expect(createTokenCounter("unknown-model-xyz").name).toBe("estimate");
  });
});
