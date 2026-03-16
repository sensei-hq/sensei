// packages/engine/src/lib/github-adapter.ts
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { extractSummary, parseLlmsIndex } from "./doc-utils.js";

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
    const token = process.env.GITHUB_TOKEN;
    const authHeaders: Record<string, string> = {
      Accept: "application/vnd.github.v3+json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    };
    const treeUrl = `https://api.github.com/repos/${owner}/${repo}/git/trees/${branch}?recursive=1`;
    const treeRes = await fetch(treeUrl, { headers: authHeaders });
    if (!treeRes.ok) throw new Error(`GithubAdapter: GitHub API error ${treeRes.status} for ${treeUrl}`);

    const tree: GitHubTreeResponse = await treeRes.json();
    if (tree.truncated) {
      console.warn(
        `[GithubAdapter] Warning: tree truncated for ${owner}/${repo} — some files may be missing.\n` +
        `Fetched ${tree.tree.length} of potentially more files.`
      );
    }
    const prefix = basePath ? basePath + "/" : "";

    // Prefer llms.txt index if present — gives curated titles, summaries, and component grouping
    const llmsTxtItem = tree.tree.find(
      item => item.type === "blob" && item.path === (prefix + "llms.txt")
    );
    if (llmsTxtItem) {
      const llmsRawUrl = `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${llmsTxtItem.path}`;
      const llmsRes = await fetch(llmsRawUrl, { headers: authHeaders });
      if (llmsRes.ok) {
        const llmsText = await llmsRes.text();
        // Use the GitHub blob URL as base so relative links resolve correctly
        const llmsBlobUrl = `https://github.com/${owner}/${repo}/blob/${branch}/${llmsTxtItem.path}`;
        const entries = parseLlmsIndex(llmsText, llmsBlobUrl);
        const pages: DocPage[] = [];
        for (let i = 0; i < entries.length; i++) {
          const e = entries[i];
          // Convert blob URL to raw for fetching
          const rawUrl = e.url.replace(
            /^https:\/\/github\.com\/([^/]+)\/([^/]+)\/blob\/([^/]+)\//,
            `https://raw.githubusercontent.com/$1/$2/$3/`
          );
          const res = await fetch(rawUrl, { headers: authHeaders });
          if (!res.ok) {
            console.warn(`[GithubAdapter] Failed to fetch ${rawUrl}: HTTP ${res.status}`);
            pages.push({ title: e.title, url: e.url, summary: e.summary, content: e.summary, sourceType: "github", component: e.component, sequence: i });
            continue;
          }
          const content = await res.text();
          pages.push({ title: e.title, url: e.url, summary: e.summary, content, sourceType: "github", component: e.component, sequence: i });
        }
        return pages;
      }
    }

    // Fallback: crawl all .md and .txt files under the base path
    const docFiles = tree.tree.filter(
      item => item.type === "blob" && item.path.startsWith(prefix) &&
        (item.path.endsWith(".md") || item.path.endsWith(".txt")) &&
        !item.path.endsWith("llms.txt")
    );
    const pages: DocPage[] = [];
    for (const file of docFiles) {
      const rawUrl = `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${file.path}`;
      const res = await fetch(rawUrl, { headers: authHeaders });
      if (!res.ok) {
        console.warn(`[GithubAdapter] Failed to fetch ${rawUrl}: HTTP ${res.status}`);
        continue;
      }
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
