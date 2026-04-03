import { describe, it, expect } from "vitest";
import { generateDek, wrapDek, unwrapDek, encryptSecret, decryptSecret } from "./crypto.js";
import { randomBytes } from "crypto";

describe("crypto — KEK/DEK hierarchy", () => {
  const kek = randomBytes(32);

  it("generateDek() returns 32 bytes", () => {
    const dek = generateDek();
    expect(dek.length).toBe(32);
  });

  it("wrapDek / unwrapDek round-trips correctly", () => {
    const dek = generateDek();
    const wrapped = wrapDek(dek, kek);
    // [12 IV][16 tag][32 ciphertext] = 60 bytes
    expect(wrapped.length).toBe(60);
    const unwrapped = unwrapDek(wrapped, kek);
    expect(unwrapped.equals(dek)).toBe(true);
  });

  it("wrapDek produces different ciphertext each call (random IV)", () => {
    const dek = generateDek();
    const w1 = wrapDek(dek, kek);
    const w2 = wrapDek(dek, kek);
    expect(w1.equals(w2)).toBe(false);
  });

  it("unwrapDek throws on tampered ciphertext", () => {
    const dek = generateDek();
    const wrapped = wrapDek(dek, kek);
    wrapped[30] ^= 0xff; // flip a byte in the ciphertext area
    expect(() => unwrapDek(wrapped, kek)).toThrow();
  });

  it("encryptSecret / decryptSecret round-trips a string", () => {
    const dek = generateDek();
    const secret = "sk-ant-api03-supersecret";
    const encrypted = encryptSecret(secret, dek);
    const recovered = decryptSecret(encrypted, dek);
    expect(recovered).toBe(secret);
  });

  it("encryptSecret produces different ciphertext each call", () => {
    const dek = generateDek();
    const e1 = encryptSecret("same-value", dek);
    const e2 = encryptSecret("same-value", dek);
    expect(e1.equals(e2)).toBe(false);
  });

  it("decryptSecret throws on tampered data", () => {
    const dek = generateDek();
    const enc = encryptSecret("hello", dek);
    enc[30] ^= 0xff;
    expect(() => decryptSecret(enc, dek)).toThrow();
  });
});
