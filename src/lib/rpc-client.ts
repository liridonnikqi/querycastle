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
  request: {
    testConnection: (params: ConnectionInput) =>
      invokeCmd<{ ok: boolean; message: string; serverVersion: string | null }>("test_connection", {
        params,
      }),
    connect: (params: ConnectionInput) => invokeCmd<ConnectionStatus>("connect", { params }),
    disconnect: () => invokeCmd<{ ok: boolean }>("disconnect"),
    switchSession: (sessionId: string) =>
      invokeCmd<ConnectionStatus>("switch_session", { params: { sessionId } }),
    disconnectSession: (sessionId: string) =>
      invokeCmd<ConnectionStatus>("disconnect_session", { params: { sessionId } }),
    connectionStatus: () => invokeCmd<ConnectionStatus>("connection_status"),
    runQuery: (params: { sql: string }) => invokeCmd<QueryResultPayload>("run_query", { params }),
    getDatabaseExplorer: () => invokeCmd<DatabaseExplorer>("get_database_explorer"),
    listDatabases: () => invokeCmd<string[]>("list_databases"),
    selectDatabase: (params: { database: string }) =>
      invokeCmd<ConnectionStatus>("select_database", { params }),
    applyTableChanges: (params: {
      schema: string;
      table: string;
      changes: TableChangesPayload;
    }) =>
      invokeCmd<ApplyTableChangesResult>("apply_table_changes", {
        params,
      }),
    getLaunchSqlFile: () => invokeCmd<LaunchSqlFilePayload | null>("get_launch_sql_file"),
    getLaunchSqliteFile: () => invokeCmd<LaunchSqliteFilePayload | null>("get_launch_sqlite_file"),
    getObjectDefinition: (params: ObjectDefinitionParams) =>
      invokeCmd<ObjectDefinition>("get_object_definition", { params }),
  },
};
