import type { DatabaseExplorer, DatabaseTable, DatabaseType } from '$lib/rpc';
import { findExplorerTable } from '$lib/utils/schema-objects';
import { quoteSqlIdentifier } from '$lib/utils/sql';

export const HIDDEN_ROW_ID_COLUMN = '_querycastle_row_id';
export const MYSQL_ROW_ALIAS = '_querycastle_src';

export type DialectCapabilities = {
	formatLanguage: 'mysql' | 'postgresql';
	canCreateDatabase: boolean;
	supportsNullsLast: boolean;
	usesInformationSchema: boolean;
};

export function dialectCapabilities(
	databaseType: DatabaseType,
): DialectCapabilities {
	return {
		formatLanguage: databaseType === 'mysql' ? 'mysql' : 'postgresql',
		canCreateDatabase: databaseType === 'postgres',
		supportsNullsLast: databaseType !== 'mysql',
		usesInformationSchema: databaseType !== 'sqlite',
	};
}

export function engineDisplayName(databaseType: DatabaseType): string {
	if (databaseType === 'mysql') return 'MySQL';
	if (databaseType === 'sqlite') return 'SQLite';
	return 'PostgreSQL';
}

export function qualifyTable(
	databaseType: DatabaseType,
	schema: string,
	table: string,
): string {
	return `${quoteSqlIdentifier(databaseType, schema)}.${quoteSqlIdentifier(databaseType, table)}`;
}

export function primaryKeyColumns(table: DatabaseTable): string[] {
	return table.columns.filter((column) => column.isPrimary).map((column) => column.name);
}

export function buildMysqlRowHashExpression(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
	columnPrefix?: string,
): string | null {
	const tableMeta = findExplorerTable(explorer, schema, table);
	if (!tableMeta) return null;
	const columns = primaryKeyColumns(tableMeta);
	if (columns.length === 0) return null;

	const parts = columns.map((column) => {
		const safeColumn = quoteSqlIdentifier('mysql', column);
		const qualifiedColumn = columnPrefix
			? `${columnPrefix}.${safeColumn}`
			: safeColumn;
		return `coalesce(cast(${qualifiedColumn} as char), '__querycastle_null__')`;
	});
	return `md5(concat_ws(char(31), ${parts.join(', ')}))`;
}

export function mysqlRowAlias(
	databaseType: DatabaseType,
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): string | undefined {
	if (databaseType !== 'mysql') return undefined;
	return buildMysqlRowHashExpression(explorer, schema, table, MYSQL_ROW_ALIAS)
		? MYSQL_ROW_ALIAS
		: undefined;
}
