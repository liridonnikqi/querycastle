import type { DatabaseExplorer, DatabaseForeignKey, DatabaseType } from '$lib/rpc';
import { buildMysqlRowHashExpression } from '$lib/utils/editable-query';
import { quoteSqlIdentifier } from '$lib/utils/sql';
import type { RelationHop } from '$lib/utils/workspace';

export const HIDDEN_ROW_ID_COLUMN = '_querycastle_ctid';
const TITLE_VALUE_MAX = 24;
const FOLLOW_ROW_LIMIT = 100;

export function quoteIdent(databaseType: DatabaseType, name: string): string {
	return quoteSqlIdentifier(databaseType, name);
}

export function quoteLiteral(databaseType: DatabaseType, value: unknown): string {
	if (value === null || value === undefined) return 'NULL';
	if (typeof value === 'boolean') {
		if (databaseType === 'postgres') return value ? 'TRUE' : 'FALSE';
		return value ? '1' : '0';
	}
	if (typeof value === 'number' && Number.isFinite(value)) return String(value);
	if (typeof value === 'bigint') return String(value);

	let text: string;
	if (typeof value === 'string') text = value;
	else if (value instanceof Date) text = value.toISOString();
	else {
		try {
			text = JSON.stringify(value) ?? String(value);
		} catch {
			text = String(value);
		}
	}

	if (databaseType === 'mysql') {
		text = text.replaceAll('\\', '\\\\').replaceAll('\0', '\\0').replaceAll("'", "''");
	} else {
		text = text.replaceAll("'", "''");
	}
	return `'${text}'`;
}

export function isFollowableValue(value: unknown): boolean {
	if (value === null || value === undefined) return false;
	if (typeof value === 'string' && value.trim() === '') return false;
	return true;
}

export function formatFollowValue(value: unknown, maxLength = TITLE_VALUE_MAX): string {
	const raw = value == null ? 'NULL' : String(value);
	if (raw.length <= maxLength) return raw;
	return `${raw.slice(0, Math.max(1, maxLength - 1))}…`;
}

export function createRelationHop(params: {
	direction: RelationHop['direction'];
	from: RelationHop['from'];
	to: RelationHop['to'];
}): RelationHop {
	return {
		direction: params.direction,
		from: params.from,
		to: params.to,
		label: `${params.from.table}.${params.from.column} → ${params.to.table}.${params.to.column}`,
	};
}

export function buildFollowTabTitle(hop: RelationHop): string {
	return `${hop.to.table} ${hop.to.column}=${formatFollowValue(hop.from.value)}`;
}

export function hopValuesEqual(a: unknown, b: unknown): boolean {
	if (a === b) return true;
	if (a == null && b == null) return true;
	return String(a) === String(b);
}

export function hopsEqual(a: RelationHop, b: RelationHop): boolean {
	return (
		a.direction === b.direction &&
		a.from.schema === b.from.schema &&
		a.from.table === b.from.table &&
		a.from.column === b.from.column &&
		hopValuesEqual(a.from.value, b.from.value) &&
		a.to.schema === b.to.schema &&
		a.to.table === b.to.table &&
		a.to.column === b.to.column
	);
}

export function trailsEqual(a: RelationHop[], b: RelationHop[]): boolean {
	if (a.length !== b.length) return false;
	return a.every((hop, index) => hopsEqual(hop, b[index]!));
}

export function buildTrailCrumbs(trail: RelationHop[]): Array<{
	index: number;
	label: string;
	tooltip: string;
	isCurrent: boolean;
}> {
	if (trail.length === 0) return [];
	const origin = trail[0]!.from;
	const crumbs = [
		{
			index: 0,
			label: origin.table,
			tooltip: `${origin.schema}.${origin.table}`,
			isCurrent: false,
		},
	];
	for (let index = 0; index < trail.length; index++) {
		const hop = trail[index]!;
		const fullValue = hop.from.value == null ? 'NULL' : String(hop.from.value);
		crumbs.push({
			index: index + 1,
			label: `${hop.to.table} ${hop.to.column}=${formatFollowValue(hop.from.value)}`,
			tooltip: `${hop.to.schema}.${hop.to.table} where ${hop.to.column} = ${fullValue}`,
			isCurrent: false,
		});
	}
	crumbs[crumbs.length - 1]!.isCurrent = true;
	return crumbs;
}

