import { describe, expect, it } from 'vitest';
import {
	buildPendingChangeCards,
	buildPendingSqlPreview,
	buildRowInspectSql,
	diffText,
	formatApplyResultMessage,
	pendingChangeCount,
} from '$lib/utils/pending-changes';

describe('pending changes', () => {
	it('summarizes apply results for the user', () => {
		expect(
			formatApplyResultMessage({ inserted: 1, updated: 0, deleted: 0 }),
		).toBe('Added 1 row');
		expect(
			formatApplyResultMessage({ inserted: 2, updated: 1, deleted: 3 }),
		).toBe('Added 2 rows. Updated 1 row. Deleted 3 rows');
		expect(
			formatApplyResultMessage({ inserted: 0, updated: 0, deleted: 0 }),
		).toBe('Changes saved');
	});

	it('counts each edited field, insert, and delete', () => {
		expect(
			pendingChangeCount({
				updates: new Map([['1', { name: 'Ada', email: 'a@x.com' }]]),
				inserts: [{ id: 'n1', values: { email: 'b@x.com' } }],
				deletes: new Set(['2']),
			}),
		).toBe(4);
	});

	it('builds visual diff cards for updates', () => {
		const cards = buildPendingChangeCards({
			schema: 'public',
			table: 'users',
			rows: [{ _querycastle_ctid: '1', email: 'old@x.com' }],
			updates: new Map([['1', { email: 'new@x.com' }]]),
			inserts: [],
			deletes: new Set(),
		});
		expect(cards).toHaveLength(1);
		expect(cards[0]).toMatchObject({
			badge: 'U',
			title: 'public.users > row 1 > email',
			before: 'old@x.com',
			after: 'new@x.com',
		});
	});

	it('treats a cleared cell as empty, not the word Empty', () => {
		const cards = buildPendingChangeCards({
			schema: 'public',
			table: 'users',
			rows: [{ _querycastle_ctid: '1', email: 'old@x.com' }],
			updates: new Map([['1', { email: null }]]),
			inserts: [],
			deletes: new Set(),
		});
		expect(cards[0]?.before).toBe('old@x.com');
		expect(cards[0]?.after).toBe('');
	});

	it('diffs changed characters', () => {
		expect(diffText('hello', 'hallo')).toEqual([
			{ kind: 'eq', text: 'h' },
			{ kind: 'del', text: 'e' },
			{ kind: 'add', text: 'a' },
			{ kind: 'eq', text: 'llo' },
		]);
	});

	it('previews postgres update sql with a primary key', () => {
		expect(
			buildPendingSqlPreview({
				databaseType: 'postgres',
				schema: 'public',
				table: 'users',
				updates: [{ ctid: '(0,1)', values: { email: "o'brien@x.com" } }],
				deletes: [],
				inserts: [],
				rows: [{ _querycastle_ctid: '(0,1)', id: '2', email: 'old@x.com' }],
				pkColumns: ['id'],
			}),
		).toBe(
			`update "public"."users" set "email" = 'o''brien@x.com' where "id" = 2;`,
		);
	});

	it('previews a delete using the primary key', () => {
		expect(
			buildPendingSqlPreview({
				databaseType: 'postgres',
				schema: 'demo',
				table: 'users',
				updates: [],
				deletes: ['(0,1)'],
				inserts: [],
				rows: [{ _querycastle_ctid: '(0,1)', id: 2, email: 'a@x.com' }],
				pkColumns: ['id'],
			}),
		).toBe(`delete from "demo"."users" where "id" = 2;`);
	});

	it('inspects a postgres row by primary key', () => {
		expect(
			buildRowInspectSql({
				databaseType: 'postgres',
				schema: 'demo',
				table: 'users',
				ctid: '(0,2)',
				row: { _querycastle_ctid: '(0,2)', id: '2' },
				pkColumns: ['id'],
			}),
		).toBe(`select * from "demo"."users" where "id" = 2;`);
	});
});
