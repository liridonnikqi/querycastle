import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
import type { SchemaAction, TableAction } from '$lib/utils/workspace';
import { buildMysqlRowHashExpression } from '$lib/utils/editable-query';
import { quoteIdent, quoteSqlIdentifier } from '$lib/utils/sql';

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
	return (
		explorer?.schemas
			.find((item) => item.name === schema)
			?.tables.find((item) => item.name === table)?.columns[0]?.name ?? null
	);
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

	if (action === 'copy_name') return { kind: 'copy_name', text: `${schema}.${table}` };
	if (action === 'hide') {
		return { kind: 'error', message: `Hide is not implemented yet for ${schema}.${table}` };
	}
	if (action === 'import_file') {
		return { kind: 'error', message: 'Import from file is not implemented yet.' };
	}
	if (action === 'rename') return { kind: 'rename', value: table };

	if (action === 'drop') {
		return {
			kind: 'editor_sql',
			sql: `drop table ${safeSchema}.${safeTable};`,
			message: 'Drop statement inserted into editor. Review before running.',
		};
	}
	if (action === 'truncate') {
		return {
			kind: 'editor_sql',
			sql: `truncate table ${safeSchema}.${safeTable};`,
			message: 'Truncate statement inserted into editor. Review before running.',
		};
	}
	if (action === 'duplicate') {
		return {
			kind: 'editor_sql',
			sql: `create table ${safeSchema}.${safeTable}_copy as select * from ${safeSchema}.${safeTable};`,
			message: 'Duplicate statement inserted into editor. Review before running.',
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
			? databaseType === 'mysql'
				? ` order by ${quoteSqlIdentifier(databaseType, orderColumn)} asc`
				: ` order by ${quoteSqlIdentifier(databaseType, orderColumn)} asc nulls last`
			: '';

		if (databaseType === 'postgres') {
			query = `select ctid::text as _querycastle_ctid, * from ${safeSchema}.${safeTable}${orderByClause} limit 100;`;
		} else if (databaseType === 'sqlite') {
			query = `select cast(rowid as text) as _querycastle_ctid, * from ${safeSchema}.${safeTable}${orderByClause} limit 100;`;
		} else if (databaseType === 'mysql') {
			const rowHashWithAlias = buildMysqlRowHashExpression(
				explorer,
				schema,
				table,
				'_querycastle_src',
			);
			if (!rowHashWithAlias) {
				return {
					kind: 'error',
					message: 'Could not determine table columns for MySQL editing.',
				};
			}
			query = `select ${rowHashWithAlias} as _querycastle_ctid, _querycastle_src.* from ${safeSchema}.${safeTable} as _querycastle_src${orderByClause} limit 100;`;
		} else {
			query = `select * from ${safeSchema}.${safeTable}${orderByClause} limit 100;`;
		}

		title = `${table} [all]`;
		context =
			databaseType === 'postgres' || databaseType === 'sqlite' || databaseType === 'mysql'
				? { schema, table }
				: null;
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

	const query =
		databaseType === 'sqlite'
			? `select name as table_name, type\nfrom sqlite_master\nwhere type in ('table', 'view') and name not like 'sqlite_%'\norder by name;`
			: `select tablename as table_name\nfrom pg_catalog.pg_tables\nwhere schemaname = '${schema.replaceAll("'", "''")}'\norder by tablename;`;
	return { kind: 'run_query', query, title: `${schema} [tables]` };
}

export function buildRenameTableSql(params: {
	schema: string;
	table: string;
	nextName: string;
}): string {
	return `alter table ${quoteIdent(params.schema)}.${quoteIdent(params.table)} rename to ${quoteIdent(params.nextName)};`;
}

export function buildCreateDatabaseSql(name: string, encoding: string): string {
	return `create database ${quoteIdent(name)} encoding '${encoding}';`;
}
