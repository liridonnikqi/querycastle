import { describe, expect, it } from 'vitest';
import { buildExplainSql, displayDatabaseName } from './dialect';

describe('displayDatabaseName', () => {
	it('returns basename for sqlite file paths', () => {
		expect(
			displayDatabaseName('C:\\Users\\dev\\data\\qc_audit_test.db', 'sqlite'),
		).toBe('qc_audit_test.db');
		expect(displayDatabaseName('/var/lib/app/main.sqlite3', 'sqlite')).toBe(
			'main.sqlite3',
		);
	});

	it('returns database name directly for server databases', () => {
		expect(displayDatabaseName('postgres', 'postgres')).toBe('postgres');
		expect(displayDatabaseName('', 'sqlite')).toBe('');
	});
});

describe('buildExplainSql', () => {
	it('builds dialect-appropriate explain queries', () => {
		expect(buildExplainSql('sqlite', 'SELECT * FROM users;')).toBe(
			'EXPLAIN QUERY PLAN SELECT * FROM users',
		);
		expect(buildExplainSql('mysql', 'SELECT * FROM users;')).toBe(
			'EXPLAIN SELECT * FROM users',
		);
		expect(buildExplainSql('postgres', 'SELECT * FROM users;')).toBe(
			'EXPLAIN (COSTS true, VERBOSE false, FORMAT text) SELECT * FROM users',
		);
	});

	it('preserves queries that already start with EXPLAIN', () => {
		expect(buildExplainSql('sqlite', 'EXPLAIN QUERY PLAN SELECT 1;')).toBe(
			'EXPLAIN QUERY PLAN SELECT 1',
		);
	});
});
