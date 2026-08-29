import { describe, expect, it } from 'vitest';
import { shopExplorer } from '$lib/utils/relation-fixtures';
import {
	definitionTabTitle,
	explorerObjectCount,
	filterExplorer,
	isExplorerView,
	routineSignature,
	schemaFunctions,
	schemaProcedures,
	schemaTables,
	schemaViews,
} from '$lib/utils/schema-objects';

describe('schema objects', () => {
	const explorer = shopExplorer();

	it('formats routine signatures', () => {
		expect(routineSignature(schemaFunctions(explorer.schemas[0]!)[0]!)).toBe(
			'add_user(integer, text)',
		);
		expect(schemaProcedures(explorer.schemas[0]!)).toEqual([]);
	});

	it('titles definition tabs with arguments', () => {
		expect(definitionTabTitle('function', 'add_user', 'integer, text')).toBe(
			'add_user(integer, text)',
		);
		expect(definitionTabTitle('index', 'users_pkey')).toBe('users_pkey');
	});

	it('counts tables, routines, and sequences', () => {
		expect(explorerObjectCount(explorer)).toBe(9);
	});

	it('filters functions by name', () => {
		const filtered = filterExplorer(explorer, 'add_user');
		expect(filtered.schemas).toHaveLength(1);
		expect(filtered.schemas[0]?.tables).toHaveLength(0);
		expect(schemaFunctions(filtered.schemas[0]!)).toHaveLength(1);
	});

	it('detects views', () => {
		expect(isExplorerView(explorer, 'public', 'empty_view')).toBe(true);
		expect(isExplorerView(explorer, 'public', 'users')).toBe(false);
		expect(schemaViews(explorer.schemas[0]!).map((item) => item.name)).toEqual([
			'empty_view',
		]);
		expect(schemaTables(explorer.schemas[0]!).some((item) => item.name === 'empty_view')).toBe(
			false,
		);
	});
});
