import { describe, expect, it } from 'vitest';
import {
	quoteCatalogIdentifiersInSql,
	quoteSqlIdentifierIfNeeded,
	unquoteIdent,
} from '$lib/utils/sql';

describe('unquoteIdent', () => {
	it('preserves mixed-case unquoted names', () => {
		expect(unquoteIdent('User')).toBe('User');
		expect(unquoteIdent('"User"')).toBe('User');
	});
});

describe('quoteSqlIdentifierIfNeeded', () => {
	it('quotes postgres names that would be folded', () => {
		expect(quoteSqlIdentifierIfNeeded('postgres', 'users')).toBe('users');
		expect(quoteSqlIdentifierIfNeeded('postgres', 'User')).toBe('"User"');
	});
});

describe('quoteCatalogIdentifiersInSql', () => {
	it('quotes mixed-case table names postgres would otherwise fold', () => {
		expect(
			quoteCatalogIdentifiersInSql(
				'SELECT * from public.User',
				'postgres',
				['public', 'User', 'Id'],
			),
		).toBe('SELECT * from public."User"');
	});

	it('does not rewrite lowercase names or SQL keywords', () => {
		expect(
			quoteCatalogIdentifiersInSql(
				'SELECT * FROM public.users ORDER BY id',
				'postgres',
				['public', 'users', 'id'],
			),
		).toBe('SELECT * FROM public.users ORDER BY id');
	});

	it('leaves string literals and comments alone', () => {
		expect(
			quoteCatalogIdentifiersInSql(
				"SELECT 'User' -- User\nFROM public.User",
				'postgres',
				['User'],
			),
		).toBe("SELECT 'User' -- User\nFROM public.\"User\"");
	});
});
