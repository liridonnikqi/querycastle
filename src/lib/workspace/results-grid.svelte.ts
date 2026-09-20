import type { FkOption } from '$lib/utils/fk-lookup';
import { HIDDEN_ROW_ID_COLUMN } from '$lib/utils/dialect';
import type { PageSize } from '$lib/utils/table-browse';

export type PendingInsertRow = { id: string; values: Record<string, unknown> };

const UNDO_LIMIT = 50;

type GridUndoOp =
	| {
			kind: 'cell';
			rowId: string;
			column: string;
			previous: unknown;
			hadPending: boolean;
	  }
	| { kind: 'insert-add'; id: string }
	| { kind: 'insert-remove'; row: PendingInsertRow; index: number }
	| {
			kind: 'insert-cell';
			id: string;
			column: string;
			previous: unknown;
			hadKey: boolean;
	  }
	| {
			kind: 'delete';
			rowIds: string[];
			snapshots: Array<[string, Record<string, unknown>]>;
			updates: Array<[string, Record<string, unknown>]>;
	  };

export function rowIdOf(row: Record<string, unknown>): string {
	return String(row[HIDDEN_ROW_ID_COLUMN] ?? '');
}

export function isHiddenRowIdColumn(column: string): boolean {
	return column === HIDDEN_ROW_ID_COLUMN;
}