function firstOrderColumn(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): string | null {
	return (
		explorer?.schemas
			.find((item) => item.name === schema)
			?.tables.find((item) => item.name === table)?.columns[0]?.name ??
		explorer?.schemas
			.find((item) => item.name.toLowerCase() === schema.toLowerCase())
			?.tables.find((item) => item.name.toLowerCase() === table.toLowerCase())
			?.columns[0]?.name ??
		null
	);
}

function qualifyTable(databaseType: DatabaseType, schema: string, table: string): string {
	return `${quoteIdent(databaseType, schema)}.${quoteIdent(databaseType, table)}`;
}

function buildOrderByClause(
	databaseType: DatabaseType,
	orderColumn: string | null,
): string {
	if (!orderColumn) return '';
	const quoted = quoteIdent(databaseType, orderColumn);
	if (databaseType === 'mysql') return ` order by ${quoted} asc`;
	return ` order by ${quoted} asc nulls last`;
}

export function buildFilteredTableSql(params: {
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	schema: string;
	table: string;
	whereColumn: string;
	value: unknown;
}): string | null {
	if (!isFollowableValue(params.value)) return null;

	const { databaseType, explorer, schema, table, whereColumn, value } = params;
	const qualifiedTable = qualifyTable(databaseType, schema, table);
	const quotedWhere = quoteIdent(databaseType, whereColumn);
	const literal = quoteLiteral(databaseType, value);
	const orderColumn = firstOrderColumn(explorer, schema, table) ?? whereColumn;
	const orderByClause = buildOrderByClause(databaseType, orderColumn);
	const whereClause = ` where ${quotedWhere} = ${literal}`;

	if (databaseType === 'sqlite') {
		return `select cast(rowid as text) as ${HIDDEN_ROW_ID_COLUMN}, * from ${qualifiedTable}${whereClause}${orderByClause} limit ${FOLLOW_ROW_LIMIT};`;
	}

	if (databaseType === 'mysql') {
		const rowHashWithAlias = buildMysqlRowHashExpression(
			explorer,
			schema,
			table,
			'_querycastle_src',
		);
		if (!rowHashWithAlias) return null;
		return `select ${rowHashWithAlias} as ${HIDDEN_ROW_ID_COLUMN}, _querycastle_src.* from ${qualifiedTable} as _querycastle_src where _querycastle_src.${quotedWhere} = ${literal}${orderByClause} limit ${FOLLOW_ROW_LIMIT};`;
	}

	return `select ctid::text as ${HIDDEN_ROW_ID_COLUMN}, * from ${qualifiedTable}${whereClause}${orderByClause} limit ${FOLLOW_ROW_LIMIT};`;
}

export function buildOutgoingFollowSql(params: {
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	fromTable: { schema: string; table: string };
	fk: DatabaseForeignKey;
	value: unknown;
}): string | null {
	return buildFilteredTableSql({
		databaseType: params.databaseType,
		explorer: params.explorer,
		schema: params.fk.referencedSchema,
		table: params.fk.referencedTable,
		whereColumn: params.fk.referencedColumn,
		value: params.value,
	});
}

export function buildIncomingFollowSql(params: {
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	childTable: { schema: string; table: string };
	fk: DatabaseForeignKey;
	parentValue: unknown;
}): string | null {
	return buildFilteredTableSql({
		databaseType: params.databaseType,
		explorer: params.explorer,
		schema: params.childTable.schema,
		table: params.childTable.table,
		whereColumn: params.fk.column,
		value: params.parentValue,
	});
}

export function buildFollowSqlFromHop(params: {
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	hop: RelationHop;
}): string | null {
	return buildFilteredTableSql({
		databaseType: params.databaseType,
		explorer: params.explorer,
		schema: params.hop.to.schema,
		table: params.hop.to.table,
		whereColumn: params.hop.to.column,
		value: params.hop.from.value,
	});
}
