import type { DatabaseExplorer, DatabaseTable, DatabaseType } from '$lib/rpc';
import { findExplorerTable } from '$lib/utils/schema-objects';
import { quoteSqlIdentifier } from '$lib/utils/sql';
import type { SqlLanguage } from 'sql-formatter';

export const HIDDEN_ROW_ID_COLUMN = '_querycastle_row_id';
export const ROW_SOURCE_ALIAS = '_querycastle_src';
export const MYSQL_ROW_ALIAS = ROW_SOURCE_ALIAS;

export const POSTGRES_CREATE_DATABASE_ENCODINGS = [
	'UTF8',
	'LATIN1',
	'LATIN2',
	'WIN1252',
] as const;

export type DialectCapabilities = {
	formatLanguage: SqlLanguage;
	canCreateDatabase: boolean;
	createDatabaseEncodings: readonly string[];
	supportsNullsLast: boolean;
	supportsExplain: boolean;
	usesInformationSchema: boolean;
	defaultPort: number;
	defaultUser: string;
	defaultDatabase: string;
	defaultName: string;
};

const DIALECT_BY_TYPE: Record<DatabaseType, DialectCapabilities> = {
	mysql: {
		formatLanguage: 'mysql',
		canCreateDatabase: false,
		createDatabaseEncodings: [],
		supportsNullsLast: false,
		supportsExplain: true,
		usesInformationSchema: true,
		defaultPort: 3306,
		defaultUser: 'root',
		defaultDatabase: 'mysql',
		defaultName: 'local_mysql',
	},
	sqlite: {
		formatLanguage: 'sqlite',
		canCreateDatabase: false,
		createDatabaseEncodings: [],
		supportsNullsLast: true,
		supportsExplain: true,
		usesInformationSchema: false,
		defaultPort: 0,
		defaultUser: '',
		defaultDatabase: 'main',
		defaultName: 'local_sqlite',
	},
	mssql: {
		formatLanguage: 'transactsql',
		canCreateDatabase: true,
		createDatabaseEncodings: [],
		supportsNullsLast: false,
		supportsExplain: false,
		usesInformationSchema: true,
		defaultPort: 1433,
		defaultUser: 'sa',
		defaultDatabase: 'master',
		defaultName: 'local_mssql',
	},
	postgres: {
		formatLanguage: 'postgresql',
		canCreateDatabase: true,
		createDatabaseEncodings: POSTGRES_CREATE_DATABASE_ENCODINGS,
		supportsNullsLast: true,
		supportsExplain: true,
		usesInformationSchema: true,
		defaultPort: 5432,
		defaultUser: 'postgres',
		defaultDatabase: 'postgres',
		defaultName: 'local_pg',
	},
};

export function dialectCapabilities(
	databaseType: DatabaseType,
): DialectCapabilities {
	return DIALECT_BY_TYPE[databaseType];
}

export function engineDisplayName(databaseType: DatabaseType): string {
	if (databaseType === 'mysql') return 'MySQL';
	if (databaseType === 'sqlite') return 'SQLite';
	if (databaseType === 'mssql') return 'SQL Server';
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
	const fromColumns = table.columns
		.filter((column) => column.isPrimary)
		.map((column) => column.name);
	if (fromColumns.length > 0) return fromColumns;
	const pkIndex = (table.indexes ?? []).find((index) => index.isPrimary);
	if (!pkIndex) return [];
	return pkIndex.columns
		.split(',')
		.map((name) => name.trim())
		.filter((name) => name.length > 0);
}

function pkHashParts(
	databaseType: DatabaseType,
	columns: string[],
	columnPrefix?: string,
): string[] {
	return columns.map((column) => {
		const safeColumn = quoteSqlIdentifier(databaseType, column);
		const qualifiedColumn = columnPrefix
			? `${columnPrefix}.${safeColumn}`
			: safeColumn;
		if (databaseType === 'mssql') {
			return `coalesce(convert(varchar(max), ${qualifiedColumn}), '__querycastle_null__')`;
		}
		return `coalesce(cast(${qualifiedColumn} as char), '__querycastle_null__')`;
	});
}

export function buildPkHashExpression(
	databaseType: DatabaseType,
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
	columnPrefix?: string,
): string | null {
	if (databaseType !== 'mysql' && databaseType !== 'mssql') return null;
	const tableMeta = findExplorerTable(explorer, schema, table);
	if (!tableMeta) return null;
	const columns = primaryKeyColumns(tableMeta);
	if (columns.length === 0) return null;
	const parts = pkHashParts(databaseType, columns, columnPrefix);
	if (databaseType === 'mssql') {
		const payload =
			parts.length === 1
				? parts[0]!
				: `concat_ws(char(31), ${parts.join(', ')})`;
		return `convert(varchar(32), hashbytes('MD5', ${payload}), 2)`;
	}
	return `md5(concat_ws(char(31), ${parts.join(', ')}))`;
}

export function buildMysqlRowHashExpression(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
	columnPrefix?: string,
): string | null {
	return buildPkHashExpression('mysql', explorer, schema, table, columnPrefix);
}

export function rowSourceAlias(
	databaseType: DatabaseType,
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): string | undefined {
	if (databaseType !== 'mysql' && databaseType !== 'mssql') return undefined;
	return buildPkHashExpression(databaseType, explorer, schema, table, ROW_SOURCE_ALIAS)
		? ROW_SOURCE_ALIAS
		: undefined;
}

export function mysqlRowAlias(
	databaseType: DatabaseType,
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): string | undefined {
	return rowSourceAlias(databaseType, explorer, schema, table);
}

export function usesPkHashRowId(databaseType: DatabaseType): boolean {
	return databaseType === 'mysql' || databaseType === 'mssql';
}
