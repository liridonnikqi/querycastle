import { describe, expect, it } from 'vitest';
import { HIDDEN_ROW_ID_COLUMN } from '$lib/utils/dialect';
import {
	isCommandQueryResult,
	isGridEditable,
	queryOutcomeMessage,
	visibleGridColumns,
} from '$lib/utils/result-grid';
import type { DatabaseExplorer } from '$lib/rpc';

const personsExplorer: DatabaseExplorer = {
	database: 'testing',
	schemas: [
		{
			name: 'dbo',
			tables: [
				{
					schema: 'dbo',
					name: 'Persons',
					kind: 'table',
					columns: [
						{ name: 'PersonID', dataType: 'int', notNull: true, isPrimary: true },
						{ name: 'LastName', dataType: 'varchar', notNull: true, isPrimary: false },
					],
					foreignKeys: [],
					indexes: [],
					triggers: [],
				},
			],
		},
	],
};

describe('visibleGridColumns', () => {
	it('hides the row id and falls back to explorer columns for empty results', () => {
		expect(
			visibleGridColumns([HIDDEN_ROW_ID_COLUMN, 'PersonID', 'LastName'], []),
		).toEqual(['PersonID', 'LastName']);
		expect(visibleGridColumns([], ['PersonID', 'LastName'])).toEqual([
			'PersonID',
			'LastName',
		]);
	});
});

describe('isGridEditable', () => {
	it('allows insert on an empty editable table without a row-id column', () => {
		expect(
			isGridEditable({
				databaseType: 'mssql',
				explorer: personsExplorer,
				context: { schema: 'dbo', table: 'Persons' },
				resultColumns: [],
				rowCount: 0,
				visibleColumns: ['PersonID', 'LastName'],
			}),
		).toBe(true);
	});

	it('does not treat a command result as an editable grid', () => {
		expect(
			isGridEditable({
				databaseType: 'mssql',
				explorer: personsExplorer,
				context: null,
				resultColumns: [],
				rowCount: 0,
				visibleColumns: [],
			}),
		).toBe(false);
	});

	it('blocks edits on a read-only connection', () => {
		expect(
			isGridEditable({
				databaseType: 'mssql',
				explorer: personsExplorer,
				context: { schema: 'dbo', table: 'Persons' },
				resultColumns: [],
				rowCount: 0,
				visibleColumns: ['PersonID', 'LastName'],
				readOnly: true,
			}),
		).toBe(false);
	});
});

describe('isCommandQueryResult', () => {
	it('treats a completed non-select as a command, not an empty sheet', () => {
		expect(
			isCommandQueryResult({
				loading: false,
				sqlError: '',
				resultColumns: [],
				visibleColumns: [],
				lastRunSql: 'create table t (id int)',
			}),
		).toBe(true);
		expect(
			isCommandQueryResult({
				loading: false,
				sqlError: '',
				resultColumns: [],
				visibleColumns: ['PersonID'],
				lastRunSql: 'select * from Persons',
			}),
		).toBe(false);
	});
});

describe('queryOutcomeMessage', () => {
	it('reports rows for result sets and affected rows for commands', () => {
		expect(
			queryOutcomeMessage({
				durationMs: 12,
				rowCount: 3,
				truncated: false,
				columns: ['id'],
			}),
		).toBe('Last query executed successfully in 12ms and returned 3 rows.');
		expect(
			queryOutcomeMessage({
				durationMs: 30,
				rowCount: 0,
				truncated: false,
				columns: [],
			}),
		).toBe('Command completed successfully in 30ms.');
		expect(
			queryOutcomeMessage({
				durationMs: 8,
				rowCount: 1,
				truncated: false,
				columns: [],
			}),
		).toBe('Command completed successfully in 8ms (1 row affected).');
	});
});
