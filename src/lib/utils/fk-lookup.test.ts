import { describe, expect, it } from 'vitest';
import { buildFkLookupSql, rowsToFkOptions } from '$lib/utils/fk-lookup';

const fk = {
	column: 'customer_id',
	referencedSchema: 'public',
	referencedTable: 'users',
	referencedColumn: 'id',
};

describe('fk lookup', () => {
	it('builds a searchable postgres lookup query', () => {
		expect(
			buildFkLookupSql({
				databaseType: 'postgres',
				fk,
				labelColumns: ['email'],
				search: 'ada',
			}),
		).toBe(
			`select "id", "email" from "public"."users" where cast("id" as text) ilike '%ada%' or cast("email" as text) ilike '%ada%' order by "email" asc nulls last limit 200;`,
		);
	});

	it('maps rows to human labels', () => {
		expect(
			rowsToFkOptions(
				{
					columns: ['id', 'email'],
					rows: [
						{ id: 1, email: 'ada@example.com' },
						{ id: 2, email: null },
					],
					rowCount: 2,
					durationMs: 1,
					truncated: false,
				},
				'id',
				['email'],
			),
		).toEqual([
			{ id: 1, label: 'ada@example.com' },
			{ id: 2, label: '2' },
		]);
	});
});
