// packages/engine/src/lib/source-adapter.ts
import type { LibEntry, DocPage } from "@sensei/shared";

export interface SourceAdapter {
  fetch(entry: LibEntry): Promise<DocPage[]>;
}
