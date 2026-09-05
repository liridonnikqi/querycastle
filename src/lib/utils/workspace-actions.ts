import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
import { findExplorerTable, isExplorerView } from '$lib/utils/schema-objects';
import { quoteIdent, quoteSqlIdentifier } from '$lib/utils/sql';
import { buildOrderByClause, buildTableSelect, canEditTable } from '$lib/utils/table-select';
import type { SchemaAction, TableAction } from '$lib/utils/workspace';

type QueryContext = { schema: string; table: string } | null;

export type TableActionPlan =
	| { kind: 'copy_name'; text: string }
	| { kind: 'error'; message: string }
	| { kind: 'rename'; value: string }
	| { kind: 'editor_sql'; sql: string; message: string }
	| { kind: 'editor_sql_clear_error'; sql: string }
	| { kind: 'run_query'; query: string; title: string; context: QueryContext };

export type SchemaActionPlan =
	| { kind: 'copy'; text: string }
	| { kind: 'run_query'; query: string; title: string };

function firstOrderColumn(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): string | null {
	return findExplorerTable(explorer, schema, table)?.columns[0]?.name ?? null;
}

export function buildTableActionPlan(params: {
	action: TableAction;
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
	schema: string;
	table: string;
}): TableActionPlan {
	const { action, databaseType, explorer, schema, table } = params;
	const safeSchema = quoteSqlIdentifier(databaseType, schema);
	const safeTable = quoteSqlIdentifier(databaseType, table);
	const viewingView = isExplorerView(explorer, schema, table);

	if (action === 'copy_name') return { kind: 'copy_name', text: `${schema}.${table}` };
	if (action === 'hide') {
		return { kind: 'error', message: `Hide is not implemented yet for ${schema}.${table}` };
	}
	if (action === 'import_file') {
		return { kind: 'error', message: 'Import from file is not implemented yet.' };
	}
	if (action === 'rename') return { kind: 'rename', value: table };

	if (action === 'drop') {
		const cascade = databaseType === 'postgres' ? ' cascade' : '';
		const objectType = viewingView ? 'view' : 'table';
		return {
			kind: 'run_query',
			query: `drop ${objectType} ${safeSchema}.${safeTable}${cascade};`,
			title: `${table} [drop]`,
			context: null,
		};
	}
	if (action === 'truncate') {
		if (viewingView) {
			return { kind: 'error', message: `Cannot truncate view ${schema}.${table}.` };
		}
		return {
			kind: 'run_query',
			query: `truncate table ${safeSchema}.${safeTable};`,
			title: `${table} [truncate]`,
			context: null,
		};
	}
	if (action === 'duplicate') {
		if (viewingView) {
			return { kind: 'error', message: `Cannot duplicate view ${schema}.${table}.` };
		}
		// Quote the suffixed name as a whole: appending _copy outside the
		// quotes produced invalid SQL for names needing quoting
		// (e.g. "my table"_copy).
		const copyTable = quoteSqlIdentifier(databaseType, `${table}_copy`);
		return {
			kind: 'run_query',
			query: `create table ${safeSchema}.${copyTable} as select * from ${safeSchema}.${safeTable};`,
			title: `${table} [duplicate]`,
			context: null,
		};
	}
	if (action === 'sql_create') {
		const sql =
			databaseType === 'sqlite'
				? `-- Table definition helper\npragma table_info('${table.replaceAll("'", "''")}');`
				: `-- Table definition helper\nselect column_name, data_type, is_nullable\nfrom information_schema.columns\nwhere table_schema = '${schema.replaceAll("'", "''")}'\n  and table_name = '${table.replaceAll("'", "''")}'\norder by ordinal_position;`;
		return { kind: 'editor_sql_clear_error', sql };
	}

	let query = '';
	let title = `${table}`;
	let context: QueryContext = null;

	if (action === 'view_data') {
		const orderColumn = firstOrderColumn(explorer, schema, table);
		const orderByClause = orderColumn
			? buildOrderByClause(databaseType, { column: orderColumn, dir: 'asc' })
			: '';
		const editable = canEditTable(databaseType, explorer, schema, table);
		const built = buildTableSelect({
			databaseType,
			explorer,
			schema,
			table,
			orderClause: orderByClause,
			limit: 100,
			includeRowId: editable,
		});
		if (!built) {
			return { kind: 'error', message: `Could not build a query for ${schema}.${table}.` };
		}
		query = built;
		if (viewingView) {
			title = `${table}`;
			context = null;
		} else {
			title = `${table} [all]`;
			context = editable ? { schema, table } : null;
		}
	}

	if (action === 'view_structure') {
		query =
			databaseType === 'sqlite'
				? `pragma table_info('${table.replaceAll("'", "''")}');`
				: `select column_name, data_type, is_nullable from information_schema.columns where table_schema = '${schema.replaceAll("'", "''")}' and table_name = '${table.replaceAll("'", "''")}' order by ordinal_position;`;
		title = `${table} [structure]`;
	}

	if (action === 'export_file') {
		query = `select * from ${safeSchema}.${safeTable} limit 1000;`;
		title = `${table} [export]`;
	}

	return { kind: 'run_query', query, title, context };
}

export function buildSchemaActionPlan(params: {
	action: SchemaAction;
	databaseType: DatabaseType;
	schema: string;
}): SchemaActionPlan {
	const { action, databaseType, schema } = params;
	if (action === 'copy_name') return { kind: 'copy', text: schema };
	if (action === 'copy_quoted_name') return { kind: 'copy', text: quoteIdent(schema) };

	const escaped = schema.replaceAll("'", "''");
	const query =
		databaseType === 'sqlite'
			? `select name as table_name, type\nfrom sqlite_master\nwhere type in ('table', 'view') and name not like 'sqlite_%'\norder by name;`
			: databaseType === 'mysql'
				? `select table_name\nfrom information_schema.tables\nwhere table_schema = '${escaped}'\norder by table_name;`
				: `select tablename as table_name\nfrom pg_catalog.pg_tables\nwhere schemaname = '${escaped}'\norder by tablename;`;
	return { kind: 'run_query', query, title: `${schema} [tables]` };
}

export function buildRenameTableSql(params: {
	databaseType: DatabaseType;
	schema: string;
	table: string;
	nextName: string;
}): string {
	const quote = (name: string) => quoteSqlIdentifier(params.databaseType, name);
	return `alter table ${quote(params.schema)}.${quote(params.table)} rename to ${quote(params.nextName)};`;
}

export function buildCreateDatabaseSql(name: string, encoding: string): string {
	return `create database ${quoteIdent(name)} encoding '${encoding}';`;
}
