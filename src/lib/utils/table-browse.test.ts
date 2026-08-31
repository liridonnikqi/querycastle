import { describe, expect, it } from 'vitest';
import { shopExplorer } from '$lib/utils/relation-fixtures';
import {
	buildLimitClause,
	buildTableBrowseSql,
	buildTableCountSql,
	extractWhereClause,
	nextSortState,
	parseCountResult,
	totalPages,
} from '$lib/utils/table-browse';

describe('table browse sql', () => {
	it('extracts a follow-the-row where clause', () => {
		expect(
			extractWhereClause(
				'select ctid::text as _querycastle_ctid, * from "public"."users" where "id" = 9 order by "id" asc nulls last limit 100;',
			),
		).toBe('"id" = 9');
	});

	it('pages a postgres table browse', () => {
		expect(
			buildTableBrowseSql({
				databaseType: 'postgres',
				explorer: shopExplorer(),
				schema: 'public',
				table: 'users',
				baseWhere: '"id" = 9',
				filters: [{ column: 'email', value: 'ada' }],
				sort: { column: 'email', dir: 'desc' },
				limit: 50,
				offset: 50,
			}),
		).toBe(
			`select ctid::text as _querycastle_ctid, * from "public"."users" where "id" = 9 and cast("email" as text) ilike '%ada%' order by "email" desc nulls last limit 50 offset 50;`,
		);
	});

	it('builds a count query without row ids', () => {
		expect(
			buildTableCountSql({
				databaseType: 'postgres',
				schema: 'public',
				table: 'users',
				baseWhere: '"id" = 9',
				filters: [],
			}),
		).toBe('select count(*) as count from "public"."users" where "id" = 9;');
	});

	it('cycles sort and pages', () => {
		expect(nextSortState(null, 'email')).toEqual({ column: 'email', dir: 'asc' });
		expect(nextSortState({ column: 'email', dir: 'asc' }, 'email')).toEqual({
			column: 'email',
			dir: 'desc',
		});
		expect(nextSortState({ column: 'email', dir: 'desc' }, 'email')).toBeNull();
		expect(totalPages(160, 50)).toBe(4);
		expect(buildLimitClause(50, 0)).toBe(' limit 50');
		expect(parseCountResult([{ count: '160' }])).toBe(160);
	});
});
