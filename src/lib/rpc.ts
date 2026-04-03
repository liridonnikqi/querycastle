export type DatabaseType = "postgres" | "mysql" | "sqlite";

export type ConnectionInput = {
  databaseType: DatabaseType;
  name: string;
  host: string;
  port: number;
  user: string;
  password: string;
  database: string;
  ssl: boolean;
  useConnectionString?: boolean;
  connectionString?: string;
};

export type ConnectionStatus = {
  connected: boolean;
  databaseType: DatabaseType;
  name: string;
  host: string;
  port: number;
  database: string;
  user: string;
  serverVersion: string | null;
};

export type QueryResultPayload = {
  columns: string[];
  rows: Array<Record<string, unknown>>;
  rowCount: number;
  durationMs: number;
};

export type TableChangesPayload = {
  updates: Array<{ ctid: string; values: Record<string, unknown> }>;
  deletes: string[];
  inserts: Array<Record<string, unknown>>;
};

export type ApplyTableChangesResult = {
  ok: boolean;
  updated: number;
  deleted: number;
  inserted: number;
  updatedRows: Array<{ oldCtid: string; newCtid: string; values: Record<string, unknown> }>;
};

export type DatabaseColumn = {
  name: string;
  dataType: string;
  notNull: boolean;
};

export type DatabaseForeignKey = {
  column: string;
  referencedSchema: string;
  referencedTable: string;
  referencedColumn: string;
};

export type DatabaseTable = {
  schema: string;
  name: string;
  kind: "table" | "view";
  columns: DatabaseColumn[];
  foreignKeys: DatabaseForeignKey[];
};

export type DatabaseSchema = {
  name: string;
  tables: DatabaseTable[];
};

export type DatabaseExplorer = {
  database: string;
  schemas: DatabaseSchema[];
};

export type LaunchSqlFilePayload = {
  path: string;
  content: string;
};
