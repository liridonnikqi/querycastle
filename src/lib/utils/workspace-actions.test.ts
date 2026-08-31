import { describe, expect, it } from 'vitest';
import { shopExplorer } from '$lib/utils/relation-fixtures';
import { buildTableActionPlan } from '$lib/utils/workspace-actions';

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
});
