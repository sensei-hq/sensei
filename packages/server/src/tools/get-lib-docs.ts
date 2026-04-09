// packages/server/src/tools/get-lib-docs.ts
import { getActivityLog } from "../activity-log.js";

export interface LibSection {
  title: string;        // section title (H2 heading)
  content: string;      // section markdown
  document: {
    title: string;
    url: string | null;
    component: string | null;
    summary: string;
  };
  similarity?: number;
}

export async function getLibDocsTool(
  repoId: string,
  lib: string,
  opts?: { component?: string; query?: string; limit?: number },
): Promise<{ lib: string; sections: LibSection[] }> {
  try {
    const rows = getActivityLog(repoId).getLibDocs(lib, opts);

    const sections: LibSection[] = rows.map((r) => ({
      title: r.title,
      content: r.content ?? "",
      document: {
        title: r.title,
        url: r.url ?? r.localPath ?? null,
        component: r.component ?? null,
        summary: r.summary,
      },
      similarity: undefined,
    }));

    return { lib, sections };
  } catch (err) {
    console.warn(`getLibDocsTool: error fetching docs for lib "${lib}":`, err instanceof Error ? err.message : String(err));
    return { lib, sections: [] };
  }
}
