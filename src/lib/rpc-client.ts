import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyTableChangesResult,
  ConnectionInput,
  ConnectionStatus,
  DatabaseExplorer,
  LaunchSqlFilePayload,
  LaunchSqliteFilePayload,
  ObjectDefinition,
  ObjectDefinitionParams,
  QueryResultPayload,
  TableChangesPayload,
} from "./rpc";

function invokeError(error: unknown): Error {
  if (error instanceof Error) return error;
  if (typeof error === "string") return new Error(error);
  if (error && typeof error === "object" && "message" in error) {
    return new Error(String((error as { message: unknown }).message));
  }
  return new Error(String(error));
}

async function invokeCmd<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (error) {
    throw invokeError(error);
  }
}

export const rpc = {
  testConnection: (params: ConnectionInput) =>
    invokeCmd<{ ok: boolean; message: string; serverVersion: string | null }>("test_connection", {
      params,
    }),
  connect: (params: ConnectionInput) => invokeCmd<ConnectionStatus>("connect", { params }),
  disconnect: () => invokeCmd<void>("disconnect"),
  switchSession: (sessionId: string) =>
    invokeCmd<ConnectionStatus>("switch_session", { params: { sessionId } }),
  disconnectSession: (sessionId: string) =>
    invokeCmd<ConnectionStatus>("disconnect_session", { params: { sessionId } }),
  secretSet: (connectionName: string, password: string) =>
    invokeCmd<void>("secret_set", { params: { connectionName, password } }),
  secretGet: (connectionName: string) =>
    invokeCmd<string | null>("secret_get", { params: { connectionName } }),
  secretDelete: (connectionName: string) =>
    invokeCmd<void>("secret_delete", { params: { connectionName } }),
  connectionStatus: () => invokeCmd<ConnectionStatus>("connection_status"),
  runQuery: (params: { sql: string; sessionId: string }) =>
    invokeCmd<QueryResultPayload>("run_query", { params }),
  getDatabaseExplorer: (sessionId: string) =>
    invokeCmd<DatabaseExplorer>("get_database_explorer", { params: { sessionId } }),
  listDatabases: (sessionId: string) =>
    invokeCmd<string[]>("list_databases", { params: { sessionId } }),
  selectDatabase: (params: { sessionId: string; database: string }) =>
    invokeCmd<ConnectionStatus>("select_database", { params }),
  applyTableChanges: (params: {
    sessionId: string;
    schema: string;
    table: string;
    changes: TableChangesPayload;
  }) =>
    invokeCmd<ApplyTableChangesResult>("apply_table_changes", {
      params,
    }),
  getLaunchSqlFile: () => invokeCmd<LaunchSqlFilePayload | null>("get_launch_sql_file"),
  getLaunchSqliteFile: () => invokeCmd<LaunchSqliteFilePayload | null>("get_launch_sqlite_file"),
  getObjectDefinition: (params: ObjectDefinitionParams & { sessionId: string }) =>
    invokeCmd<ObjectDefinition>("get_object_definition", { params }),
};
