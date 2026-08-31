import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
import { mysqlRowAlias } from '$lib/utils/dialect';
import { quoteLiteral } from '$lib/utils/relation-sql';
import { isExplorerView } from '$lib/utils/schema-objects';
import { quoteSqlIdentifier } from '$lib/utils/sql';
import {
	buildLimitClause,
	buildOrderByClause,
	buildTableSelect,
	qualifyTable,
} from '$lib/utils/table-select';

export const PAGE_SIZE_OPTIONS = [25, 50, 100, 200] as const;
export type PageSize = (typeof PAGE_SIZE_OPTIONS)[number];

export type GridSort = { column: string; dir: 'asc' | 'desc' };

export type GridColumnFilter = { column: string; value: string };

export function extractWhereClause(sql: string): string {
	const cleaned = sql.trim().replace(/;+\s*$/, '');
	const match = cleaned.match(/\bwhere\b\s+([\s\S]+?)\s*(?:order\s+by|limit)\b/i);
	return match?.[1]?.trim() ?? '';
}

export function likeContainsPattern(query: string): string {
	return `%${query.replaceAll('\\', '\\\\').replaceAll('%', '\\%').replaceAll('_', '\\_')}%`;
}

export function buildFilterPredicate(
	databaseType: DatabaseType,
	column: string,
	value: string,
	tableAlias?: string,
): string {
	const ident = quoteSqlIdentifier(databaseType, column);
	const qualified = tableAlias ? `${tableAlias}.${ident}` : ident;
	const pattern = quoteLiteral(databaseType, likeContainsPattern(value));
	if (databaseType === 'postgres') {
		return `cast(${qualified} as text) ilike ${pattern}`;
	}
	if (databaseType === 'mysql') {
		return `cast(${qualified} as char) like ${pattern}`;
	}
	return `cast(${qualified} as text) like ${pattern}`;
}

export function stripRowAlias(sqlFragment: string): string {
	return sqlFragment.replace(/_querycastle_src\./g, '');
}

export function combineWhereClauses(parts: string[]): string {
	const clauses = parts.map((part) => part.trim()).filter((part) => part.length > 0);
	if (clauses.length === 0) return '';
	return ` where ${clauses.join(' and ')}`;
}

export { buildLimitClause, buildOrderByClause, qualifyTable };

export function buildTableCountSql(params: {
	databaseType: DatabaseType;
	schema: string;
	table: string;
	baseWhere: string;
	filters: GridColumnFilter[];
}): string {
	const tableRef = qualifyTable(params.databaseType, params.schema, params.table);
	const filterParts = params.filters
		.filter((item) => item.value.trim().length > 0)
		.map((item) => buildFilterPredicate(params.databaseType, item.column, item.value.trim()));
	const where = combineWhereClauses([stripRowAlias(params.baseWhere), ...filterParts]);
	return `select count(*) as count from ${tableRef}${where};`;
}

export function parseCountResult(rows: Array<Record<string, unknown>>): number {
	const row = rows[0];
	if (!row) return 0;
	const value =
		row.count ??
		row.COUNT ??
		row['count(*)'] ??
		row['COUNT(*)'] ??
		Object.values(row)[0];
	const num = Number(value);
	return Number.isFinite(num) ? num : 0;
}

export function buildTableBrowseSql(params: {
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	schema: string;
	table: string;
	baseWhere: string;
	filters: GridColumnFilter[];
	sort: GridSort | null;
	limit: number;
	offset: number;
}): string | null {
	const { databaseType, explorer, schema, table, baseWhere, filters, sort, limit, offset } =
		params;
	const alias = mysqlRowAlias(databaseType, explorer, schema, table);
	const filterParts = filters
		.filter((item) => item.value.trim().length > 0)
		.map((item) => buildFilterPredicate(databaseType, item.column, item.value.trim(), alias));
	const where = combineWhereClauses([stripRowAlias(baseWhere), ...filterParts]);
	return buildTableSelect({
		databaseType,
		explorer,
		schema,
		table,
		whereClause: where,
		orderClause: buildOrderByClause(databaseType, sort),
		limit,
		offset,
		includeRowId: !isExplorerView(explorer, schema, table),
	});
}

export function nextSortState(current: GridSort | null, column: string): GridSort | null {
	if (!current || current.column !== column) return { column, dir: 'asc' };
	if (current.dir === 'asc') return { column, dir: 'desc' };
	return null;
}

export function filterSortRows(
	rows: Array<Record<string, unknown>>,
	filters: GridColumnFilter[],
	sort: GridSort | null,
): Array<Record<string, unknown>> {
	let next = rows;
	const active = filters.filter((item) => item.value.trim().length > 0);
	if (active.length > 0) {
		next = next.filter((row) =>
			active.every((item) =>
				String(row[item.column] ?? '')
					.toLowerCase()
					.includes(item.value.trim().toLowerCase()),
			),
		);
	}
	if (sort) {
		const { column, dir } = sort;
		next = [...next].sort((a, b) => {
			const av = a[column];
			const bv = b[column];
			if (av == null && bv == null) return 0;
			if (av == null) return 1;
			if (bv == null) return -1;
			if (typeof av === 'number' && typeof bv === 'number') {
				return dir === 'asc' ? av - bv : bv - av;
			}
			const cmp = String(av).localeCompare(String(bv), undefined, { numeric: true });
			return dir === 'asc' ? cmp : -cmp;
		});
	}
	return next;
}

export function totalPages(totalRows: number, pageSize: number): number {
	if (totalRows <= 0) return 1;
	return Math.max(1, Math.ceil(totalRows / Math.max(1, pageSize)));
}
