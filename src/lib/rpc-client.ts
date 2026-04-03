import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyTableChangesResult,
  ConnectionInput,
  ConnectionStatus,
  DatabaseExplorer,
  LaunchSqlFilePayload,
  QueryResultPayload,
  TableChangesPayload,
} from "./rpc";

export const rpc = {
  request: {
    testConnection: (params: ConnectionInput) =>
      invoke<{ ok: boolean; message: string; serverVersion: string | null }>("test_connection", {
        params,
      }),
    connect: (params: ConnectionInput) => invoke<ConnectionStatus>("connect", { params }),
    disconnect: () => invoke<{ ok: boolean }>("disconnect"),
    connectionStatus: () => invoke<ConnectionStatus>("connection_status"),
    runQuery: (params: { sql: string }) => invoke<QueryResultPayload>("run_query", { params }),
    getDatabaseExplorer: () => invoke<DatabaseExplorer>("get_database_explorer"),
    listDatabases: () => invoke<string[]>("list_databases"),
    selectDatabase: (params: { database: string }) => invoke<ConnectionStatus>("select_database", { params }),
    applyTableChanges: (params: {
      schema: string;
      table: string;
      changes: TableChangesPayload;
    }) => invoke<ApplyTableChangesResult>("apply_table_changes", {
      params,
    }),
    getLaunchSqlFile: () => invoke<LaunchSqlFilePayload | null>("get_launch_sql_file"),
  },
};
