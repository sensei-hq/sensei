import { readFileSync, writeFileSync, existsSync, mkdirSync } from "fs";
import { dirname } from "path";
import { randomUUID } from "crypto";

export async function readOrCreateUuid(uuidPath: string): Promise<string> {
  if (existsSync(uuidPath)) {
    return readFileSync(uuidPath, "utf8").trim();
  }
  mkdirSync(dirname(uuidPath), { recursive: true });
  const id = randomUUID();
  writeFileSync(uuidPath, id, "utf8");
  return id;
}