function valuesEqual(left: unknown, right: unknown): boolean {
	if (left === right) return true;
	if (left == null && right == null) return true;
	return false;
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
	undoStack = $state<GridUndoOp[]>([]);
	private applyingUndo = false;

	get canUndo() {
		return this.undoStack.length > 0;
	}

	private pushUndo(op: GridUndoOp) {
		if (this.applyingUndo) return;
		this.undoStack = [...this.undoStack.slice(-(UNDO_LIMIT - 1)), op];
	}

	undo() {
		const stack = this.undoStack;
		if (stack.length === 0) return;
		const op = stack[stack.length - 1]!;
		this.undoStack = stack.slice(0, -1);
		this.applyingUndo = true;
		try {
			switch (op.kind) {
				case 'cell': {
					const map = new Map(this.pendingUpdates);
					const prev = { ...(map.get(op.rowId) ?? {}) };
					if (op.hadPending) prev[op.column] = op.previous;
					else delete prev[op.column];
					if (Object.keys(prev).length === 0) map.delete(op.rowId);
					else map.set(op.rowId, prev);
					this.pendingUpdates = map;
					break;
				}
				case 'insert-add':
					this.pendingInserts = this.pendingInserts.filter((row) => row.id !== op.id);
					break;
				case 'insert-remove': {
					const next = [...this.pendingInserts];
					next.splice(op.index, 0, op.row);
					this.pendingInserts = next;
					break;
				}
				case 'insert-cell':
					this.pendingInserts = this.pendingInserts.map((row) => {
						if (row.id !== op.id) return row;
						const values = { ...row.values };
						if (op.hadKey) values[op.column] = op.previous;
						else delete values[op.column];
						return { ...row, values };
					});
					break;
				case 'delete': {
					const nextDeletes = new Set(this.pendingDeletes);
					const nextSnapshots = new Map(this.deletedSnapshots);
					const nextUpdates = new Map(this.pendingUpdates);
					for (const rowId of op.rowIds) {
						nextDeletes.delete(rowId);
						nextSnapshots.delete(rowId);
					}
					for (const [rowId, values] of op.updates) {
						nextUpdates.set(rowId, values);
					}
					this.pendingDeletes = nextDeletes;
					this.deletedSnapshots = nextSnapshots;
					this.pendingUpdates = nextUpdates;
					break;
				}
			}
		} finally {
			this.applyingUndo = false;
		}
	}

	clearDrafts() {
		this.pendingUpdates = new Map();
		this.pendingDeletes = new Set();
		this.pendingInserts = [];
		this.deletedSnapshots = new Map();
		this.selectedRows = new Set();
		this.activeRowId = null;
		this.undoStack = [];
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

	cellValue(
		row: Record<string, unknown>,
		rowId: string,
		column: string,
	): unknown {
		const pending = this.pendingUpdates.get(rowId);
		if (pending && Object.prototype.hasOwnProperty.call(pending, column)) {
			return pending[column];
		}
		return row[column];
	}

	hasPendingCell(rowId: string, column: string): boolean {
		return this.pendingUpdates.get(rowId)?.[column] !== undefined;
	}

	setCellValue(
		rowId: string,
		column: string,
		nextValue: unknown,
		baseValue: unknown,
	) {
		const pending = this.pendingUpdates.get(rowId);
		const hadPending =
			Boolean(pending) && Object.prototype.hasOwnProperty.call(pending, column);
		this.pushUndo({
			kind: 'cell',
			rowId,
			column,
			previous: hadPending ? pending?.[column] : undefined,
			hadPending,
		});
		const map = new Map(this.pendingUpdates);
		const prev = { ...(map.get(rowId) ?? {}) };
		if (valuesEqual(nextValue, baseValue)) {
			delete prev[column];
		} else {
			prev[column] = nextValue;
		}
		if (Object.keys(prev).length === 0) map.delete(rowId);
		else map.set(rowId, prev);
		this.pendingUpdates = map;
	}

	toggleRowSelected(rowId: string) {
		const next = new Set(this.selectedRows);
		if (next.has(rowId)) next.delete(rowId);
		else next.add(rowId);
		this.selectedRows = next;
		this.activeRowId = rowId;
	}

	toggleSelectAll(ids: string[]) {
		const allSelected = ids.length > 0 && ids.every((id) => this.selectedRows.has(id));
		this.selectedRows = allSelected ? new Set() : new Set(ids);
	}

	clearSelection() {
		this.selectedRows = new Set();
	}

	deleteRows(rowIds: string[], rows: Array<Record<string, unknown>>) {
		const snapshots: Array<[string, Record<string, unknown>]> = [];
		const updates: Array<[string, Record<string, unknown>]> = [];
		const nextDeletes = new Set(this.pendingDeletes);
		const nextUpdates = new Map(this.pendingUpdates);
		const nextSelected = new Set(this.selectedRows);
		const nextSnapshots = new Map(this.deletedSnapshots);
		for (const rowId of rowIds) {
			const row = rows.find((item) => rowIdOf(item) === rowId);
			if (row) {
				nextSnapshots.set(rowId, { ...row });
				snapshots.push([rowId, { ...row }]);
			}
			const existingUpdate = nextUpdates.get(rowId);
			if (existingUpdate) updates.push([rowId, { ...existingUpdate }]);
			nextDeletes.add(rowId);
			nextUpdates.delete(rowId);
			nextSelected.delete(rowId);
		}
		this.pushUndo({ kind: 'delete', rowIds: [...rowIds], snapshots, updates });
		this.pendingDeletes = nextDeletes;
		this.pendingUpdates = nextUpdates;
		this.selectedRows = nextSelected;
		this.deletedSnapshots = nextSnapshots;
	}

	startInsert(values: Record<string, unknown> = {}): string {
		const id = crypto.randomUUID();
		this.pushUndo({ kind: 'insert-add', id });
		this.pendingInserts = [...this.pendingInserts, { id, values }];
		return id;
	}

	setInsertValue(id: string, column: string, value: unknown) {
		const current = this.pendingInserts.find((row) => row.id === id);
		const hadKey = Boolean(
			current && Object.prototype.hasOwnProperty.call(current.values, column),
		);
		this.pushUndo({
			kind: 'insert-cell',
			id,
			column,
			previous: hadKey ? current?.values[column] : undefined,
			hadKey,
		});
		this.pendingInserts = this.pendingInserts.map((row) =>
			row.id === id ? { ...row, values: { ...row.values, [column]: value } } : row,
		);
	}

	removeInsert(id: string) {
		const index = this.pendingInserts.findIndex((row) => row.id === id);
		if (index === -1) return;
		const row = this.pendingInserts[index]!;
		this.pushUndo({ kind: 'insert-remove', row, index });
		this.pendingInserts = this.pendingInserts.filter((item) => item.id !== id);
	}

	setColumnFilter(column: string, value: string) {
		this.columnFilters = { ...this.columnFilters, [column]: value };
	}

	clearFilters() {
		this.columnFilters = {};
		this.showFilterRow = false;
	}

	setPageSize(next: PageSize) {
		this.pageSize = next;
	}

	beginFkLoad(key: string) {
		const next = new Set(this.fkLoadingKeys);
		next.add(key);
		this.fkLoadingKeys = next;
	}

	endFkLoad(key: string) {
		const next = new Set(this.fkLoadingKeys);
		next.delete(key);
		this.fkLoadingKeys = next;
	}

	setFkOptions(key: string, options: FkOption[]) {
		const next = new Map(this.fkOptionCache);
		next.set(key, options);
		this.fkOptionCache = next;
	}

	fkOptions(key: string): FkOption[] {
		return this.fkOptionCache.get(key) ?? [];
	}

	isFkLoading(prefix: string): boolean {
		for (const key of this.fkLoadingKeys) {
			if (key === prefix || key.startsWith(`${prefix}::`)) return true;
		}
		return false;
	}
}
