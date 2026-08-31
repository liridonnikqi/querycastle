import { diffChars } from 'diff';
import type { DatabaseType } from '$lib/rpc';
import { quoteLiteral } from '$lib/utils/relation-sql';
import { quoteSqlIdentifier } from '$lib/utils/sql';

export type PendingUpdate = { ctid: string; values: Record<string, unknown> };
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

export function pendingChangeCount(params: {
	updates: Map<string, Record<string, unknown>>;
	inserts: PendingInsert[];
	deletes: Set<string>;
}): number {
	let fields = 0;
	for (const values of params.updates.values()) {
		fields += Object.keys(values).filter((key) => key !== '_querycastle_ctid').length;
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
		const id = String(row['_querycastle_ctid'] ?? '');
		if (id) rowNumber.set(id, index + 1);
	});

	for (const [ctid, values] of params.updates) {
		const row = params.rows.find((item) => String(item['_querycastle_ctid'] ?? '') === ctid);
		const n = rowNumber.get(ctid);
		const rowLabel = n != null ? `row ${n}` : 'row';
		for (const [column, next] of Object.entries(values)) {
			if (column === '_querycastle_ctid') continue;
			cards.push({
				id: `u:${ctid}:${column}`,
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

	for (const ctid of params.deletes) {
		const snapshot =
			params.deletedSnapshots?.get(ctid) ??
			params.rows.find((item) => String(item['_querycastle_ctid'] ?? '') === ctid);
		const n = rowNumber.get(ctid);
		const rowLabel = n != null ? `row ${n}` : 'row';
		const summary = snapshot
			? Object.entries(snapshot)
					.filter(([key]) => key !== '_querycastle_ctid')
					.slice(0, 3)
					.map(([column, value]) => `${column}=${formatDiffValue(value)}`)
					.join(', ')
			: 'Row deleted';
		cards.push({
			id: `d:${ctid}`,
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
		.filter(([column]) => column !== '_querycastle_ctid')
		.map(
			([column, value]) =>
				`${quoteSqlIdentifier(databaseType, column)} = ${quoteLiteral(databaseType, value)}`,
		)
		.join(', ');
}

export function buildPendingSqlPreview(params: {
	databaseType: DatabaseType;
	schema: string;
	table: string;
	updates: Array<{ ctid: string; values: Record<string, unknown> }>;
	deletes: string[];
	inserts: Array<Record<string, unknown>>;
}): string {
	const tableRef = qualifyTable(params.databaseType, params.schema, params.table);
	const statements: string[] = [];

	for (const update of params.updates) {
		const sets = setClause(params.databaseType, update.values);
		if (!sets) continue;
		if (params.databaseType === 'postgres') {
			statements.push(
				`update ${tableRef} set ${sets} where ctid = ${quoteLiteral(params.databaseType, update.ctid)}::tid;`,
			);
		} else if (params.databaseType === 'sqlite') {
			statements.push(
				`update ${tableRef} set ${sets} where rowid = ${quoteLiteral(params.databaseType, update.ctid)};`,
			);
		} else {
			statements.push(`update ${tableRef} set ${sets} where /* row */ ${quoteLiteral(params.databaseType, update.ctid)};`);
		}
	}

	for (const ctid of params.deletes) {
		if (params.databaseType === 'postgres') {
			statements.push(
				`delete from ${tableRef} where ctid = ${quoteLiteral(params.databaseType, ctid)}::tid;`,
			);
		} else if (params.databaseType === 'sqlite') {
			statements.push(
				`delete from ${tableRef} where rowid = ${quoteLiteral(params.databaseType, ctid)};`,
			);
		} else {
			statements.push(`delete from ${tableRef} where /* row */ ${quoteLiteral(params.databaseType, ctid)};`);
		}
	}

	for (const row of params.inserts) {
		const entries = Object.entries(row).filter(([column]) => column !== '_querycastle_ctid');
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
