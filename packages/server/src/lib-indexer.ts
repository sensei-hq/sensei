import { replaceLibDocs, writeLibMeta, addLibUser } from "./lib-store.js";
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

    // Write to shared ~/.sensei/libraries/{name}/docs.db
    replaceLibDocs(entry.name, docs);

    // Update meta
    await writeLibMeta(entry.name, {
      name: entry.name,
      sourceType: entry.source_type,
      baseUrl: entry.base_url,
      usedBy: [this.repoId],
      indexedAt: new Date().toISOString(),
    });

    // Register this repo as a user
    await addLibUser(entry.name, this.repoId);

    return { sectionsIndexed: docs.length };
  }
}
