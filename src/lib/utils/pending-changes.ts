import { diffChars } from 'diff';
import type { DatabaseType } from '$lib/rpc';
import { HIDDEN_ROW_ID_COLUMN } from '$lib/utils/dialect';
import { quoteLiteral } from '$lib/utils/relation-sql';
import { quoteSqlIdentifier } from '$lib/utils/sql';

export type PendingUpdate = { rowId: string; values: Record<string, unknown> };
export type PendingInsert = { id: string; values: Record<string, unknown> };

export type PendingChangeCard = {
	id: string;
	kind: 'update' | 'insert' | 'delete';
	badge: 'U' | 'I' | 'D';
	title: string;
	before: string | null;
	after: string | null;
};

export function formatDiffValue(value: unknown): string {
	if (value === null || value === undefined || value === '') return '';
	if (typeof value === 'boolean') return value ? 'True' : 'False';
	return String(value);
}

export type DiffHunk = { kind: 'eq' | 'add' | 'del'; text: string };

export function diffText(before: string, after: string): DiffHunk[] {
	if (before === after) return [{ kind: 'eq', text: after }];
	if (before.length === 0) return [{ kind: 'add', text: after }];
	if (after.length === 0) return [{ kind: 'del', text: before }];

	const parts = diffChars(before, after, { maxEditLength: 4_000 });
	if (!parts) {
		return [
			{ kind: 'del', text: before },
			{ kind: 'add', text: after },
		];
	}

	return parts.map((part) => ({
		kind: part.added ? 'add' : part.removed ? 'del' : 'eq',
		text: part.value,
	}));
}

export function formatApplyResultMessage(result: {
	updated: number;
	deleted: number;
	inserted: number;
}): string {
	const parts: string[] = [];
	if (result.inserted > 0) {
		parts.push(result.inserted === 1 ? 'Added 1 row' : `Added ${result.inserted} rows`);
	}
	if (result.updated > 0) {
		parts.push(result.updated === 1 ? 'Updated 1 row' : `Updated ${result.updated} rows`);
	}
	if (result.deleted > 0) {
		parts.push(result.deleted === 1 ? 'Deleted 1 row' : `Deleted ${result.deleted} rows`);
	}
	return parts.length > 0 ? parts.join('. ') : 'Changes saved';
}

export function pendingChangeCount(params: {
	updates: Map<string, Record<string, unknown>>;
	inserts: PendingInsert[];
	deletes: Set<string>;
}): number {
	let fields = 0;
	for (const values of params.updates.values()) {
		fields += Object.keys(values).filter((key) => key !== HIDDEN_ROW_ID_COLUMN).length;
	}
	return fields + params.inserts.length + params.deletes.size;
}

export function buildPendingChangeCards(params: {
	schema: string;
	table: string;
	rows: Array<Record<string, unknown>>;
	updates: Map<string, Record<string, unknown>>;
	inserts: PendingInsert[];
	deletes: Set<string>;
	deletedSnapshots?: Map<string, Record<string, unknown>>;
}): PendingChangeCard[] {
	const cards: PendingChangeCard[] = [];
	const tableRef = `${params.schema}.${params.table}`;
	const rowNumber = new Map<string, number>();
	params.rows.forEach((row, index) => {
		const id = String(row[HIDDEN_ROW_ID_COLUMN] ?? '');
		if (id) rowNumber.set(id, index + 1);
	});

	for (const [rowId, values] of params.updates) {
		const row = params.rows.find((item) => String(item[HIDDEN_ROW_ID_COLUMN] ?? '') === rowId);
		const n = rowNumber.get(rowId);
		const rowLabel = n != null ? `row ${n}` : 'row';
		for (const [column, next] of Object.entries(values)) {
			if (column === HIDDEN_ROW_ID_COLUMN) continue;
			cards.push({
				id: `u:${rowId}:${column}`,
				kind: 'update',
				badge: 'U',
				title: `${tableRef} > ${rowLabel} > ${column}`,
				before: formatDiffValue(row?.[column]),
				after: formatDiffValue(next),
			});
		}
	}

	params.inserts.forEach((insert, index) => {
		const summary = Object.entries(insert.values)
			.filter(([, value]) => value != null && value !== '')
			.map(([column, value]) => `${column}=${formatDiffValue(value)}`)
			.slice(0, 4)
			.join(', ');
		cards.push({
			id: `i:${insert.id}`,
			kind: 'insert',
			badge: 'I',
			title: `${tableRef} > new row ${index + 1}`,
			before: null,
			after: summary.length > 0 ? summary : 'New row',
		});
	});

	for (const rowId of params.deletes) {
		const snapshot =
			params.deletedSnapshots?.get(rowId) ??
			params.rows.find((item) => String(item[HIDDEN_ROW_ID_COLUMN] ?? '') === rowId);
		const n = rowNumber.get(rowId);
		const rowLabel = n != null ? `row ${n}` : 'row';
		const summary = snapshot
			? Object.entries(snapshot)
					.filter(([key]) => key !== HIDDEN_ROW_ID_COLUMN)
					.slice(0, 3)
					.map(([column, value]) => `${column}=${formatDiffValue(value)}`)
					.join(', ')
			: 'Row deleted';
		cards.push({
			id: `d:${rowId}`,
			kind: 'delete',
			badge: 'D',
			title: `${tableRef} > ${rowLabel}`,
			before: summary,
			after: null,
		});
	}

	return cards;
}

