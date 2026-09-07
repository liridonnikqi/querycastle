import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
import { HIDDEN_ROW_ID_COLUMN } from '$lib/utils/dialect';
import { findExplorerTable } from '$lib/utils/schema-objects';
import { canEditTable } from '$lib/utils/table-select';

export function explorerTableColumnNames(
	explorer: DatabaseExplorer | null,
	context: { schema: string; table: string } | null,
): string[] {
	if (!context) return [];
	return (
		findExplorerTable(explorer, context.schema, context.table)?.columns.map(
			(column) => column.name,
		) ?? []
	);
}

export function visibleGridColumns(
	resultColumns: string[],
	explorerColumns: string[],
): string[] {
	const fromResult = resultColumns.filter(
		(column) => column !== HIDDEN_ROW_ID_COLUMN,
	);
	return fromResult.length > 0 ? fromResult : explorerColumns;
}

export function isGridEditable(params: {
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	context: { schema: string; table: string } | null;
	resultColumns: string[];
	rowCount: number;
	visibleColumns: string[];
}): boolean {
	if (!params.context) return false;
	if (
		!canEditTable(
			params.databaseType,
			params.explorer,
			params.context.schema,
			params.context.table,
		)
	) {
		return false;
	}
	if (params.resultColumns.includes(HIDDEN_ROW_ID_COLUMN)) return true;
	return params.rowCount === 0 && params.visibleColumns.length > 0;
}

export function isCommandQueryResult(params: {
	loading: boolean;
	sqlError: string;
	resultColumns: string[];
	visibleColumns: string[];
	lastRunSql: string;
}): boolean {
	return (
		!params.loading &&
		!params.sqlError &&
		params.resultColumns.length === 0 &&
		params.visibleColumns.length === 0 &&
		params.lastRunSql.trim().length > 0
	);
}

export function queryOutcomeMessage(params: {
	durationMs: number;
	rowCount: number;
	truncated: boolean;
	columns: string[];
}): string {
	if (params.columns.length === 0) {
		const affected =
			params.rowCount > 0
				? ` (${params.rowCount} ${params.rowCount === 1 ? 'row' : 'rows'} affected)`
				: '';
		return `Command completed successfully in ${params.durationMs}ms${affected}.`;
	}
	return `Last query executed successfully in ${params.durationMs}ms and returned ${params.rowCount} rows${params.truncated ? ' (capped)' : ''}.`;
}
