import { describe, expect, it } from 'vitest';
import { shopExplorer } from '$lib/utils/relation-fixtures';
import {
	buildFollowTabTitle,
	buildIncomingFollowSql,
	buildOutgoingFollowSql,
	createRelationHop,
	formatFollowValue,
	isFollowableValue,
	quoteLiteral,
} from '$lib/utils/relation-sql';
import { quoteSqlIdentifier } from '$lib/utils/sql';
import type { DatabaseForeignKey } from '$lib/rpc';

const customerFk: DatabaseForeignKey = {
	column: 'customer_id',
	referencedSchema: 'public',
	referencedTable: 'users',
	referencedColumn: 'id',
};

const explorer = shopExplorer();
const fromOrders = { schema: 'public', table: 'orders' };

describe('quoteSqlIdentifier', () => {
	it('quotes reserved words per dialect', () => {
		expect(quoteSqlIdentifier('postgres', 'user')).toBe('"user"');
		expect(quoteSqlIdentifier('sqlite', 'order')).toBe('"order"');
		expect(quoteSqlIdentifier('mysql', 'user')).toBe('`user`');
	});

	it('escapes quote characters inside identifiers', () => {
		expect(quoteSqlIdentifier('postgres', 'we"ird')).toBe('"we""ird"');
		expect(quoteSqlIdentifier('mysql', 'we`ird')).toBe('`we``ird`');
	});
});

describe('quoteLiteral', () => {
	it('quotes strings with embedded quotes', () => {
		expect(quoteLiteral('postgres', "O'Brien")).toBe("'O''Brien'");
		expect(quoteLiteral('sqlite', "O'Brien")).toBe("'O''Brien'");
		expect(quoteLiteral('mysql', "O'Brien")).toBe("'O''Brien'");
	});

	it('emits unquoted integers', () => {
		expect(quoteLiteral('postgres', 9)).toBe('9');
		expect(quoteLiteral('mysql', 9)).toBe('9');
		expect(quoteLiteral('sqlite', 9)).toBe('9');
	});

	it('emits dialect booleans', () => {
		expect(quoteLiteral('postgres', true)).toBe('TRUE');
		expect(quoteLiteral('mysql', false)).toBe('0');
		expect(quoteLiteral('sqlite', true)).toBe('1');
	});

	it('emits NULL for null', () => {
		expect(quoteLiteral('postgres', null)).toBe('NULL');
	});
});

describe('isFollowableValue', () => {
	it('rejects null and empty values in v1', () => {
		expect(isFollowableValue(null)).toBe(false);
		expect(isFollowableValue(undefined)).toBe(false);
		expect(isFollowableValue('')).toBe(false);
		expect(isFollowableValue('  ')).toBe(false);
		expect(isFollowableValue(0)).toBe(true);
		expect(isFollowableValue(9)).toBe(true);
	});
});

describe('buildOutgoingFollowSql', () => {
	it('does not build a hop for null values', () => {
		expect(
			buildOutgoingFollowSql({
				databaseType: 'postgres',
				explorer,
				fromTable: fromOrders,
				fk: customerFk,
				value: null,
			}),
		).toBeNull();
	});

	it('builds postgres SQL with ctid, quoting, and integer literal', () => {
		expect(
			buildOutgoingFollowSql({
				databaseType: 'postgres',
				explorer,
				fromTable: fromOrders,
				fk: customerFk,
				value: 9,
			}),
		).toBe(
			'select ctid::text as _querycastle_row_id, * from "public"."users" where "id" = 9 order by "id" asc nulls last limit 100;',
		);
	});

	it('builds sqlite SQL with rowid', () => {
		expect(
			buildOutgoingFollowSql({
				databaseType: 'sqlite',
				explorer,
				fromTable: fromOrders,
				fk: customerFk,
				value: 9,
			}),
		).toBe(
			'select cast(rowid as text) as _querycastle_row_id, * from "public"."users" where "id" = 9 order by "id" asc nulls last limit 100;',
		);
	});

	it('builds mysql SQL with row hash and backticks', () => {
		const sql = buildOutgoingFollowSql({
			databaseType: 'mysql',
			explorer,
			fromTable: fromOrders,
			fk: customerFk,
			value: 9,
		});
		expect(sql).toContain(' as _querycastle_row_id, _querycastle_src.* from `public`.`users` as _querycastle_src');
		expect(sql).toContain('where _querycastle_src.`id` = 9');
		expect(sql).toContain('order by `id` asc limit 100;');
		expect(sql).toContain('md5(');
	});

	it('quotes string values and reserved-word tables', () => {
		const fk: DatabaseForeignKey = {
			column: 'owner_id',
			referencedSchema: 'public',
			referencedTable: 'user',
			referencedColumn: 'name',
		};
		expect(
			buildOutgoingFollowSql({
				databaseType: 'postgres',
				explorer,
				fromTable: fromOrders,
				fk,
				value: "O'Brien",
			}),
		).toBe(
			`select ctid::text as _querycastle_row_id, * from "public"."user" where "name" = 'O''Brien' order by "id" asc nulls last limit 100;`,
		);
	});
});

describe('buildIncomingFollowSql', () => {
	it('does not build a hop for null parent values', () => {
		expect(
			buildIncomingFollowSql({
				databaseType: 'postgres',
				explorer,
				childTable: { schema: 'public', table: 'orders' },
				fk: customerFk,
				parentValue: null,
			}),
		).toBeNull();
	});

	it('filters the child table on the FK column', () => {
		expect(
			buildIncomingFollowSql({
				databaseType: 'postgres',
				explorer,
				childTable: { schema: 'public', table: 'orders' },
				fk: customerFk,
				parentValue: 9,
			}),
		).toBe(
			'select ctid::text as _querycastle_row_id, * from "public"."orders" where "customer_id" = 9 order by "id" asc nulls last limit 100;',
		);
	});
});

describe('buildFollowTabTitle', () => {
	it('keeps titles short and truncates long values', () => {
		const hop = createRelationHop({
			direction: 'outgoing',
			from: {
				schema: 'public',
				table: 'orders',
				column: 'customer_id',
				value: 9,
			},
			to: { schema: 'public', table: 'users', column: 'id' },
		});
		expect(buildFollowTabTitle(hop)).toBe('users id=9');
		expect(hop.label).toBe('orders.customer_id → users.id');
		expect(formatFollowValue('abcdefghijklmnopqrstuvwxyz')).toBe('abcdefghijklmnopqrstuvw…');
	});
});