function qualifyTable(databaseType: DatabaseType, schema: string, table: string): string {
	return `${quoteSqlIdentifier(databaseType, schema)}.${quoteSqlIdentifier(databaseType, table)}`;
}

function setClause(
	databaseType: DatabaseType,
	values: Record<string, unknown>,
): string {
	return Object.entries(values)
		.filter(([column]) => column !== HIDDEN_ROW_ID_COLUMN)
		.map(
			([column, value]) =>
				`${quoteSqlIdentifier(databaseType, column)} = ${quoteLiteral(databaseType, value)}`,
		)
		.join(', ');
}

function rowById(
	rows: Array<Record<string, unknown>> | undefined,
	rowId: string,
): Record<string, unknown> | undefined {
	return rows?.find((row) => String(row[HIDDEN_ROW_ID_COLUMN] ?? '') === rowId);
}

function whereLiteral(databaseType: DatabaseType, value: unknown): string {
	if (value === null || value === undefined) return 'NULL';
	if (typeof value === 'number' && Number.isFinite(value)) return String(value);
	if (typeof value === 'bigint') return String(value);
	if (typeof value === 'boolean') return quoteLiteral(databaseType, value);
	const text = String(value).trim();
	if (/^-?\d+(\.\d+)?$/.test(text)) return text;
	return quoteLiteral(databaseType, value);
}

function pkColumnsForRow(
	pkColumns: string[] | undefined,
	row: Record<string, unknown> | undefined,
): string[] {
	if (pkColumns && pkColumns.length > 0) return pkColumns;
	if (row && Object.prototype.hasOwnProperty.call(row, 'id')) return ['id'];
	return [];
}

export function rowWhereSql(
	databaseType: DatabaseType,
	row: Record<string, unknown> | undefined,
	rowId: string,
	pkColumns?: string[],
): string {
	const keys = pkColumnsForRow(pkColumns, row);
	if (keys.length > 0 && row) {
		const parts: string[] = [];
		let complete = true;
		for (const column of keys) {
			if (!Object.prototype.hasOwnProperty.call(row, column) || row[column] === undefined) {
				complete = false;
				break;
			}
			parts.push(
				`${quoteSqlIdentifier(databaseType, column)} = ${whereLiteral(databaseType, row[column])}`,
			);
		}
		if (complete && parts.length > 0) return parts.join(' and ');
	}

	const loc = quoteLiteral(databaseType, rowId);
	if (databaseType === 'postgres') return `ctid = ${loc}::tid`;
	if (databaseType === 'sqlite') return `rowid = ${loc}`;
	return `/* row */ ${loc}`;
}

export function buildRowInspectSql(params: {
	databaseType: DatabaseType;
	schema: string;
	table: string;
	rowId: string;
	row?: Record<string, unknown>;
	pkColumns?: string[];
}): string {
	const tableRef = qualifyTable(params.databaseType, params.schema, params.table);
	const where = rowWhereSql(
		params.databaseType,
		params.row,
		params.rowId,
		params.pkColumns,
	);
	return `select * from ${tableRef} where ${where};`;
}

export function buildPendingSqlPreview(params: {
	databaseType: DatabaseType;
	schema: string;
	table: string;
	updates: Array<{ rowId: string; values: Record<string, unknown> }>;
	deletes: string[];
	inserts: Array<Record<string, unknown>>;
	rows?: Array<Record<string, unknown>>;
	deletedSnapshots?: Map<string, Record<string, unknown>>;
	pkColumns?: string[];
}): string {
	const tableRef = qualifyTable(params.databaseType, params.schema, params.table);
	const statements: string[] = [];

	for (const update of params.updates) {
		const sets = setClause(params.databaseType, update.values);
		if (!sets) continue;
		const row = rowById(params.rows, update.rowId);
		const where = rowWhereSql(
			params.databaseType,
			row,
			update.rowId,
			params.pkColumns,
		);
		statements.push(`update ${tableRef} set ${sets} where ${where};`);
	}

	for (const rowId of params.deletes) {
		const row =
			params.deletedSnapshots?.get(rowId) ?? rowById(params.rows, rowId);
		const where = rowWhereSql(
			params.databaseType,
			row,
			rowId,
			params.pkColumns,
		);
		statements.push(`delete from ${tableRef} where ${where};`);
	}

	for (const row of params.inserts) {
		const entries = Object.entries(row).filter(([column]) => column !== HIDDEN_ROW_ID_COLUMN);
		if (entries.length === 0) continue;
		const cols = entries
			.map(([column]) => quoteSqlIdentifier(params.databaseType, column))
			.join(', ');
		const values = entries
			.map(([, value]) => quoteLiteral(params.databaseType, value))
			.join(', ');
		statements.push(`insert into ${tableRef} (${cols}) values (${values});`);
	}

	return statements.join('\n');
}
