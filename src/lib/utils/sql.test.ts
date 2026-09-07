import { describe, expect, it } from 'vitest';
import {
	commandSuccessMessage,
	quoteCatalogIdentifiersInSql,
	quoteSqlIdentifier,
	quoteSqlIdentifierIfNeeded,
	sqlCommandVerb,
	unquoteIdent,
} from '$lib/utils/sql';

describe('commandSuccessMessage', () => {
	it('toasts DML and DDL verbs and ignores selects', () => {
		expect(sqlCommandVerb('CREATE TABLE t (id int)')).toBe('create');
		expect(commandSuccessMessage('create table t (id int)')).toBe('Create succeeded');
		expect(commandSuccessMessage('  DROP DATABASE demo')).toBe('Drop succeeded');
		expect(commandSuccessMessage('insert into t values (1)')).toBe('Insert succeeded');
		expect(commandSuccessMessage('truncate table t')).toBe('Table truncated');
		expect(commandSuccessMessage('select 1')).toBeNull();
	});
});

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

	it('uses brackets for SQL Server and escapes closing brackets', () => {
		expect(quoteSqlIdentifier('mssql', 'users')).toBe('[users]');
		expect(quoteSqlIdentifier('mssql', 'my]table')).toBe('[my]]table]');
		expect(unquoteIdent('[my]]table]')).toBe('my]table');
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
