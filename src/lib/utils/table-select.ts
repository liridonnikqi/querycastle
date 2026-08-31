import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
import {
	HIDDEN_ROW_ID_COLUMN,
	MYSQL_ROW_ALIAS,
	buildMysqlRowHashExpression,
	dialectCapabilities,
	qualifyTable,
} from '$lib/utils/dialect';
import { isExplorerView } from '$lib/utils/schema-objects';
import { quoteSqlIdentifier } from '$lib/utils/sql';

export { HIDDEN_ROW_ID_COLUMN, qualifyTable } from '$lib/utils/dialect';

export type TableSelectParams = {
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	schema: string;
	table: string;
	selectList?: string;
	whereClause?: string;
	orderClause?: string;
	limit?: number;
	offset?: number;
	includeRowId?: boolean;
};

export function buildOrderByClause(
	databaseType: DatabaseType,
	sort: { column: string; dir: 'asc' | 'desc' } | null,
): string {
	if (!sort) return '';
	const ident = quoteSqlIdentifier(databaseType, sort.column);
	if (dialectCapabilities(databaseType).supportsNullsLast) {
		return ` order by ${ident} ${sort.dir} nulls last`;
	}
	return ` order by ${ident} ${sort.dir}`;
}

export function buildLimitClause(limit: number, offset = 0): string {
	const safeLimit = Math.max(1, Math.floor(limit));
	const safeOffset = Math.max(0, Math.floor(offset));
	if (safeOffset === 0) return ` limit ${safeLimit}`;
	return ` limit ${safeLimit} offset ${safeOffset}`;
}

export function buildTableSelect(params: TableSelectParams): string | null {
	const {
		databaseType,
		explorer,
		schema,
		table,
		selectList = '*',
		whereClause = '',
		orderClause = '',
		limit,
		offset = 0,
	} = params;
	const viewingView = isExplorerView(explorer, schema, table);
	const includeRowId = (params.includeRowId ?? true) && !viewingView;
	const tableRef = qualifyTable(databaseType, schema, table);
	const paging = limit == null ? '' : buildLimitClause(limit, offset);

	if (!includeRowId) {
		return `select ${selectList} from ${tableRef}${whereClause}${orderClause}${paging};`;
	}

	if (databaseType === 'sqlite') {
		return `select cast(rowid as text) as ${HIDDEN_ROW_ID_COLUMN}, ${selectList} from ${tableRef}${whereClause}${orderClause}${paging};`;
	}

	if (databaseType === 'mysql') {
		const rowHash = buildMysqlRowHashExpression(
			explorer,
			schema,
			table,
			MYSQL_ROW_ALIAS,
		);
		if (!rowHash) {
			return `select ${selectList} from ${tableRef}${whereClause}${orderClause}${paging};`;
		}
		const mysqlSelect =
			selectList.trim() === '*' ? `${MYSQL_ROW_ALIAS}.*` : selectList;
		return `select ${rowHash} as ${HIDDEN_ROW_ID_COLUMN}, ${mysqlSelect} from ${tableRef} as ${MYSQL_ROW_ALIAS}${whereClause}${orderClause}${paging};`;
	}

	return `select ctid::text as ${HIDDEN_ROW_ID_COLUMN}, ${selectList} from ${tableRef}${whereClause}${orderClause}${paging};`;
}

export function canEditTable(
	databaseType: DatabaseType,
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): boolean {
	if (isExplorerView(explorer, schema, table)) return false;
	if (databaseType === 'mysql') {
		return buildMysqlRowHashExpression(explorer, schema, table) !== null;
	}
	return true;
}
