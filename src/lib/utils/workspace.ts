import type { QueryResultPayload } from '$lib/rpc';

export const SAVED_CONNECTIONS_KEY = 'querycastle.savedConnections.v1';
export const QUERY_TABS_KEY = 'querycastle.queryTabs.v2';
export const QUERY_FAVORITES_KEY = 'querycastle.queryFavorites.v1';
export const QUERY_HISTORY_KEY = 'querycastle.queryHistory.v1';
export const MAIN_VIEW_KEY = 'querycastle.mainView.v1';

export type TabKind = 'query' | 'data';
export type MainView = 'sql' | 'saved_queries' | 'history';
export type WorkspaceTab = {
	id: string;
	title: string;
	kind: TabKind;
	sql: string;
	lastRunSql: string;
	result: QueryResultPayload;
	sqlError: string;
	resultContext: { schema: string; table: string } | null;
};
export type TabContextMenu = { x: number; y: number; tabId: string } | null;

export type TableAction =
	| 'view_data'
	| 'view_structure'
	| 'export_file'
	| 'import_file'
	| 'copy_name'
	| 'hide'
	| 'sql_create'
	| 'rename'
	| 'drop'
	| 'truncate'
	| 'duplicate';

export type SchemaAction = 'copy_name' | 'copy_quoted_name' | 'sql_list_tables';

export function createEmptyResult(): QueryResultPayload {
	return { columns: [], rows: [], rowCount: 0, durationMs: 0 };
}

export function createDefaultTab(): WorkspaceTab {
	return {
		id: crypto.randomUUID(),
		title: 'Query 1',
		kind: 'query',
		sql: 'SELECT 1;',
		lastRunSql: '',
		result: createEmptyResult(),
		sqlError: '',
		resultContext: null,
	};
}

export function deriveFavoriteTitle(sql: string): string {
	const firstLine = sql
		.split('\n')
		.map((line) => line.trim())
		.find((line) => line.length > 0);
	if (!firstLine) return 'Saved Query';
	return firstLine.length > 56 ? `${firstLine.slice(0, 56)}...` : firstLine;
}

export function nowLabel(): string {
	return new Date().toLocaleTimeString([], {
		hour: '2-digit',
		minute: '2-digit',
	});
}

export function clampResultsHeight(height: number, total: number): number {
	const minHeight = 140;
	const maxHeight = Math.max(minHeight, total - 180);
	return Math.min(maxHeight, Math.max(minHeight, height));
}
