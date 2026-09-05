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
  sslInsecure?: boolean;
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
  sessionId?: string;
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
  isPrimary: boolean;
  hasDefault?: boolean;
};

export type DatabaseForeignKey = {
  column: string;
  referencedSchema: string;
  referencedTable: string;
  referencedColumn: string;
};

export type DatabaseIndex = {
  name: string;
  columns: string;
  unique: boolean;
  isPrimary: boolean;
  definition: string | null;
};

export type DatabaseTrigger = {
  name: string;
  definition: string | null;
};

export type DatabaseRoutineKind = "function" | "procedure";

export type DatabaseRoutine = {
  schema: string;
  name: string;
  kind: DatabaseRoutineKind | string;
  identityArgs: string;
  language: string | null;
  returnType: string | null;
  objectId: string;
};

export type DatabaseSequence = {
  schema: string;
  name: string;
  dataType: string | null;
};

export type DatabaseTable = {
  schema: string;
  name: string;
  kind: "table" | "view";
  columns: DatabaseColumn[];
  foreignKeys: DatabaseForeignKey[];
  indexes?: DatabaseIndex[];
  triggers?: DatabaseTrigger[];
};

export type DatabaseSchema = {
  name: string;
  tables: DatabaseTable[];
  routines?: DatabaseRoutine[];
  sequences?: DatabaseSequence[];
};

export type ObjectDefinitionParams = {
  kind: string;
  schema: string;
  name: string;
  objectId?: string | null;
  identityArgs?: string | null;
  table?: string | null;
};

export type ObjectDefinition = {
  title: string;
  sql: string;
};

export type DatabaseExplorer = {
  database: string;
  schemas: DatabaseSchema[];
};

export type LaunchSqlFilePayload = {
  path: string;
  content: string;
};

export type LaunchSqliteFilePayload = {
  path: string;
};
