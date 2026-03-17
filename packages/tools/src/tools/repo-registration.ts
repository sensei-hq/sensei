// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnySupabaseClient = any;

export interface RepoInfo {
  name: string;
  remote_url: string | null;
  default_branch?: string;
  description?: string;
  stack?: string[];
}

/** Upsert repo into sensei.repos. Returns the repo UUID, or null on error.
 *  Client must be created with db: { schema: "sensei" } (via makeSenseiClient). */
export async function registerRepo(
  client: AnySupabaseClient,
  info: RepoInfo,
): Promise<string | null> {
  const { data, error } = await client
    .from("repos")
    .upsert({
      name:           info.name,
      remote_url:     info.remote_url,
      default_branch: info.default_branch ?? null,
      description:    info.description ?? null,
      stack:          info.stack ?? null,
    }, { onConflict: "remote_url", ignoreDuplicates: false })
    .select("id");

  if (error || !data?.[0]) {
    if (error) console.error("[indexer] Supabase repo upsert error:", error.message);
    return null;
  }
  return data[0].id as string;
}
