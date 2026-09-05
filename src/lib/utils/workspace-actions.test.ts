import { describe, expect, it } from 'vitest';
import { shopExplorer } from '$lib/utils/relation-fixtures';
import { buildRenameTableSql, buildTableActionPlan } from '$lib/utils/workspace-actions';

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
		expect(plan.query).toContain('ctid::text as _querycastle_ctid');
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
	});
});
