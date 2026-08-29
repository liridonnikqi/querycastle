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
		expect(plan?.context).toEqual({ schema: 'public', table: 'users' });
	});
});
