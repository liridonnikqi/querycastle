import type { DatabaseColumn, DatabaseExplorer, DatabaseForeignKey, DatabaseTable } from '$lib/rpc';

function col(name: string, isPrimary = false, hasDefault = false): DatabaseColumn {
	return { name, dataType: 'int', notNull: true, isPrimary, hasDefault };
}

function table(
	schema: string,
	name: string,
	columns: DatabaseColumn[],
	foreignKeys: DatabaseForeignKey[] = [],
	kind: DatabaseTable['kind'] = 'table',
): DatabaseTable {
	return { schema, name, kind, columns, foreignKeys, indexes: [], triggers: [] };
}

export function shopExplorer(): DatabaseExplorer {
	return {
		database: 'shop',
		schemas: [
			{
				name: 'public',
				tables: [
					table('public', 'users', [col('id', true), col('email')], []),
					table(
						'public',
						'orders',
						[col('id', true), col('customer_id')],
						[
							{
								column: 'customer_id',
								referencedSchema: 'public',
								referencedTable: 'users',
								referencedColumn: 'id',
							},
						],
					),
					table(
						'public',
						'addresses',
						[col('id', true), col('user_id')],
						[
							{
								column: 'user_id',
								referencedSchema: 'public',
								referencedTable: 'users',
								referencedColumn: 'id',
							},
						],
					),
					table(
						'public',
						'employees',
						[col('id', true), col('manager_id')],
						[
							{
								column: 'manager_id',
								referencedSchema: 'public',
								referencedTable: 'employees',
								referencedColumn: 'id',
							},
						],
					),
					table('public', 'user', [col('id', true), col('name')]),
					table(
						'public',
						'notes',
						[col('org_id'), col('user_id')],
						[
							{
								column: 'org_id',
								referencedSchema: 'public',
								referencedTable: 'users',
								referencedColumn: 'org_id',
							},
							{
								column: 'user_id',
								referencedSchema: 'public',
								referencedTable: 'users',
								referencedColumn: 'id',
							},
						],
					),
					table('public', 'empty_view', [col('id', true)], [], 'view'),
				],
				routines: [
					{
						schema: 'public',
						name: 'add_user',
						kind: 'function',
						identityArgs: 'integer, text',
						language: 'plpgsql',
						returnType: 'integer',
						objectId: '1',
					},
				],
				sequences: [{ schema: 'public', name: 'users_id_seq', dataType: 'bigint' }],
			},
		],
	};
}
