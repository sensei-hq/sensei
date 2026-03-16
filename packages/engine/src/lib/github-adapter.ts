// packages/engine/src/lib/github-adapter.ts
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { extractSummary } from "./doc-utils.js";

interface GitHubTreeItem { path: string; type: "blob" | "tree"; url: string; }
interface GitHubTreeResponse { tree: GitHubTreeItem[]; truncated: boolean; }

export interface ParsedGithubUrl { owner: string; repo: string; branch: string; basePath: string; }

export function parseGithubUrl(url: string): ParsedGithubUrl | null {
  const match = url.match(/^https:\/\/github\.com\/([^/]+)\/([^/]+)\/tree\/([^/]+)\/?(.*)$/);
  if (!match) return null;
  return { owner: match[1], repo: match[2], branch: match[3], basePath: match[4] ?? "" };
}

export class GithubAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`GithubAdapter: entry "${entry.name}" requires base_url`);

    const parsed = parseGithubUrl(entry.base_url);
    if (!parsed) throw new Error(`GithubAdapter: invalid GitHub tree URL: ${entry.base_url}`);

    const { owner, repo, branch, basePath } = parsed;
    const treeUrl = `https://api.github.com/repos/${owner}/${repo}/git/trees/${branch}?recursive=1`;
    const treeRes = await fetch(treeUrl, { headers: { Accept: "application/vnd.github.v3+json" } });
    if (!treeRes.ok) throw new Error(`GithubAdapter: GitHub API error ${treeRes.status} for ${treeUrl}`);

    const tree: GitHubTreeResponse = await treeRes.json();
    if (tree.truncated) {
      console.warn(
        `[GithubAdapter] Warning: tree truncated for ${owner}/${repo} — some files may be missing.\n` +
        `Fetched ${tree.tree.length} of potentially more files.`
      );
    }
    const prefix = basePath ? basePath + "/" : "";
    const mdFiles = tree.tree.filter(item => item.type === "blob" && item.path.startsWith(prefix) && item.path.endsWith(".md"));

    const pages: DocPage[] = [];
    for (const file of mdFiles) {
      const rawUrl = `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${file.path}`;
      const res = await fetch(rawUrl);
      if (!res.ok) continue;
      const content = await res.text();
      pages.push({
        title: extractH1(content) ?? stemName(file.path),
        url: `https://github.com/${owner}/${repo}/blob/${branch}/${file.path}`,
        summary: extractSummary(content),
        content,
        sourceType: "github",
        component: inferComponent(file.path, basePath),
      });
    }
    return pages;
  }
}

function extractH1(content: string): string | undefined {
  return content.match(/^#\s+(.+)$/m)?.[1]?.trim();
}


function stemName(filePath: string): string {
  return (filePath.split("/").pop() ?? filePath).replace(/\.md$/, "");
}

function inferComponent(filePath: string, basePath: string): string | undefined {
  const parts = filePath.split("/");
  const baseDepth = basePath ? basePath.split("/").length : 0;
  if (parts.length <= baseDepth + 1) return undefined;
  return parts[baseDepth];
}
