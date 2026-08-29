import type { DatabaseExplorer, DatabaseForeignKey, DatabaseTable } from '$lib/rpc';
import { HIDDEN_ROW_ID_COLUMN } from '$lib/utils/relation-sql';

export type IncomingRelation = {
	schema: string;
	table: string;
	fk: DatabaseForeignKey;
};

function namesEqual(a: string, b: string): boolean {
	return a === b || a.toLowerCase() === b.toLowerCase();
}

export function findExplorerTable(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): DatabaseTable | null {
	if (!explorer) return null;
	const schemaMatch =
		explorer.schemas.find((item) => item.name === schema) ??
		explorer.schemas.find((item) => namesEqual(item.name, schema));
	if (!schemaMatch) return null;
	return (
		schemaMatch.tables.find((item) => item.name === table) ??
		schemaMatch.tables.find((item) => namesEqual(item.name, table)) ??
		null
	);
}

export function isSingleColumnForeignKey(
	tableForeignKeys: DatabaseForeignKey[],
	fk: DatabaseForeignKey,
): boolean {
	const siblings = tableForeignKeys.filter(
		(item) =>
			namesEqual(item.referencedSchema, fk.referencedSchema) &&
			namesEqual(item.referencedTable, fk.referencedTable),
	);
	const uniqueRefCols = new Set(siblings.map((item) => item.referencedColumn.toLowerCase()));
	const uniqueSrcCols = new Set(siblings.map((item) => item.column.toLowerCase()));
	if (siblings.length > 1 && uniqueRefCols.size > 1 && uniqueSrcCols.size > 1) {
		return false;
	}
	return true;
}

export function resolveOutgoingRelations(
	explorer: DatabaseExplorer | null,
	resultContext: { schema: string; table: string } | null,
	column: string,
): DatabaseForeignKey[] {
	if (!explorer || !resultContext) return [];
	if (column === HIDDEN_ROW_ID_COLUMN) return [];
	const table = findExplorerTable(explorer, resultContext.schema, resultContext.table);
	if (!table || table.foreignKeys.length === 0) return [];
	return table.foreignKeys.filter(
		(fk) => namesEqual(fk.column, column) && isSingleColumnForeignKey(table.foreignKeys, fk),
	);
}

export function resolveIncomingRelations(
	explorer: DatabaseExplorer | null,
	resultContext: { schema: string; table: string } | null,
): IncomingRelation[] {
	if (!explorer || !resultContext) return [];
	const incoming: IncomingRelation[] = [];
	for (const schema of explorer.schemas) {
		for (const table of schema.tables) {
			if (table.foreignKeys.length === 0) continue;
			for (const fk of table.foreignKeys) {
				if (!isSingleColumnForeignKey(table.foreignKeys, fk)) continue;
				if (
					namesEqual(fk.referencedSchema, resultContext.schema) &&
					namesEqual(fk.referencedTable, resultContext.table)
				) {
					incoming.push({ schema: table.schema, table: table.name, fk });
				}
			}
		}
	}
	return incoming;
}

export function outgoingFkColumns(
	explorer: DatabaseExplorer | null,
	resultContext: { schema: string; table: string } | null,
	columns: string[],
): Set<string> {
	const names = new Set<string>();
	for (const column of columns) {
		if (resolveOutgoingRelations(explorer, resultContext, column).length > 0) {
			names.add(column);
		}
	}
	return names;
}
