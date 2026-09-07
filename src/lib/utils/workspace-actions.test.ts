import { describe, expect, it } from 'vitest';
import { shopExplorer } from '$lib/utils/relation-fixtures';
import {
	buildPkHashExpression,
	dialectCapabilities,
	primaryKeyColumns,
} from '$lib/utils/dialect';
import { canEditTable } from '$lib/utils/table-select';
import {
	buildCreateDatabaseSql,
	buildRenameTableSql,
	buildTableActionPlan,
} from '$lib/utils/workspace-actions';
import type { DatabaseExplorer } from '$lib/rpc';

const explorer = shopExplorer();

describe('buildTableActionPlan', () => {
	it('loads view data without table row ids', () => {
		const plan = buildTableActionPlan({
			action: 'view_data',
			databaseType: 'postgres',
			explorer,
			schema: 'public',
			table: 'empty_view',
		});
		expect(plan).toMatchObject({
			kind: 'run_query',
			title: 'empty_view',
			context: null,
		});
		if (plan.kind !== 'run_query') return;
		expect(plan.query).toBe(
			'select * from "public"."empty_view" order by "id" asc nulls last limit 100;',
		);
		expect(plan.query).not.toContain('ctid');
	});

	it('still uses ctid when viewing a table', () => {
		const plan = buildTableActionPlan({
			action: 'view_data',
			databaseType: 'postgres',
			explorer,
			schema: 'public',
			table: 'users',
		});
		expect(plan.kind).toBe('run_query');
		if (plan.kind !== 'run_query') return;
		expect(plan.query).toContain('ctid::text as _querycastle_row_id');
		expect(plan.context).toEqual({ schema: 'public', table: 'users' });
	});

	it('drops views with drop view', () => {
		const plan = buildTableActionPlan({
			action: 'drop',
			databaseType: 'postgres',
			explorer,
			schema: 'public',
			table: 'empty_view',
		});
		expect(plan.kind).toBe('run_query');
		if (plan.kind !== 'run_query') return;
		expect(plan.query).toBe('drop view "public"."empty_view" cascade;');
	});

	it('quotes the duplicate-table suffix inside the identifier', () => {
		const plan = buildTableActionPlan({
			action: 'duplicate',
			databaseType: 'postgres',
			explorer,
			schema: 'my schema',
			table: 'my table',
		});
		expect(plan.kind).toBe('run_query');
		if (plan.kind !== 'run_query') return;
		expect(plan.query).toBe(
			'create table "my schema"."my table_copy" as select * from "my schema"."my table";',
		);
	});

	it('quotes duplicate tables with backticks for mysql', () => {
		const plan = buildTableActionPlan({
			action: 'duplicate',
			databaseType: 'mysql',
			explorer,
			schema: 'shop',
			table: 'orders',
		});
		expect(plan.kind).toBe('run_query');
		if (plan.kind !== 'run_query') return;
		expect(plan.query).toBe(
			'create table `shop`.`orders_copy` as select * from `shop`.`orders`;',
		);
	});

	it('quotes rename identifiers per dialect', () => {
		expect(
			buildRenameTableSql({
				databaseType: 'postgres',
				schema: 'public',
				table: 'users',
				nextName: 'members',
			}),
		).toBe('alter table "public"."users" rename to "members";');
		expect(
			buildRenameTableSql({
				databaseType: 'mysql',
				schema: 'shop',
				table: 'orders',
				nextName: 'orders_v2',
			}),
		).toBe('alter table `shop`.`orders` rename to `orders_v2`;');
		expect(
			buildRenameTableSql({
				databaseType: 'mssql',
				schema: 'dbo',
				table: 'users',
				nextName: 'members',
			}),
		).toBe("exec sp_rename N'dbo.users', N'members', N'OBJECT';");
	});

	it('treats a SQL Server PK index as editable when column flags are missing', () => {
		const explorer: DatabaseExplorer = {
			database: 'testing',
			schemas: [
				{
					name: 'dbo',
					tables: [
						{
							schema: 'dbo',
							name: 'Persons',
							kind: 'table',
							columns: [
								{
									name: 'PersonID',
									dataType: 'int',
									notNull: true,
									isPrimary: false,
								},
								{
									name: 'LastName',
									dataType: 'varchar',
									notNull: true,
									isPrimary: false,
								},
							],
							foreignKeys: [],
							indexes: [
								{
									name: 'PK_Persons',
									columns: 'PersonID',
									unique: true,
									isPrimary: true,
									definition: null,
								},
							],
							triggers: [],
						},
					],
				},
			],
		};
		expect(primaryKeyColumns(explorer.schemas[0]!.tables[0]!)).toEqual(['PersonID']);
		expect(canEditTable('mssql', explorer, 'dbo', 'Persons')).toBe(true);
		const hash = buildPkHashExpression('mssql', explorer, 'dbo', 'Persons');
		expect(hash).toContain('hashbytes');
		expect(hash).not.toContain('concat_ws');
	});

	it('builds create database SQL per dialect', () => {
		expect(buildCreateDatabaseSql('postgres', 'analytics', 'UTF8')).toBe(
			'create database "analytics" encoding \'UTF8\';',
		);
		expect(buildCreateDatabaseSql('mssql', 'analytics')).toBe(
			'create database [analytics];',
		);
		expect(dialectCapabilities('mssql').canCreateDatabase).toBe(true);
		expect(dialectCapabilities('mssql').createDatabaseEncodings).toEqual([]);
		expect(dialectCapabilities('postgres').createDatabaseEncodings).toContain('UTF8');
	});

	it('duplicates SQL Server tables with SELECT INTO', () => {
		const plan = buildTableActionPlan({
			action: 'duplicate',
			databaseType: 'mssql',
			explorer,
			schema: 'dbo',
			table: 'users',
		});
		expect(plan.kind).toBe('run_query');
		if (plan.kind !== 'run_query') return;
		expect(plan.query).toBe('select * into [dbo].[users_copy] from [dbo].[users];');
	});
});
