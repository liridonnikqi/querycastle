import { describe, expect, it } from 'vitest';
import { shopExplorer } from '$lib/utils/relation-fixtures';
import {
	resolveIncomingRelations,
	resolveOutgoingRelations,
} from '$lib/utils/relation-resolve';

const explorer = shopExplorer();
const ordersCtx = { schema: 'public', table: 'orders' };
const usersCtx = { schema: 'public', table: 'users' };
const employeesCtx = { schema: 'public', table: 'employees' };

describe('resolveOutgoingRelations', () => {
	it('returns the FK on orders.customer_id', () => {
		expect(resolveOutgoingRelations(explorer, ordersCtx, 'customer_id')).toEqual([
			{
				column: 'customer_id',
				referencedSchema: 'public',
				referencedTable: 'users',
				referencedColumn: 'id',
			},
		]);
	});

	it('returns empty for missing explorer, hidden row id, and views without FKs', () => {
		expect(resolveOutgoingRelations(null, ordersCtx, 'customer_id')).toEqual([]);
		expect(resolveOutgoingRelations(explorer, ordersCtx, '_querycastle_row_id')).toEqual([]);
		expect(
			resolveOutgoingRelations(
				explorer,
				{ schema: 'public', table: 'empty_view' },
				'id',
			),
		).toEqual([]);
	});

	it('skips composite-looking FKs', () => {
		expect(
			resolveOutgoingRelations(
				explorer,
				{ schema: 'public', table: 'notes' },
				'user_id',
			),
		).toEqual([]);
		expect(
			resolveOutgoingRelations(
				explorer,
				{ schema: 'public', table: 'notes' },
				'org_id',
			),
		).toEqual([]);
	});
});

describe('resolveIncomingRelations', () => {
	it('lists orders and addresses from a users row', () => {
		const incoming = resolveIncomingRelations(explorer, usersCtx);
		expect(incoming.map((item) => item.table).sort()).toEqual(['addresses', 'orders']);
	});

	it('includes self-referential FKs', () => {
		const incoming = resolveIncomingRelations(explorer, employeesCtx);
		expect(incoming).toEqual([
			{
				schema: 'public',
				table: 'employees',
				fk: {
					column: 'manager_id',
					referencedSchema: 'public',
					referencedTable: 'employees',
					referencedColumn: 'id',
				},
			},
		]);
		expect(resolveOutgoingRelations(explorer, employeesCtx, 'manager_id')).toHaveLength(1);
	});

	it('returns empty when explorer is missing', () => {
		expect(resolveIncomingRelations(null, usersCtx)).toEqual([]);
	});
});
