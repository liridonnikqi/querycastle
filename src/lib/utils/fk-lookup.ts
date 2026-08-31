import type { DatabaseExplorer, DatabaseForeignKey, DatabaseType, QueryResultPayload } from '$lib/rpc';
import { formatFkOptionLabel, pickFkLabelColumns } from '$lib/utils/grid-editors';
import { findExplorerTable } from '$lib/utils/relation-resolve';
import { quoteLiteral } from '$lib/utils/relation-sql';
import { quoteSqlIdentifier } from '$lib/utils/sql';

export type FkOption = {
	id: unknown;
	label: string;
};

const LOOKUP_LIMIT = 200;

function likePattern(query: string): string {
	return `%${query.replaceAll('\\', '\\\\').replaceAll('%', '\\%').replaceAll('_', '\\_')}%`;
}

export function buildFkLookupSql(params: {
	databaseType: DatabaseType;
	fk: DatabaseForeignKey;
	labelColumns: string[];
	search?: string;
}): string {
	const { databaseType, fk, labelColumns, search } = params;
	const tableRef = `${quoteSqlIdentifier(databaseType, fk.referencedSchema)}.${quoteSqlIdentifier(databaseType, fk.referencedTable)}`;
	const idCol = quoteSqlIdentifier(databaseType, fk.referencedColumn);
	const labelCols = labelColumns.map((column) => quoteSqlIdentifier(databaseType, column));
	const selectList = [idCol, ...labelCols].join(', ');
	const orderCol = labelCols[0] ?? idCol;
	const orderBy =
		databaseType === 'mysql'
			? ` order by ${orderCol} asc`
			: ` order by ${orderCol} asc nulls last`;

	let where = '';
	const trimmed = search?.trim() ?? '';
	if (trimmed.length > 0) {
		const pattern = quoteLiteral(databaseType, likePattern(trimmed));
		const likeOp = databaseType === 'postgres' ? 'ilike' : 'like';
		const predicates = [idCol, ...labelCols].map((column) => {
			if (databaseType === 'postgres') {
				return `cast(${column} as text) ${likeOp} ${pattern}`;
			}
			if (databaseType === 'mysql') {
				return `cast(${column} as char) ${likeOp} ${pattern}`;
			}
			return `cast(${column} as text) ${likeOp} ${pattern}`;
		});
		where = ` where ${predicates.join(' or ')}`;
	}

	return `select ${selectList} from ${tableRef}${where}${orderBy} limit ${LOOKUP_LIMIT};`;
}

export function rowsToFkOptions(
	payload: QueryResultPayload,
	idColumn: string,
	labelColumns: string[],
): FkOption[] {
	const idKey =
		payload.columns.find((column) => column.toLowerCase() === idColumn.toLowerCase()) ??
		payload.columns[0];
	if (!idKey) return [];
	const labelKeys = labelColumns
		.map(
			(name) =>
				payload.columns.find((column) => column.toLowerCase() === name.toLowerCase()) ?? name,
		)
		.filter((column) => column !== idKey);

	const options: FkOption[] = [];
	for (const row of payload.rows) {
		const id = row[idKey];
		if (id == null) continue;
		options.push({
			id,
			label: formatFkOptionLabel(
				id,
				labelKeys.map((column) => row[column]),
			),
		});
	}
	return options;
}

export async function loadFkOptions(params: {
	runQuery: (sql: string) => Promise<QueryResultPayload>;
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	fk: DatabaseForeignKey;
	search?: string;
}): Promise<FkOption[]> {
	const table = findExplorerTable(
		params.explorer,
		params.fk.referencedSchema,
		params.fk.referencedTable,
	);
	const labelColumns = pickFkLabelColumns(table, params.fk.referencedColumn);
	const sql = buildFkLookupSql({
		databaseType: params.databaseType,
		fk: params.fk,
		labelColumns,
		search: params.search,
	});
	const payload = await params.runQuery(sql);
	return rowsToFkOptions(payload, params.fk.referencedColumn, labelColumns);
}
