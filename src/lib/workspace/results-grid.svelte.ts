import type { FkOption } from '$lib/utils/fk-lookup';
import { HIDDEN_ROW_ID_COLUMN } from '$lib/utils/dialect';
import type { PageSize } from '$lib/utils/table-browse';

export type PendingInsertRow = { id: string; values: Record<string, unknown> };

export function rowIdOf(row: Record<string, unknown>): string {
	return String(row[HIDDEN_ROW_ID_COLUMN] ?? '');
}

export function isHiddenRowIdColumn(column: string): boolean {
	return column === HIDDEN_ROW_ID_COLUMN;
}

/** Pending edits, selection, FK cache, and browse chrome for the results grid. */
export class ResultsGridSession {
	selectedRows = $state(new Set<string>());
	activeRowId = $state<string | null>(null);
	pendingUpdates = $state(new Map<string, Record<string, unknown>>());
	pendingDeletes = $state(new Set<string>());
	pendingInserts = $state<PendingInsertRow[]>([]);
	deletedSnapshots = $state(new Map<string, Record<string, unknown>>());
	fkOptionCache = $state(new Map<string, FkOption[]>());
	fkLoadingKeys = $state(new Set<string>());
	pageSize = $state<PageSize>(100);
	totalRowCount = $state(0);
	columnFilters = $state<Record<string, string>>({});
	showFilterRow = $state(false);
	baseWhere = $state('');

	clearDrafts() {
		this.pendingUpdates = new Map();
		this.pendingDeletes = new Set();
		this.pendingInserts = [];
		this.deletedSnapshots = new Map();
		this.selectedRows = new Set();
		this.activeRowId = null;
	}

	resetBrowse() {
		this.pageSize = 100;
		this.totalRowCount = 0;
		this.columnFilters = {};
		this.showFilterRow = false;
		this.baseWhere = '';
		this.fkOptionCache = new Map();
		this.fkLoadingKeys = new Set();
	}

	toUpdates() {
		return Array.from(this.pendingUpdates.entries()).map(([rowId, values]) => ({
			rowId,
			values,
		}));
	}
}
