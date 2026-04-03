// packages/shared/src/crypto.ts
// KEK/DEK encryption hierarchy for LLM API keys and sensitive credentials.
// Algorithm: AES-256-GCM
// Wrapped DEK format: [12-byte IV][16-byte auth tag][32-byte ciphertext]

import { randomBytes, createCipheriv, createDecipheriv } from "crypto";

const IV_BYTES   = 12;
const TAG_BYTES  = 16;
const DEK_BYTES  = 32;
const ALGO       = "aes-256-gcm" as const;

/** Generate a fresh 256-bit Data Encryption Key. */
export function generateDek(): Buffer {
  return randomBytes(DEK_BYTES);
}

/**
 * Wrap a DEK with the server-side KEK using AES-256-GCM.
 * Output: [12-byte IV][16-byte auth tag][32-byte ciphertext]
 */
export function wrapDek(dek: Buffer, kek: Buffer): Buffer {
  const iv = randomBytes(IV_BYTES);
  const cipher = createCipheriv(ALGO, kek, iv);
  const ciphertext = Buffer.concat([cipher.update(dek), cipher.final()]);
  const tag = cipher.getAuthTag();
  return Buffer.concat([iv, tag, ciphertext]);
}

/** Unwrap a DEK from its encrypted form using the KEK. */
export function unwrapDek(wrapped: Buffer, kek: Buffer): Buffer {
  const iv         = wrapped.subarray(0, IV_BYTES);
  const tag        = wrapped.subarray(IV_BYTES, IV_BYTES + TAG_BYTES);
  const ciphertext = wrapped.subarray(IV_BYTES + TAG_BYTES);
  const decipher   = createDecipheriv(ALGO, kek, iv);
  decipher.setAuthTag(tag);
  return Buffer.concat([decipher.update(ciphertext), decipher.final()]);
}

/**
 * Encrypt a plaintext string with the account's DEK.
 * Output: [12-byte IV][16-byte auth tag][variable ciphertext]
 */
export function encryptSecret(plaintext: string, dek: Buffer): Buffer {
  const iv      = randomBytes(IV_BYTES);
  const cipher  = createCipheriv(ALGO, dek, iv);
  const ct      = Buffer.concat([cipher.update(Buffer.from(plaintext, "utf-8")), cipher.final()]);
  const tag     = cipher.getAuthTag();
  return Buffer.concat([iv, tag, ct]);
}

/** Decrypt a secret encrypted with encryptSecret(). */
export function decryptSecret(encrypted: Buffer, dek: Buffer): string {
  const iv         = encrypted.subarray(0, IV_BYTES);
  const tag        = encrypted.subarray(IV_BYTES, IV_BYTES + TAG_BYTES);
  const ciphertext = encrypted.subarray(IV_BYTES + TAG_BYTES);
  const decipher   = createDecipheriv(ALGO, dek, iv);
  decipher.setAuthTag(tag);
  return Buffer.concat([decipher.update(ciphertext), decipher.final()]).toString("utf-8");
}

/** Load the KEK from env. Throws if not configured. */
export function loadKek(): Buffer {
  const raw = process.env.SENSEI_KEK;
  if (!raw) throw new Error("SENSEI_KEK env var is required for encryption");
  const kek = Buffer.from(raw, "hex");
  if (kek.length !== DEK_BYTES) throw new Error(`SENSEI_KEK must be ${DEK_BYTES * 2} hex chars (${DEK_BYTES} bytes)`);
  return kek;
}
