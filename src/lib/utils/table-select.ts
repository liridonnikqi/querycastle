import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
import {
	HIDDEN_ROW_ID_COLUMN,
	ROW_SOURCE_ALIAS,
	buildPkHashExpression,
	dialectCapabilities,
	qualifyTable,
	usesPkHashRowId,
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

export function buildLimitClause(
	limit: number,
	offset = 0,
	databaseType: DatabaseType = 'postgres',
): string {
	const safeLimit = Math.max(1, Math.floor(limit));
	const safeOffset = Math.max(0, Math.floor(offset));
	if (databaseType === 'mssql') {
		return ` offset ${safeOffset} rows fetch next ${safeLimit} rows only`;
	}
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
		limit,
		offset = 0,
	} = params;
	const viewingView = isExplorerView(explorer, schema, table);
	const includeRowId = (params.includeRowId ?? true) && !viewingView;
	const tableRef = qualifyTable(databaseType, schema, table);
	let orderClause = params.orderClause ?? '';
	if (databaseType === 'mssql' && limit != null && !orderClause.trim()) {
		orderClause = ' order by (select null)';
	}
	const paging = limit == null ? '' : buildLimitClause(limit, offset, databaseType);

	if (!includeRowId) {
		return `select ${selectList} from ${tableRef}${whereClause}${orderClause}${paging};`;
	}

	if (databaseType === 'sqlite') {
		return `select cast(rowid as text) as ${HIDDEN_ROW_ID_COLUMN}, ${selectList} from ${tableRef}${whereClause}${orderClause}${paging};`;
	}

	if (usesPkHashRowId(databaseType)) {
		const rowHash = buildPkHashExpression(
			databaseType,
			explorer,
			schema,
			table,
			ROW_SOURCE_ALIAS,
		);
		if (!rowHash) {
			return `select ${selectList} from ${tableRef}${whereClause}${orderClause}${paging};`;
		}
		const aliasedSelect =
			selectList.trim() === '*' ? `${ROW_SOURCE_ALIAS}.*` : selectList;
		return `select ${rowHash} as ${HIDDEN_ROW_ID_COLUMN}, ${aliasedSelect} from ${tableRef} as ${ROW_SOURCE_ALIAS}${whereClause}${orderClause}${paging};`;
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
	if (usesPkHashRowId(databaseType)) {
		return buildPkHashExpression(databaseType, explorer, schema, table) !== null;
	}
	return true;
}
