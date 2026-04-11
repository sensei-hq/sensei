import { type Database, type Connection, type QueryResult } from "kuzu";

async function runIgnoreExists(
  conn: Connection,
  statement: string
): Promise<void> {
  try {
    const result = await conn.query(statement);
    // QueryResult might be an array for multiple statements
    if (Array.isArray(result)) {
      for (const r of result as QueryResult[]) r.close();
    } else {
      (result as QueryResult).close();
    }
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    // Ignore "already exists" errors
    if (
      !msg.includes("already exists") &&
      !msg.includes("Binder exception") &&
      !msg.includes("CatalogException")
    ) {
      throw e;
    }
  }
}

export async function ensureSchema(db: Database): Promise<void> {
  const kuzu = await import("kuzu");
  const conn = new kuzu.Connection(db);
  try {
    await ensureSchemaWithConn(conn);
  } finally {
    await conn.close();
  }
}

export async function ensureSchemaWithConn(conn: Connection): Promise<void> {
  await _ensureSchemaWithConn(conn);
}

async function _ensureSchemaWithConn(conn: Connection): Promise<void> {
  // Node tables
  await runIgnoreExists(
    conn,
    `CREATE NODE TABLE IF NOT EXISTS Function(
      id STRING,
      name STRING,
      file STRING,
      line INT64,
      sig STRING,
      body STRING,
      docstring STRING,
      complexity INT64 DEFAULT 1,
      project STRING,
      PRIMARY KEY(id)
    )`
  );

  await runIgnoreExists(
    conn,
    `CREATE NODE TABLE IF NOT EXISTS File(
      id STRING,
      path STRING,
      module STRING,
      lang STRING,
      project STRING,
      PRIMARY KEY(id)
    )`
  );

  await runIgnoreExists(
    conn,
    `CREATE NODE TABLE IF NOT EXISTS Type(
      id STRING,
      name STRING,
      file STRING,
      line INT64,
      kind STRING,
      project STRING,
      PRIMARY KEY(id)
    )`
  );

  await runIgnoreExists(
    conn,
    `CREATE NODE TABLE IF NOT EXISTS Comment(
      id STRING,
      text STRING,
      tag STRING,
      line INT64,
      file STRING,
      project STRING,
      PRIMARY KEY(id)
    )`
  );

  // Relationship tables
  await runIgnoreExists(
    conn,
    `CREATE REL TABLE IF NOT EXISTS CALLS(FROM Function TO Function, weight DOUBLE)`
  );

  await runIgnoreExists(
    conn,
    `CREATE REL TABLE IF NOT EXISTS IMPORTS(FROM File TO File)`
  );

  await runIgnoreExists(
    conn,
    `CREATE REL TABLE IF NOT EXISTS EXPORTS_FN(FROM File TO Function)`
  );

  await runIgnoreExists(
    conn,
    `CREATE REL TABLE IF NOT EXISTS EXPORTS_TYPE(FROM File TO Type)`
  );

  await runIgnoreExists(
    conn,
    `CREATE REL TABLE IF NOT EXISTS ANNOTATES_FN(FROM Comment TO Function)`
  );

  await runIgnoreExists(
    conn,
    `CREATE REL TABLE IF NOT EXISTS ANNOTATES_TYPE(FROM Comment TO Type)`
  );

  // Migration: add complexity column for databases created before it was in CREATE TABLE
  try {
    const r = await conn.query(`ALTER TABLE Function ADD complexity INT64 DEFAULT 1`);
    if (Array.isArray(r)) { for (const x of r as QueryResult[]) x.close(); } else { (r as QueryResult).close(); }
  } catch { /* column already exists — ignore */ }

  // Doc node — represents a documentation file (.md / .mdx)
  await runIgnoreExists(
    conn,
    `CREATE NODE TABLE IF NOT EXISTS Doc(
      id      STRING,
      path    STRING,
      title   STRING,
      project STRING,
      PRIMARY KEY(id)
    )`
  );

  // Doc relationships
  await runIgnoreExists(conn, `CREATE REL TABLE IF NOT EXISTS COVERS(FROM Doc TO File)`);
  await runIgnoreExists(conn, `CREATE REL TABLE IF NOT EXISTS MENTIONS_FN(FROM Doc TO Function)`);
  await runIgnoreExists(conn, `CREATE REL TABLE IF NOT EXISTS SUPERSEDES(FROM Doc TO Doc)`);
}
