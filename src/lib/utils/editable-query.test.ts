import { describe, expect, it } from 'vitest';
import { tryBuildEditableQuery } from '$lib/utils/editable-query';
import { shopExplorer } from '$lib/utils/relation-fixtures';

const explorer = shopExplorer();

describe('tryBuildEditableQuery', () => {
	it('does not rewrite selects against views', () => {
		expect(
			tryBuildEditableQuery({
				sql: 'select * from public.empty_view',
				databaseType: 'postgres',
				explorer,
			}),
		).toBeNull();
	});

	it('rewrites simple table selects with ctid', () => {
		const plan = tryBuildEditableQuery({
			sql: 'select * from public.users',
			databaseType: 'postgres',
			explorer,
		});
		expect(plan?.sql).toContain('ctid::text as _querycastle_ctid');
		expect(plan?.sql).toContain('from "public"."users"');
		expect(plan?.context).toEqual({ schema: 'public', table: 'users' });
	});

	it('quotes capitalized table names so postgres keeps their case', () => {
		const capitalizedExplorer = {
			database: 'app',
			schemas: [
				{
					name: 'public',
					tables: [
						{
							schema: 'public',
							name: 'User',
							kind: 'table' as const,
							columns: [
								{
									name: 'Id',
									dataType: 'int',
									notNull: true,
									isPrimary: true,
								},
							],
							foreignKeys: [],
						},
					],
				},
			],
		};
		const plan = tryBuildEditableQuery({
			sql: 'SELECT * from public.User',
			databaseType: 'postgres',
			explorer: capitalizedExplorer,
		});
		expect(plan?.context).toEqual({ schema: 'public', table: 'User' });
		expect(plan?.sql).toContain('from "public"."User"');
	});
});
