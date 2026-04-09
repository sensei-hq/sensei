// packages/server/src/lib-indexer.ts
import { getActivityLog } from "./activity-log.js";
import type { LibEntry, DocPage } from "@sensei/shared";

export class LibIndexer {
  constructor(private readonly repoId: string) {}

  async index(
    entry: LibEntry,
    pages: DocPage[],
  ): Promise<{ sectionsIndexed: number }> {
    const docs = pages.map((page) => ({
      title: page.title,
      url: page.url,
      localPath: page.localPath,
      summary: page.summary,
      content: page.content,
      sourceType: page.sourceType ?? entry.source_type,
      component: page.component,
    }));

    getActivityLog(this.repoId).replaceLibDocs(entry.name, docs);

    return { sectionsIndexed: docs.length };
  }
}
