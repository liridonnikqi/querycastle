import { describe, expect, it } from 'vitest';
import { ResultsGridSession } from '$lib/workspace/results-grid.svelte';

describe('pending grid undo', () => {
	it('restores a cell edit', () => {
		const grid = new ResultsGridSession();
		grid.setCellValue('1', 'email', 'new@x.com', 'old@x.com');
		expect(grid.pendingUpdates.get('1')?.email).toBe('new@x.com');
		expect(grid.canUndo).toBe(true);
		grid.undo();
		expect(grid.pendingUpdates.size).toBe(0);
		expect(grid.canUndo).toBe(false);
	});

	it('restores a deleted row', () => {
		const grid = new ResultsGridSession();
		grid.deleteRows(['1'], [{ _querycastle_row_id: '1', email: 'a@x.com' }]);
		expect(grid.pendingDeletes.has('1')).toBe(true);
		grid.undo();
		expect(grid.pendingDeletes.size).toBe(0);
		expect(grid.deletedSnapshots.size).toBe(0);
	});
});
