import { describe, expect, it } from 'vitest';
import {
	buildPendingChangeCards,
	buildPendingSqlPreview,
	diffText,
	pendingChangeCount,
} from '$lib/utils/pending-changes';

describe('pending changes', () => {
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

	it('previews postgres update sql', () => {
		expect(
			buildPendingSqlPreview({
				databaseType: 'postgres',
				schema: 'public',
				table: 'users',
				updates: [{ ctid: '(0,1)', values: { email: "o'brien@x.com" } }],
				deletes: [],
				inserts: [],
			}),
		).toBe(
			`update "public"."users" set "email" = 'o''brien@x.com' where ctid = '(0,1)'::tid;`,
		);
	});
});
