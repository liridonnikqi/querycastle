import { describe, expect, it } from 'vitest';
import { parseRelationTrail } from '$lib/utils/persistence';

describe('parseRelationTrail', () => {
	it('returns empty for missing or invalid trails so old tabs still load', () => {
		expect(parseRelationTrail(undefined)).toEqual([]);
		expect(parseRelationTrail(null)).toEqual([]);
		expect(parseRelationTrail('nope')).toEqual([]);
	});

	it('restores a valid hop', () => {
		expect(
			parseRelationTrail([
				{
					direction: 'outgoing',
					from: { schema: 'public', table: 'orders', column: 'customer_id', value: 9 },
					to: { schema: 'public', table: 'users', column: 'id' },
					label: 'orders.customer_id → users.id',
				},
			]),
		).toEqual([
			{
				direction: 'outgoing',
				from: { schema: 'public', table: 'orders', column: 'customer_id', value: 9 },
				to: { schema: 'public', table: 'users', column: 'id' },
				label: 'orders.customer_id → users.id',
			},
		]);
	});
});
